use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Context;
use xray::{init_logging, AppDispatcher, Config, Parser};

fn main() -> anyhow::Result<()> {
    let config = Config::new()?;

    init_logging(Path::new(config.config_path()))?;

    let image = File::open(config.image()).context("failed to open the image")?;
    let reader = BufReader::new(image);

    let parser = Parser::default();
    let image = parser.parse_image(reader).context("failed to parse the image")?;

    if image.layers.is_empty() {
        anyhow::bail!("Got an image with zero layers, nothing to inspect here")
    }

    AppDispatcher::init(image)
        .context("failed to initialize the app")?
        .run_until_stopped()
        .context("error during execution")
}
