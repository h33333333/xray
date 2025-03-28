use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Context;
use xray::{init_app_dispatcher, init_logging, run, Config, Parser};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging(Path::new("."))?;

    let config = Config::new()?;

    let image = File::open(config.image()).context("failed to open the image")?;
    let reader = BufReader::new(image);

    let parser = Parser::default();
    let image = parser.parse_image(reader).context("failed to parse the image")?;

    if image.layers.is_empty() {
        anyhow::bail!("Got an image with zero layers, nothing to inspect here")
    }

    // for (_, layer) in image.layers.iter() {
    //     if let Some(changeset) = layer.changeset.as_ref() {
    //         println!("{:?}", changeset);
    //     }
    // }

    run(init_app_dispatcher(image).context("failed to initialize the app")?)
}
