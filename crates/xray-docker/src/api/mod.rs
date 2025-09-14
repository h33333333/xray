use std::env::{self};

mod chunk_processor;
mod connection;
mod docker_host;
mod util;

use connection::DockerApiConnection;
use docker_host::{
    ContextMetadata, DEFAULT_DOCKER_HOST, DOCKER_HOST_ENV_VAR, DockerConfig,
    DockerHost,
};
use http::StatusCode;
use util::encode_request;

use crate::{DockerError, Result};

pub struct DockerApi {
    connection: DockerApiConnection,
    buffer: Vec<u8>,
}

impl DockerApi {
    /// Creates a new [DockerApi] instance using the Docker Host resolution logic from [Self::get_docker_host].
    pub fn new_with_host_resolution() -> Result<Self> {
        let host = Self::get_docker_host()?;
        let connection = DockerApiConnection::connect(host)?;

        Ok(DockerApi {
            connection,
            buffer: Vec::new(),
        })
    }

    /// Checks the provided image exists locally.
    pub fn image_is_present(&mut self, image: &str) -> Result<bool> {
        let request = http::Request::builder()
            .uri(format!("/images/{image}/json"))
            .header("host", "docker")
            .header("accept", "*/*")
            .body(Vec::new())
            .map_err(|_| {
                DockerError::Other(
                    "failed to construct the image inspect request".into(),
                )
            })?;

        // Send the  request and receive a response
        self.buffer.clear();
        encode_request(&request, &mut self.buffer)?;
        let status_code = self.connection.make_request(&mut self.buffer)?;

        Ok(status_code == StatusCode::OK)
    }

    /// Pulls the provided image.
    pub fn pull_image(&mut self, image: &str) -> Result<()> {
        let tag = image
            .split_once(":")
            .map(|(_, tag)| tag)
            .unwrap_or("latest");
        let request = http::Request::builder()
            .uri(format!("/images/create?fromImage={image}&tag={tag}"))
            .method("POST")
            .header("host", "docker")
            .header("accept", "*/*")
            .body(Vec::new())
            .map_err(|_| {
                DockerError::Other(format!("failed to construct the request to pull the '{image}' image").into())
            })?;

        // Send the  request and receive a response
        self.buffer.clear();
        encode_request(&request, &mut self.buffer)?;
        let status_code = self.connection.make_request(&mut self.buffer)?;

        if status_code != http::StatusCode::OK {
            match status_code {
                StatusCode::NOT_FOUND => Err(DockerError::Other(
                    "failed to pull the image: no such image".into(),
                )),
                _ => Err(DockerError::Other("failed to pull the image".into())),
            }
        } else {
            Ok(())
        }
    }

    /// Downloads a tarball of the provided image.
    pub fn export_image(&mut self, image: &str) -> Result<&[u8]> {
        let request = http::Request::builder()
            .uri(format!("/images/{image}/get"))
            .header("host", "docker")
            .header("accept", "*/*")
            .body(Vec::new())
            .map_err(|_| {
                DockerError::Other(format!("failed to construct the request to export the '{image}' image").into())
            })?;

        // Send the  request and receive a response
        self.buffer.clear();
        encode_request(&request, &mut self.buffer)?;
        let status_code = self.connection.make_request(&mut self.buffer)?;

        if status_code != http::StatusCode::OK {
            match status_code {
                StatusCode::NOT_FOUND => Err(DockerError::Other(
                    "failed to export the image: no such image".into(),
                )),
                _ => {
                    Err(DockerError::Other("failed to export the image".into()))
                }
            }
        } else {
            Ok(&self.buffer)
        }
    }

    /// Consumets this Docker API instance and returns the underlying buffer.
    pub fn into_buffer(self) -> Vec<u8> {
        self.buffer
    }

    /// Returns the resolved Docker host for the current system.
    pub fn get_docker_host() -> Result<DockerHost> {
        if let Ok(host) = env::var(DOCKER_HOST_ENV_VAR) {
            // No need to check anything else if we have an explicit env var
            return Ok(host.into());
        }

        // If we don't have an env, we need to check the Docker Context
        let Some(docker_config) = DockerConfig::new()? else {
            // We can't do anything else at this point besides returning the default Docker host
            return Ok(DEFAULT_DOCKER_HOST.into());
        };

        let context_meta =
            ContextMetadata::new_from_docker_config(&docker_config)?;
        if let Some(context_meta) = context_meta {
            Ok(context_meta.into_docker_host().into())
        } else {
            // Return default Docker host for the current OS
            Ok(DEFAULT_DOCKER_HOST.into())
        }
    }
}
