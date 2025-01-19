use std::path::Path;

use xray::{init_logging, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging(Path::new("."))?;

    let _config = Config::new()?;

    Ok(())
}
