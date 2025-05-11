use std::borrow::Cow;

use thiserror::Error;

mod api;

pub use api::DockerApi;

pub type Result<T> = std::result::Result<T, DockerError>;

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("failed to get the home directory of the current user")]
    GetHomeError(#[from] homedir::GetHomeError),
    #[error("failed to resolve the '{var_name}' env variable")]
    EnvLookupError {
        var_name: Cow<'static, str>,
        #[source]
        source: std::env::VarError,
    },
    #[error("failed to perform an I/O operation: {description}")]
    IoError {
        description: Cow<'static, str>,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to deserialize a JSON {description}")]
    JsonDeserializationError {
        description: Cow<'static, str>,
        #[source]
        source: serde_json::Error,
    },
    #[error("{0}")]
    Other(Cow<'static, str>),
}

impl DockerError {
    fn from_var_error_with_var_name(source: std::env::VarError, var_name: Cow<'static, str>) -> DockerError {
        DockerError::EnvLookupError { var_name, source }
    }

    fn from_io_error_with_description(
        source: std::io::Error,
        description: impl Fn() -> Cow<'static, str>,
    ) -> DockerError {
        DockerError::IoError {
            description: description(),
            source,
        }
    }

    fn from_serde_error_with_description(
        source: serde_json::Error,
        description: impl Fn() -> Cow<'static, str>,
    ) -> DockerError {
        DockerError::JsonDeserializationError {
            description: description(),
            source,
        }
    }
}
