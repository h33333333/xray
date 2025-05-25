use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use dirs::home_dir;

#[derive(clap::Args)]
#[group(required = false, multiple = false)]
struct ClapImageSource {
    /// Force image resolution using Docker
    #[arg(short = 'd', long = "docker")]
    force_docker: bool,
    /// Force image resolution using a tarred image
    #[arg(short = 'f', long = "fs")]
    force_fs: bool,
}

impl ClapImageSource {
    fn into_enum(self) -> ImageSource {
        if self.force_docker {
            ImageSource::ForceDocker
        } else if self.force_fs {
            ImageSource::ForceFS
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
    /// Default: $HOME/.xray
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
    /// Try to read the image from FS, try Docker if FS resolution failed
    Default,
    /// Try Docker, don't try reading the image from FS
    ForceDocker,
    /// Try to read the image from FS, don't try Docker
    ForceFS,
}

#[derive(Debug)]
pub struct Config {
    config_path: PathBuf,
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
            .or_else(|| {
                let mut home = home_dir()?;
                home.push(".xray");
                Some(home)
            })
            .context("failed to get the config directory")?;

        std::fs::create_dir_all(&config_path).context("failed to create the config directory")?;

        Ok(Config {
            config_path,
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

    pub fn image(&self) -> &str {
        &self.image
    }

    pub fn image_source(&self) -> ImageSource {
        self.image_source
    }
}
