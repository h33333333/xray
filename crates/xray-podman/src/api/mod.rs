mod command_runner;

use anyhow::Context;

use super::{PodmanError, Result};
use crate::api::command_runner::CommandRunner;

const EXIT_CODE_MISSING_IMAGE: i32 = 1;

/// A wrapper for the Podman CLI that allows Xray to retrieve the required information about images.
#[derive(Debug, Clone, Default)]
pub struct PodmanApi;

impl PodmanApi {
    pub fn new() -> Self {
        Default::default()
    }

    /// Checks whether the provided image exists locally.
    pub fn image_is_present(&self, image: &str) -> Result<bool> {
        let runner =
            CommandRunner::spawn("podman", &["image", "exists", image])?;

        let output = runner
            .wait_until_completion()
            .context("failed to run a podman CLI command to completion")?;

        if output.status.success() {
            // No need to do anything else.
            return Ok(true);
        }
        if output
            .status
            .code()
            .is_some_and(|code| code == EXIT_CODE_MISSING_IMAGE)
        {
            // Image doesn't exist.
            return Ok(false);
        }
        // Otherwise it's an error and we need to check it.

        Err(PodmanError::construct_cli_error_with_details(
            "validate image existence",
            output.status.code().unwrap_or(0),
            &output.stderr,
        ))
    }

    /// Pulls the provided image.
    pub fn pull_image(&self, image: &str) -> Result<()> {
        let runner = CommandRunner::spawn("podman", &["pull", "-q", image])?;

        let output = runner
            .wait_until_completion()
            .context("failed to run a podman CLI command to completion")?;

        if output.status.success() {
            // No need to do anything else.
            return Ok(());
        }
        // Otherwise it's an error and we need to check it.

        Err(PodmanError::construct_cli_error_with_details(
            "pull an image",
            output.status.code().unwrap_or(0),
            &output.stderr,
        ))
    }

    /// Downloads a tarball of the provided image.
    pub fn save_image(&self, image: &str) -> Result<Vec<u8>> {
        let runner = CommandRunner::spawn(
            "podman",
            &["save", "-q", "--format", "oci-archive", image],
        )?;

        let output = runner
            .wait_until_completion()
            .context("failed to run a podman CLI command to completion")?;

        if !output.status.success() {
            return Err(PodmanError::construct_cli_error_with_details(
                "save image",
                output.status.code().unwrap_or(0),
                &output.stderr,
            ));
        }

        Ok(output.stdout)
    }
}
