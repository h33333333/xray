//! Contains all the logic related to parsing and processing of OCI-compliant container images represented as Tar blobs.

mod constants;
mod json;
mod tree;
mod util;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::io::{Read, Seek};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use anyhow::Context;
use constants::{BLOB_PATH_PREFIX, SHA256_DIGEST_LENGTH, TAR_BLOCK_SIZE, TAR_MAGIC_NUMBER, TAR_MAGIC_NUMBER_START_IDX};
use indexmap::IndexMap;
use json::{ImageHistory, ImageLayerConfigs, JsonBlob};
use serde::de::DeserializeOwned;
use tar::Archive;
pub use tree::TreeFilter;
use tree::{Node, Tree};
use util::{determine_blob_type, get_entry_size_in_blocks, sha256_digest_from_hex};

pub type Sha256Digest = [u8; SHA256_DIGEST_LENGTH];
pub type LayerChangeSet = Tree;
pub type DirMap = BTreeMap<PathBuf, Tree>;

type LayerSize = u64;

/// Represents state of a [Node] in a layer.
#[derive(Debug, Clone, Copy)]
pub enum NodeStatus {
    /// A node added in the current layer
    Added(u64),
    /// A node that was updated in the current layer
    Modified(u64),
    /// A node that was deleted in the current layer
    Deleted,
}

/// Represents state of a file in a layer.
#[derive(Debug, Clone)]
pub struct FileState {
    status: NodeStatus,
    /// Is `Some` if file is a hardlink/symlink that links to the contained [PathBuf].
    actual_file: Option<PathBuf>,
}

impl FileState {
    pub fn new(status: NodeStatus, actual_file: Option<PathBuf>) -> Self {
        FileState { status, actual_file }
    }
}

/// Represents state of a directory in a layer.
#[derive(Debug, Clone)]
pub struct DirectoryState {
    status: NodeStatus,
    children: DirMap,
}

impl DirectoryState {
    pub fn new_empty() -> Self {
        DirectoryState {
            status: NodeStatus::Added(0),
            children: DirMap::default(),
        }
    }

    pub fn new_with_size(size: u64) -> Self {
        DirectoryState {
            status: NodeStatus::Added(size),
            children: DirMap::default(),
        }
    }
}

/// A parsed OCI-compliant container image.
#[derive(Default)]
pub struct Image {
    /// The repository of the image.
    pub repository: String,
    /// The tag of the image.
    pub tag: String,
    /// The total size of the image in bytes.
    pub size: u64,
    /// The architecture of the image.
    pub architecture: String,
    /// The OS of the image.
    pub os: String,
    /// The total number of layers.
    pub total_layers: usize,
    /// The total number of non-empty layers.
    pub non_empty_layers: usize,
    /// All [Layers](Layer) of this image.
    pub layers: IndexMap<Sha256Digest, Layer>,
}

/// A single layer within the [Image].
pub struct Layer {
    /// A [LayerChangeSet] for this layer.
    ///
    /// Can be missing if the layer is empty.
    pub changeset: Option<LayerChangeSet>,
    /// Size of this layer.
    pub size: u64,
    /// Command that created this layer.
    pub created_by: String,
    /// Comment to the command from [Layer::created_by].
    pub comment: Option<String>,
}

/// A parser for OCI-compliant container images represented as Tar blobs.
#[derive(Default)]
pub struct Parser {
    parsed_layers: HashMap<Sha256Digest, (LayerChangeSet, LayerSize)>,
    layer_configs: Option<ImageLayerConfigs>,
    history: Option<ImageHistory>,
    architecture: Option<String>,
    os: Option<String>,
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

                    let (layer_changeset, layer_size) = self
                        .parse_tar_blob(
                            &mut reader,
                            entry_size_in_blocks * TAR_BLOCK_SIZE as u64,
                            layer_sha256_digest,
                        )
                        .context("error while parsing a tar layer")?;

                    self.parsed_layers
                        .insert(layer_sha256_digest, (layer_changeset, layer_size));

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
                                self.layer_configs = Some(parsed_layers);
                            }
                            JsonBlob::Config {
                                architecture: parsed_architecture,
                                os: parsed_os,
                                history: parsed_history,
                            } => {
                                self.architecture = Some(parsed_architecture);
                                self.os = Some(parsed_os);
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

    /// Parses a single image layer represented as a Tar blob.
    fn parse_tar_blob<R: Read + Seek>(
        &self,
        src: &mut R,
        blob_size: u64,
        layer_digest: Sha256Digest,
    ) -> anyhow::Result<(LayerChangeSet, LayerSize)> {
        let mut archive = Archive::new(src);
        // We don't want to stop when we encounter an empty Tar header, as we want to parse other blobs as well
        archive.set_ignore_zeros(true);

        let mut change_set = LayerChangeSet::new(layer_digest);

        let mut layer_size = 0;
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

                return Ok((change_set, layer_size));
            }

            if let Ok(path) = header.path() {
                if path == Path::new("./") {
                    // Some images include the top-level element, which we don't need
                    continue;
                }

                let (path, node) = if header.entry_type().is_dir() {
                    (path, Node::new_empty_dir())
                } else {
                    let size = header.size().unwrap_or(0);
                    layer_size += size;

                    let (path, status, link) =
                        if let Some(link) = header.link_name().context("failed to retrieve the link name")? {
                            (path, NodeStatus::Added(0), Some(link.into_owned()))
                        } else if let Some(file_name) = path.file_name() {
                            // Check if it's a whiteout
                            if file_name.as_encoded_bytes().starts_with(b".wh.") {
                                (
                                    // Strip the whiteout prefix
                                    Cow::Owned(path.with_file_name(OsStr::from_bytes(
                                        // SAFETY: unwrap is safe, as we know that the prefix exists
                                        file_name.as_encoded_bytes().strip_prefix(b".wh.").unwrap(),
                                    ))),
                                    NodeStatus::Deleted,
                                    None,
                                )
                            } else if file_name.as_encoded_bytes() != b".wh..wh..opq" {
                                (path, NodeStatus::Added(size), None)
                            } else {
                                // Simply ignoring opaque whiteouts does the trick
                                // FIXME: no, it doesn't. I need to mark a directory as one that contains an opaque whiteout file
                                // and then handle such directories correspondingly when merging the trees
                                continue;
                            }
                        } else {
                            // We can't do anything with such files
                            continue;
                        };

                    (path, Node::File(FileState::new(status, link)))
                };

                change_set
                    .insert(path, node, layer_digest)
                    .context("failed to insert an entry")?;
            }
        }

        Ok((change_set, layer_size))
    }

    /// Processes all the parsed data and turns it into an [Image].
    fn finalize(self) -> anyhow::Result<Image> {
        // Use IndexMap so that layers are always in the correct order
        let mut layers = IndexMap::new();

        let layer_configs = self
            .layer_configs
            .context("malformed container image: manifest is missing")?;
        let layers_history = self.history.context("malformed container image: config is missing")?;

        let total_layers = layers_history.len();
        let non_empty_layers = layer_configs.len();

        let mut per_layer_changeset = self.parsed_layers;
        let mut image_size = 0;
        for (layer_config, layer_history) in layer_configs
            .into_iter()
            .zip(layers_history.into_iter().filter(|entry| !entry.empty_layer))
        {
            let (layer_changeset, layer_size) = per_layer_changeset
                .remove(&layer_config.digest)
                // Turn into `Option<LayerChangeset>` to avoid a pointless empty `Vec` allocation.
                .map(|(changeset, size)| (Some(changeset), size))
                // Changeset can be missing if layer didn't cause any FS changes
                .unwrap_or_default();

            image_size += layer_size;
            layers.insert(
                layer_config.digest,
                Layer {
                    changeset: layer_changeset,
                    size: layer_size,
                    created_by: layer_history.created_by,
                    comment: layer_history.comment,
                },
            );
        }

        Ok(Image {
            // FIXME: extract from manifest
            repository: "hello-docker".to_owned(),
            // FIXME: extract from manifest
            tag: "latest".to_owned(),
            size: image_size,
            architecture: self
                .architecture
                .context("malformed container image: missing architecture")?,
            os: self.os.context("malformed container image: missing os")?,
            total_layers,
            non_empty_layers,
            layers,
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
