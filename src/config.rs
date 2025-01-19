use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;
use dirs::home_dir;

#[derive(Parser)]
#[command(version, about)]
struct Arg {
    #[arg(short = 'p', long)]
    config_path: Option<PathBuf>,
    #[arg(short = 'c', long, default_value_t = true)]
    cache_layers: bool,
}

#[derive(Debug)]
pub struct Config {
    config_path: PathBuf,
    cache_layers: bool,
}

impl Config {
    pub fn new() -> anyhow::Result<Self> {
        let Arg {
            config_path,
            cache_layers,
        } = Arg::try_parse().context("failed to parse CLI args")?;

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
            cache_layers,
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

    pub fn cache_layers(&self) -> bool {
        self.cache_layers
    }
}
