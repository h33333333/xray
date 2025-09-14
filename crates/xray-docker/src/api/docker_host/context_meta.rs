use std::fs::File;
use std::io::BufReader;

use serde::Deserialize;

use super::config::DockerConfig;
use crate::{DockerError, Result};

/// Contains a subset of fields from the Context metadata file that are needed to resolve the Docker host.
#[derive(Deserialize)]
pub struct ContextMetadata {
    #[serde(rename = "Endpoints")]
    endpoints: MetadataEndpoints,
}

impl ContextMetadata {
    pub const FILENAME: &str = "meta.json";

    /// Tries to read the provided [DockerConfig]'s current context metadata from the resolved context metadata directory.
    ///
    /// Will return [Option::None] if provided [DockerConfig] doesn't have current context.
    pub fn new_from_docker_config(
        docker_config: &DockerConfig,
    ) -> Result<Option<Self>> {
        let mut metadata_dir =
            match docker_config.get_current_context_metadata_dir() {
                Some(dir) => dir,
                None => return Ok(None),
            };
        metadata_dir.push(Self::FILENAME);

        let current_context = docker_config
            .current_context
            .as_ref()
            .expect("must be present, as we got a metadata dir above");

        let context_metadata_file = File::open(&metadata_dir).map_err(|e| {
            DockerError::from_io_error_with_description(e, || {
                format!("failed to read the context metadata file ({current_context})").into()
            })
        })?;
        let reader = BufReader::new(context_metadata_file);

        serde_json::from_reader::<_, ContextMetadata>(reader)
            .map_err(|e| {
                DockerError::from_serde_error_with_description(e, || {
                    format!("context metadata ({current_context})").into()
                })
            })
            .map(Option::Some)
    }

    pub fn into_docker_host(self) -> String {
        self.endpoints.docker.host
    }
}

/// Contains a subset of fields from the Context metadata endpoints field that are needed to resolve the Docker host.
#[derive(Deserialize)]
pub struct MetadataEndpoints {
    docker: Endpoint,
}

/// A single endpoint in the [MetadataEndpoints] object.
#[derive(Deserialize)]
pub struct Endpoint {
    #[serde(rename = "Host")]
    host: String,
}
