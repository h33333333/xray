use std::path::Path;

use anyhow::Context;
use xray_tui::{init_logging, resolve_image_from_config, AppDispatcher, Config};

fn main() -> anyhow::Result<()> {
    let config = Config::new()?;

    init_logging(Path::new(config.config_path()))?;

    let image = resolve_image_from_config(&config).context("failed to resolve the image")?;
    if image.layers.is_empty() {
        anyhow::bail!("Got an image with zero layers, nothing to inspect here")
    }

    AppDispatcher::init(image)
        .context("failed to initialize the app")?
        .run_until_stopped()
        .context("error during execution")
}
