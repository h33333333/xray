use std::fs::File;
use std::io::BufReader;

use anyhow::Context as _;

use super::ImageSourcer;
use crate::Parser;

pub struct FilesystemSource;

impl ImageSourcer for FilesystemSource {
    fn get_image(&self, image: &str) -> anyhow::Result<crate::parser::Image> {
        if !std::fs::exists(image).with_context(|| format!("failed to check if the '{image}' path exists"))? {
            tracing::info!("Failed to find the tarred image locally");
            anyhow::bail!("the specified path doesn't exist")
        }
        let raw_image = File::open(image).context("failed to open the tarred image")?;

        tracing::info!("Found the tarred image locally, parsing...");

        let reader = BufReader::new(raw_image);
        let parser = Parser::default();
        parser.parse_image(reader).context("failed to parse the tarred image")
    }

    fn name(&self) -> &'static str {
        "Filesystem"
    }
}
