//! Contains all the stuff related to parsing JSON blobs.

use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

use super::Sha256Digest;
use crate::parser::constants::{SHA256_DIGEST_LENGTH, SHA256_DIGEST_PREFIX};
use crate::parser::util::sha256_digest_from_hex;

pub(super) type ImageLayerConfigs = Vec<LayerConfig>;
pub(super) type ImageHistory = Vec<HistoryEntry>;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum JsonBlob {
    Manifest {
        layers: ImageLayerConfigs,
    },
    Config {
        architecture: String,
        os: String,
        history: ImageHistory,
    },
}

#[derive(Debug, Deserialize)]
pub(super) struct LayerConfig {
    #[serde(deserialize_with = "deserialize_sha256_hash")]
    pub digest: Sha256Digest,
}

#[derive(Debug, Deserialize)]
pub(super) struct HistoryEntry {
    pub created_by: String,
    pub comment: Option<String>,
    #[serde(default)]
    pub empty_layer: bool,
}

/// Deserializes a hex string with SHA256 digest that is prepended with the `sha256:` prefix.
fn deserialize_sha256_hash<'de, D>(de: D) -> Result<Sha256Digest, D::Error>
where
    D: Deserializer<'de>,
{
    struct Sha256HashVisitor;

    impl<'de> Visitor<'de> for Sha256HashVisitor {
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
