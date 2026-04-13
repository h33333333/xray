use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use dirs::{config_dir, state_dir};

#[derive(clap::Args)]
#[group(required = false, multiple = false)]
struct ClapImageSource {
    /// Force image resolution using Docker.
    #[arg(short = 'd', long = "docker")]
    force_docker: bool,
    /// Force image resolution using a tarred image.
    #[arg(short = 'f', long = "fs")]
    force_fs: bool,
    /// Force image resolution using Podman.
    #[arg(long = "podman")]
    force_podman: bool,
}

impl ClapImageSource {
    fn into_enum(self) -> ImageSource {
        if self.force_docker {
            ImageSource::ForceDocker
        } else if self.force_fs {
            ImageSource::ForceFS
        } else if self.force_podman {
            ImageSource::ForcePodman
        } else {
            ImageSource::Default
        }
    }
}

#[derive(Parser)]
#[command(version, about)]
struct Arg {
    /// Override the config directory location.
    ///
    /// Default: $XDG_CONFIG_HOME/xray or $HOME/.config/xray
    #[arg(short = 'p', long)]
    config_path: Option<PathBuf>,
    // TODO: implement layer caching
    // #[arg(short = 'c', long, default_value_t = true)]
    // cache_layers: bool,
    #[clap(flatten)]
    image_source: ClapImageSource,
    #[arg()]
    image: String,
}

/// Used to configure the provided image's source.
#[derive(Debug, Clone, Copy)]
pub enum ImageSource {
    /// Try to read to read the image from FS, then Docker, then Podman.
    Default,
    /// Try Docker, don't try reading the image from anywhere else.
    ForceDocker,
    /// Try to read the image from FS, don't try reading the image from anywhere else.
    ForceFS,
    /// Try Podman, don't try reading the image from anywhere else.
    ForcePodman,
}

#[derive(Debug)]
pub struct Config {
    config_path: PathBuf,
    state_path: PathBuf,
    image: String,
    image_source: ImageSource,
}

impl Config {
    pub fn new() -> anyhow::Result<Self> {
        let Arg {
            config_path,
            image,
            image_source,
        } = Arg::parse();
        let image_source = image_source.into_enum();

        let config_path = config_path
            .or_else(default_config_path)
            .context("failed to get the config directory")?;
        let state_path = default_state_path()
            .or_else(|| Some(config_path.clone()))
            .context("failed to get the state directory")?;

        std::fs::create_dir_all(&config_path)
            .context("failed to create the config directory")?;
        std::fs::create_dir_all(&state_path)
            .context("failed to create the state directory")?;

        Ok(Config {
            config_path,
            state_path,
            image,
            image_source,
        })
    }

    pub fn make_config_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let mut config_path = self.config_path.clone();
        config_path.push(path.as_ref());
        config_path
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    pub fn image(&self) -> &str {
        &self.image
    }

    pub fn image_source(&self) -> ImageSource {
        self.image_source
    }
}

fn default_config_path() -> Option<PathBuf> {
    config_dir().map(|mut path| {
        path.push("xray");
        path
    })
}

fn default_state_path() -> Option<PathBuf> {
    state_dir().map(|mut path| {
        path.push("xray");
        path
    })
}
