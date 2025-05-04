use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use dirs::home_dir;

#[derive(Parser)]
#[command(version, about)]
struct Arg {
    #[arg(short = 'p', long)]
    config_path: Option<PathBuf>,
    // TODO: implement layer caching
    // #[arg(short = 'c', long, default_value_t = true)]
    // cache_layers: bool,
    #[arg()]
    image: String,
}

#[derive(Debug)]
pub struct Config {
    config_path: PathBuf,
    image: String,
}

impl Config {
    pub fn new() -> anyhow::Result<Self> {
        let Arg { config_path, image } = Arg::parse();

        let config_path = config_path
            .or_else(|| {
                let mut home = home_dir()?;
                home.push(".xray");
                Some(home)
            })
            .context("failed to get the config directory")?;

        std::fs::create_dir_all(&config_path).context("failed to create the config directory")?;

        Ok(Config { config_path, image })
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
}
