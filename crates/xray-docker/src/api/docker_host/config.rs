use std::env::{self, VarError};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

use serde::Deserialize;

use super::constants::{
    DOCKER_CONFIG_DIR_ENV_VAR, DOCKER_CONTEXTS_METADATA_DIR,
    DOCKER_DEFAULT_CONFIG_DIR,
};
use super::util::{encode_sha256_digest, get_home_dir, sha256_digest};
use crate::{DockerError, Result};

/// A subset of fields from the Docker Config that are needed to resolve the Docker host.
#[derive(Deserialize)]
pub struct DockerConfig {
    #[serde(rename = "currentContext")]
    pub current_context: Option<String>,
    #[serde(skip, default)]
    pub config_dir: PathBuf,
}

impl DockerConfig {
    pub const FILENAME: &str = "config.json";

    /// Tries to parse the Docker config from the resolved config directory.
    ///
    /// Returns the parsed [DockerConfig].
    pub fn new() -> Result<Option<Self>> {
        let mut config_dir = Self::get_dir()?;
        config_dir.push(DockerConfig::FILENAME);

        if !fs::exists(&config_dir).map_err(|e| {
            DockerError::from_io_error_with_description(e, || {
                "faild to check if a the Docker config exists".into()
            })
        })? {
            // We can't continue if there is no Docker config
            return Ok(None);
        }

        let docker_config_file = File::open(&config_dir).map_err(|e| {
            DockerError::from_io_error_with_description(e, || {
                "failed to read the Docker config file".into()
            })
        })?;
        let reader = BufReader::new(docker_config_file);

        serde_json::from_reader::<_, DockerConfig>(reader)
            .map_err(|e| {
                DockerError::from_serde_error_with_description(e, || {
                    "docker config".into()
                })
            })
            .map(|mut config| {
                // Remove the Docker config filename from the path
                config_dir.pop();
                // Set the correct config directory
                config.config_dir = config_dir;
                Some(config)
            })
    }

    /// Returns the metadata directory for the currently active Docker context.
    pub fn get_current_context_metadata_dir(&self) -> Option<PathBuf> {
        let current_context = self.current_context.as_ref()?;
        let encoded_current_context_sha256_digest =
            encode_sha256_digest(sha256_digest(current_context));

        let mut context_dir = self.config_dir.to_owned();
        context_dir.push(DOCKER_CONTEXTS_METADATA_DIR);
        context_dir.push(encoded_current_context_sha256_digest);

        Some(context_dir)
    }

    /// Returns the Docker config directory. Takes into account the [DOCKER_CONFIG_DIR_ENV_VAR].
    fn get_dir() -> Result<PathBuf> {
        match env::var(DOCKER_CONFIG_DIR_ENV_VAR) {
            Ok(config_dir) => Ok(config_dir.into()),
            Err(e @ VarError::NotUnicode(_)) => {
                Err(DockerError::from_var_error_with_var_name(
                    e,
                    DOCKER_CONFIG_DIR_ENV_VAR.into(),
                ))
            }
            Err(VarError::NotPresent) => {
                let mut home_dir = get_home_dir()?;
                home_dir.push(DOCKER_DEFAULT_CONFIG_DIR);
                Ok(home_dir)
            }
        }
    }
}
