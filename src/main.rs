use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Context;
use xray::{init_logging, Config, Parser};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging(Path::new("."))?;

    let config = Config::new()?;

    let image = File::open(config.image()).context("failed to open the image")?;
    let reader = BufReader::new(image);

    let parser = Parser::default();
    parser.parse_image(reader).context("failed to parse the image")?;

    Ok(())
}
