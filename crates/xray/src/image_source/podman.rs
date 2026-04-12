use std::io::Cursor;

use anyhow::Context as _;
use xray_podman::PodmanApi;

use crate::Parser;
use crate::image_source::ImageSourcer;

pub struct PodmanSource;

impl ImageSourcer for PodmanSource {
    fn get_image(&self, image: &str) -> anyhow::Result<crate::parser::Image> {
        let podman_api = PodmanApi::new();

        if !podman_api
            .image_is_present(image)
            .context("failed to check if image is present")?
        {
            tracing::info!(
                "Missing the '{}' image locally; trying to pull from the registry",
                image
            );
            podman_api.pull_image(image)?;
        };

        tracing::info!(
            "Exporting the '{}' image from Podman. This might take some time.",
            image
        );

        let raw_image = podman_api.save_image(image)?;

        tracing::info!(
            "Successfully exported the image from Podman, parsing it..."
        );

        let reader = Cursor::new(raw_image);
        // Podman images don't usually contain Docker-like manifests, so we simply deduce the image name from
        // the input arg.
        let parser = Parser::new_with_image(image);
        parser
            .parse_image(reader)
            .context("failed to parse the image")
    }

    fn name(&self) -> &'static str {
        "Podman"
    }
}
