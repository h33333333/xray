//! Contains all the logic required to determine the Docker Host.

mod config;
mod constants;
mod context_meta;
mod util;

use std::borrow::Cow;
use std::env::{self};

use config::DockerConfig;
use constants::{DEFAULT_DOCKER_HOST, DOCKER_HOST_ENV_VAR};
use context_meta::ContextMetadata;

use crate::Result;

pub type DockerHost = Cow<'static, str>;

/// Returns the resolved Docker host for the current system.
pub fn get_docker_host() -> Result<DockerHost> {
    if let Ok(host) = env::var(DOCKER_HOST_ENV_VAR) {
        // No need to check anything else if we have an explicit env var
        return Ok(host.into());
    }

    // If we don't have an env, we need to check the Docker Context
    let docker_config = DockerConfig::new()?;
    let context_meta = ContextMetadata::new_from_docker_config(&docker_config)?;
    if let Some(context_meta) = context_meta {
        Ok(context_meta.into_docker_host().into())
    } else {
        // Return default Docker host for the current OS
        Ok(DEFAULT_DOCKER_HOST.into())
    }
}
