use std::io::Cursor;

use anyhow::Context as _;
use xray_docker::DockerApi;

use super::ImageSourcer;
use crate::Parser;

pub struct DockerSource;

impl ImageSourcer for DockerSource {
    fn get_image(&self, image: &str) -> anyhow::Result<crate::parser::Image> {
        let mut docker_api = DockerApi::new_with_host_resolution()?;

        if !docker_api
            .image_is_present(image)
            .context("failed to check if image is present")?
        {
            tracing::info!(
                "Missing the '{}' image locally; trying to pull from the registry",
                image
            );
            docker_api.pull_image(image)?;
        };

        tracing::info!(
            "Exporting the '{}' image from Docker. This might take some time.",
            image
        );

        let raw_image = docker_api.export_image(image)?;

        tracing::info!(
            "Successfully exported the image from Docker, parsing it..."
        );

        let reader = Cursor::new(raw_image);
        let parser = Parser::default();
        parser
            .parse_image(reader)
            .context("failed to parse the image")
    }

    fn name(&self) -> &'static str {
        "Docker"
    }
}
