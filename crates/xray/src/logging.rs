use std::fs::File;
use std::path::Path;

use anyhow::Context;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub const LOGGING_ENV: &str = "XRAY_LOG";
pub const LOGGING_FILE_ENV: &str = "XRAY_LOG_FILE";

pub fn init_logging(config_folder: &Path) -> anyhow::Result<()> {
    let mut log_path = config_folder.to_path_buf();
    log_path.push("xray.log");

    let log_file = File::options()
        .create(true)
        .append(true)
        .open(log_path)
        .context("failed to create a log file")?;

    let env_filter = EnvFilter::builder()
        .with_env_var(LOGGING_ENV)
        .try_from_env()
        .unwrap_or_else(|_| "xray=info".into());

    tracing_subscriber::registry()
        .with(
            layer()
                .with_target(false)
                .with_filter(LevelFilter::INFO)
                .and_then(
                    layer()
                        .with_file(true)
                        .with_line_number(true)
                        .with_filter(filter_fn(|metadata| *metadata.level() > LevelFilter::INFO)),
                )
                .with_filter(env_filter),
        )
        .with(
            layer().pretty().with_writer(log_file).with_ansi(false).with_filter(
                EnvFilter::builder()
                    .with_env_var(LOGGING_FILE_ENV)
                    .try_from_env()
                    .unwrap_or_else(|_| "xray=trace".into()),
            ),
        )
        .try_init()
        .context("error while initializing the logging")
}
