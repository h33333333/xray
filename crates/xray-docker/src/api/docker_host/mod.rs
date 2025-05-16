//! Contains all the logic required to determine the Docker Host.

mod config;
mod constants;
mod context_meta;
mod util;

use std::borrow::Cow;

pub use config::DockerConfig;
pub use constants::*;
pub use context_meta::ContextMetadata;

pub type DockerHost = Cow<'static, str>;
