mod docker;
mod filesystem;

use docker::DockerSource;
use filesystem::FilesystemSource;

use crate::config::ImageSource;
use crate::parser::Image;
use crate::Config;

/// A trait that represents entities that act as an OCI [Image] source.
trait ImageSourcer {
    /// Get the image using the provided image name or path.
    ///
    /// Returns an [Image] upon success.
    fn get_image(&self, image: &str) -> anyhow::Result<Image>;

    /// Returns a human-readable name representing this source.
    fn name(&self) -> &'static str;
}

pub fn resolve_image_from_config(config: &Config) -> anyhow::Result<Image> {
    let image_sources: Vec<&dyn ImageSourcer> = match config.image_source() {
        ImageSource::Default => vec![&FilesystemSource, &DockerSource],
        ImageSource::ForceDocker => vec![&DockerSource],
        ImageSource::ForceFS => vec![&FilesystemSource],
    };

    if image_sources.is_empty() {
        // Just a precaution for the future
        anyhow::bail!("No image sources configured")
    }

    let mut errors = Vec::new();
    for source in &image_sources {
        match source.get_image(config.image()) {
            Ok(image) => return Ok(image),
            Err(e) => {
                tracing::debug!(
                    "Failed to resolve the image using the {} resolver: {}",
                    source.name(),
                    e
                );
                let error_with_context = e.chain().map(|e| format!("{e}")).collect::<Vec<_>>().join(": ");
                errors.push((source.name(), error_with_context));
            }
        }
    }

    let joined_errors = errors
        .iter()
        .map(|(source, error)| format!("- {source}: {error}"))
        .collect::<Vec<_>>()
        .join("\n");

    anyhow::bail!(
        "Failed to resolve the image using the configured resolvers:\n{}",
        joined_errors
    )
}
