//! Contains all the logic related to parsing and processing of OCI-compliant container images represented as Tar blobs.

mod constants;
mod json;
mod util;

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek};
use std::path::PathBuf;

use anyhow::Context;
use constants::{BLOB_PATH_PREFIX, SHA256_DIGEST_LENGTH, TAR_BLOCK_SIZE, TAR_MAGIC_NUMBER, TAR_MAGIC_NUMBER_START_IDX};
use json::{ImageHistory, ImageLayerConfigs, JsonBlob};
use serde::de::DeserializeOwned;
use tar::Archive;
use util::{determine_blob_type, get_entry_size_in_blocks, sha256_digest_from_hex};

pub type Sha256Digest = [u8; SHA256_DIGEST_LENGTH];
pub type LayerChangeSet = Vec<ChangedFile>;

/// A parsed OCI-compliant container image.
#[derive(Debug, Default)]
pub struct Image {
    pub per_layer_changeset: HashMap<Sha256Digest, LayerChangeSet>,
    pub per_layer_config: BTreeMap<Sha256Digest, LayerConfig>,
}

/// Represents a single changed file within an image layer.
#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug)]
pub struct LayerConfig {
    pub size: u64,
    pub created_by: String,
    pub comment: Option<String>,
}

/// A parser for OCI-compliant container images represented as Tar blobs.
#[derive(Debug, Default)]
pub struct Parser {
    per_layer_changeset: HashMap<Sha256Digest, LayerChangeSet>,
    layers: Option<ImageLayerConfigs>,
    history: Option<ImageHistory>,
}

impl Parser {
    pub fn new() -> Self {
        Parser::default()
    }

    /// Parses an OCI-compliant container image from the provided image Tar blob.
    pub fn parse_image<R: Read + Seek>(mut self, src: R) -> anyhow::Result<Image> {
        let mut archive = Archive::new(src);
        let mut entries = archive
            .entries_with_seek()
            .context("failed to get entries from the archive")?;

        // A reusable buffer used for determining the blob type
        let mut tar_header = [0u8; TAR_BLOCK_SIZE];
        while let Some(entry) = entries.next() {
            let mut entry = entry.context("error while reading an entry")?;

            let header = entry.header();

            let entry_size_in_blocks =
                get_entry_size_in_blocks(header).context("failed to determine the entry's size in TAR blocks")?;

            if !header.path_bytes().starts_with(BLOB_PATH_PREFIX) || entry_size_in_blocks == 0 {
                // Skip the current entry if it's not a blob or if it's size is 0
                continue;
            }

            let layer_sha256_digest = sha256_digest_from_hex(
                header
                    .path_bytes()
                    .strip_prefix(BLOB_PATH_PREFIX)
                    // SAFETY: checked above
                    .expect("should start with a blob path prefix"),
            )
            .context("failed to parse the layer's sha256 digest from the path")?;

            let (blob_type, offset) = determine_blob_type(&mut tar_header, &mut entry)
                .context("failed to determine the blob type of an entry")?;

            match blob_type {
                BlobType::Empty => {}
                BlobType::Tar => {
                    // HACK: turn archive back into a reader to preserve the `Seek` trait and optimize parsing of the image layer
                    let mut reader = archive.into_inner();

                    if offset != 0 {
                        // Restore the original entry so that it gets parsed correctly.
                        // NOTE: Using `Chain` here is not possible, as `Chain` doesn't implement `Seek`
                        reader
                            .seek_relative(-(offset as i64))
                            .context("failed to wind back the reader")?;
                    }

                    let layer_changeset = self
                        .parse_tar_blob(&mut reader, entry_size_in_blocks * TAR_BLOCK_SIZE as u64)
                        .context("error while parsing a tar layer")?;

                    self.per_layer_changeset.insert(layer_sha256_digest, layer_changeset);

                    // Restore the archive and the iterator
                    archive = Archive::new(reader);
                    entries = archive.entries_with_seek()?;
                }
                BlobType::GzippedTar => todo!("Add support for gzipped tar layers"),
                BlobType::Json => {
                    let json_blob = self.parse_json_blob::<JsonBlob>(&mut tar_header[..offset].chain(entry))?;
                    if let Some(known_json_blob) = json_blob {
                        match known_json_blob {
                            JsonBlob::Manifest { layers: parsed_layers } => {
                                self.layers = Some(parsed_layers);
                            }
                            JsonBlob::Config {
                                history: parsed_history,
                            } => {
                                self.history = Some(parsed_history);
                            }
                        }
                    };
                }
                BlobType::Unknown => {
                    tracing::info!("Unknown blob type was encountered while parsing the image")
                }
            }
        }

        self.finalize()
    }

    /// Parses a single JSON blob within the image.
    fn parse_json_blob<T: DeserializeOwned>(&self, entry: &mut impl Read) -> anyhow::Result<Option<T>> {
        let parsed = match serde_json::from_reader::<_, T>(entry) {
            Ok(parsed) => Some(parsed),
            Err(e) => {
                if e.is_data() {
                    None
                } else {
                    anyhow::bail!("faield to parse a JSON blob: {}", e)
                }
            }
        };

        Ok(parsed)
    }

    /// Parses a single image layer represented as a TAR blob.
    fn parse_tar_blob<R: Read + Seek>(&self, src: &mut R, blob_size: u64) -> anyhow::Result<LayerChangeSet> {
        let mut archive = Archive::new(src);
        // We don't want to stop when we encounter an empty Tar header, as we want to parse other blobs as well
        archive.set_ignore_zeros(true);

        let mut change_set = LayerChangeSet::new();
        for entry in archive
            .entries_with_seek()
            .context("failed to get entries from the tar blob")?
        {
            let entry = entry.context("error while reading an entry from the tar blob")?;
            let header = entry.header();

            if entry.raw_header_position() >= blob_size {
                // We parsed the current blob: reset the header and return
                archive
                    .into_inner()
                    .seek_relative(-(TAR_BLOCK_SIZE as i64))
                    .context("failed to wind back the header")?;

                return Ok(change_set);
            }

            if let Ok(path) = header.path() {
                change_set.push(ChangedFile {
                    path: path.into_owned(),
                    size: header.size().unwrap_or(0),
                })
            }
        }

        Ok(change_set)
    }

    /// Processes all the parsed data and turns it into an [Image].
    fn finalize(self) -> anyhow::Result<Image> {
        let mut per_layer_config = BTreeMap::new();

        let mut layers = self.layers.context("malformed docker image: manifest is missing")?;
        for layer_history in self
            .history
            .context("malformed docker image: config is missing")?
            .into_iter()
            .rev()
            .filter(|entry| !entry.empty_layer)
        {
            let layer_config = layers
                .pop()
                .context("malformed docker image: more history entries than actual layers")?;

            per_layer_config.insert(
                layer_config.digest,
                LayerConfig {
                    size: layer_config.size,
                    created_by: layer_history.created_by,
                    comment: layer_history.comment,
                },
            );
        }

        Ok(Image {
            per_layer_changeset: self.per_layer_changeset,
            per_layer_config,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum BlobType {
    Empty,
    Tar,
    GzippedTar,
    Json,
    Unknown,
}
