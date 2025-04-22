//! Contains all the logic related to parsing and processing of OCI-compliant container images represented as Tar blobs.

mod constants;
mod json;
mod node;
mod util;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::io::{Read, Seek};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use anyhow::Context;
use constants::{
    BLOB_PATH_PREFIX, IMAGE_MANIFEST_PATH, SHA256_DIGEST_LENGTH, TAR_BLOCK_SIZE, TAR_MAGIC_NUMBER,
    TAR_MAGIC_NUMBER_START_IDX,
};
use indexmap::IndexMap;
use json::{ImageHistory, ImageLayerConfigs, JsonBlob, Manifest};
pub use node::NodeFilters;
use node::{InnerNode, Node, RestorablePath};
use serde::de::DeserializeOwned;
use tar::{Archive, Header};
use util::{determine_blob_type, get_entry_size_in_blocks, sha256_digest_from_hex};

pub type Sha256Digest = [u8; SHA256_DIGEST_LENGTH];
pub type LayerChangeSet = Node;
pub type DirMap = BTreeMap<PathBuf, Node>;

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
    pub image_name: Cow<'static, str>,
    /// The tag of the image.
    pub tag: Cow<'static, str>,
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
///
/// OCI specification source: [OCI Image Format Specification](https://github.com/opencontainers/image-spec)
#[derive(Default)]
pub struct Parser {
    parsed_layers: HashMap<Sha256Digest, (LayerChangeSet, LayerSize)>,
    layer_configs: Option<ImageLayerConfigs>,
    history: Option<ImageHistory>,
    architecture: Option<String>,
    os: Option<String>,
    tagged_name: Option<String>,
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

            // Parse the image's manifest and extract name and tag
            if entry.header().path_bytes().as_ref() == IMAGE_MANIFEST_PATH {
                self.tagged_name = Some(Manifest::from_reader(&mut entry)?);
                // We are done with this entry
                continue;
            }

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
                        .parse_tar_blob(&mut reader, entry_size_in_blocks * TAR_BLOCK_SIZE as u64)
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
                        self.process_json_blob(known_json_blob);
                    };
                }
                BlobType::Unknown => {
                    tracing::debug!("Unknown blob type was encountered while parsing the image")
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

    /// Processes a single known JSON blob extracted from an image.
    fn process_json_blob(&mut self, json_blob: JsonBlob) {
        match json_blob {
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
    }

    /// Parses a single image layer represented as a Tar blob.
    fn parse_tar_blob<R: Read + Seek>(
        &self,
        src: &mut R,
        blob_size: u64,
    ) -> anyhow::Result<(LayerChangeSet, LayerSize)> {
        let mut archive = Archive::new(src);
        // We don't want to stop when we encounter an empty Tar header, as we want to parse other blobs as well
        archive.set_ignore_zeros(true);

        // We will set the actual layer idx later in [Self::finalize]
        let mut change_set = LayerChangeSet::new(0);

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

            let Some((node_path, node, node_size)) = self
                .process_layer_entry(header)
                .context("failed to process an entry in the layer")?
            else {
                // A `None` means that we can safely skip this entry
                continue;
            };

            // Adjust the size
            layer_size += node_size;

            change_set
                .insert(
                    // Use a restorable path here to simplify the further processing
                    &mut RestorablePath::new(&node_path),
                    node,
                    // We will set the actual layer idx later in [Self::finalize], as we don't know it yet.
                    0,
                )
                .context("failed to insert an entry")?;
        }

        Ok((change_set, layer_size))
    }

    /// Processes a TAR header of a single entry (a Node) in a layer.
    ///
    /// Returns the entry's full path, as well as its status and size.
    fn process_layer_entry<'a>(&self, header: &'a Header) -> anyhow::Result<Option<(Cow<'a, Path>, InnerNode, u64)>> {
        let Ok(path) = header.path() else {
            tracing::debug!(?header, "Got a malformed header when parsing an image");
            // Don't error, continue to process the rest of the nodes as usual
            return Ok(None);
        };

        if path == Path::new("./") {
            // Some images include the top-level element, which we don't need
            return Ok(None);
        }

        if header.entry_type().is_dir() {
            return Ok(Some((path, InnerNode::new_empty_dir(), 0)));
        }

        let size = header.size().unwrap_or(0);

        // Check if it's a link
        if let Some(link) = header.link_name().context("failed to retrieve the link name")? {
            return Ok(Some((
                path,
                InnerNode::File(FileState::new(NodeStatus::Added(0), Some(link.into_owned()))),
                size,
            )));
        }

        let Some(file_name) = path.file_name() else {
            // We can't do anything with such files
            return Ok(None);
        };

        let (path, status) = if file_name.as_encoded_bytes().starts_with(b".wh.") {
            // A whiteout

            // Strip the whiteout prefix
            let path = Cow::Owned(
                path.with_file_name(OsStr::from_bytes(
                    file_name
                        .as_encoded_bytes()
                        .strip_prefix(b".wh.")
                        .expect("prefix must exist at this point"),
                )),
            );

            (path, NodeStatus::Deleted)
        } else if file_name.as_encoded_bytes() != b".wh..wh..opq" {
            // A regular file
            (path, NodeStatus::Added(size))
        } else {
            // An opaque whiteout

            // FIXME: I need to mark a directory as one that contains an opaque whiteout file
            // and then handle such directories correspondingly when merging the trees
            return Ok(None);
        };

        Ok(Some((path, InnerNode::File(FileState::new(status, None)), size)))
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
            let (mut layer_changeset, layer_size) = per_layer_changeset
                .remove(&layer_config.digest)
                .map(|(changeset, size)| (Some(changeset), size))
                // Changeset can be missing if layer didn't cause any FS changes
                .unwrap_or_default();

            if let Some(changeset) = layer_changeset.as_mut() {
                // Set the correct parent layer idx for all items in the changeset
                //
                // NOTE: an image can only have 127 layers, so the cast is perfectly fine
                changeset.set_layer_recursively(layers.len() as u8)
            }

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

        let (image_name, tag) = self
            .tagged_name
            .and_then(|mut name| {
                let tag = name.split_off(name.find(':')? + 1);
                // Remove ':'
                name.truncate(name.len() - 1);
                Some((Cow::Owned(name), Cow::Owned(tag)))
            })
            .unwrap_or((Cow::Borrowed("<missing>"), Cow::Borrowed("<missing>")));

        Ok(Image {
            image_name,
            tag,
            size: image_size,
            architecture: self
                .architecture
                .context("malformed container image: missing architecture")?,
            os: self.os.context("malformed container image: missing OS")?,
            total_layers,
            non_empty_layers,
            layers,
        })
    }
}

/// Represents the type of a single TAR entry in an image.
#[derive(Debug, Clone, Copy)]
enum BlobType {
    Empty,
    Tar,
    GzippedTar,
    Json,
    Unknown,
}
