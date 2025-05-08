//! Contains all the stuff related to parsing JSON blobs.

use std::io::Read;

use anyhow::Context;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

use super::Sha256Digest;
use crate::parser::constants::{SHA256_DIGEST_LENGTH, SHA256_DIGEST_PREFIX};
use crate::parser::util::sha256_digest_from_hex;

pub(super) type ImageLayerConfigs = Vec<LayerConfig>;
pub(super) type ImageHistory = Vec<HistoryEntry>;

/// Docker-specific manifest that can be found in the root of an image.
///
/// In in our case, it's used to extract the image name and tag, but it's not guaranteed to be present in an image,
/// especially when using non-Docker container runtimes.
#[derive(Deserialize)]
pub(super) struct DockerManifest {
    #[serde(rename = "RepoTags")]
    repo_tags: Option<Vec<String>>,
}

impl DockerManifest {
    pub fn from_reader(src: impl Read) -> anyhow::Result<Option<String>> {
        let mut manifest =
            // Docker manifest contains an array of manifest objects, so we need a vec here
            serde_json::from_reader::<_, Vec<DockerManifest>>(src).context("failed to parse the Docker's manifest")?;

        Ok(manifest
            .get_mut(0)
            .and_then(|manifest| manifest.repo_tags.as_mut().and_then(|tags| tags.pop())))
    }
}

/// Represents known JSON files that can be encountered when parsing an OCI-compliant image.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum JsonBlob {
    /// Image Index.
    ///
    /// Source: [OCI Image Index Specification](https://github.com/opencontainers/image-spec/blob/main/image-index.md)
    Index { manifests: Vec<ImageIndexManifest> },
    /// Image Manifest.
    ///
    /// Source: [OCI Image Manifest Specification](https://github.com/opencontainers/image-spec/blob/main/manifest.md).
    Manifest { layers: ImageLayerConfigs },
    /// Image Config.
    ///
    /// Sourcce: [OCI Image Configuration](https://github.com/opencontainers/image-spec/blob/main/config.md)
    Config {
        architecture: String,
        os: String,
        history: ImageHistory,
    },
}

/// Represents a subset of fields of a single entry in the `history` array that can be found in an OCI Image Config.
///
/// Source: [OCI Image Configuration](https://github.com/opencontainers/image-spec/blob/main/config.md#properties)
#[derive(Debug, Deserialize)]
pub(super) struct HistoryEntry {
    pub created_by: String,
    pub comment: Option<String>,
    #[serde(default)]
    pub empty_layer: bool,
}

/// Represents a subset of fields of a single entry in the `layers` array that can be found in an OCI Image Manifest.
///
/// Source: [OCI Image Manifest Specification](https://github.com/opencontainers/image-spec/blob/main/manifest.md#image-manifest-property-descriptions)
#[derive(Debug, Deserialize)]
pub(super) struct LayerConfig {
    #[serde(deserialize_with = "deserialize_sha256_digest")]
    pub digest: Sha256Digest,
}

/// Deserializes a hex string with SHA256 digest that is prepended with the `sha256:` prefix.
fn deserialize_sha256_digest<'de, D>(de: D) -> Result<Sha256Digest, D::Error>
where
    D: Deserializer<'de>,
{
    struct Sha256HashVisitor;

    impl Visitor<'_> for Sha256HashVisitor {
        type Value = Sha256Digest;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sha256 digest string prefixed with `sha256:`")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // Strip the `sha256:` prefix
            let raw = &v[SHA256_DIGEST_PREFIX.len()..];

            // multiply by 2 because we are dealing with a hex str
            if raw.len() != SHA256_DIGEST_LENGTH * 2 {
                return Err(serde::de::Error::custom("Invalid sha256 digest format"));
            }

            sha256_digest_from_hex(raw).map_err(|_| serde::de::Error::custom("Failed to parse the sha256 digest"))
        }
    }

    de.deserialize_str(Sha256HashVisitor)
}

/// Represents a subset of fields of a single manifest that can be found in the Image Index.
#[derive(Debug, Deserialize)]
pub(super) struct ImageIndexManifest {
    /// Source: [Annotations](https://github.com/opencontainers/image-spec/blob/main/annotations.md).
    pub annotations: Option<ImageIndexManifestAnnotations>,
}

/// A set of well-known annotation keys.
///
/// Source: [Pre-Defined Annotation Keys](https://github.com/opencontainers/image-spec/blob/main/annotations.md#pre-defined-annotation-keys)
#[derive(Debug, Deserialize)]
pub(super) struct ImageIndexManifestAnnotations {
    /// Fully qualified image name.
    #[serde(rename = "io.containerd.image.name")]
    pub fully_qualified_image_name: Option<String>,
}
