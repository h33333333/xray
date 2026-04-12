mod api;
pub use api::PodmanApi;
mod util;

use thiserror::Error;

use crate::util::OptionalString;

pub type Result<T> = std::result::Result<T, PodmanError>;

#[derive(Error, Debug)]
pub enum PodmanError {
    #[error(
        "Failed to {operation}, details from Podman (exit_code={exit_code}): '{cli_err}'"
    )]
    PodmanCli {
        exit_code: i32,
        operation: String,
        cli_err: OptionalString,
    },
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl PodmanError {
    pub(crate) fn construct_cli_error_with_details(
        operation: impl Into<String>,
        exit_code: i32,
        raw_error_details: &[u8],
    ) -> Self {
        let cli_err = OptionalString::new(
            String::from_utf8_lossy(raw_error_details)
                // Some errors are multi-line and look ugly when printed in the terminal.
                .replace("\n", ". ")
                .to_string(),
        );
        PodmanError::PodmanCli {
            exit_code,
            operation: operation.into(),
            cli_err,
        }
    }
}
