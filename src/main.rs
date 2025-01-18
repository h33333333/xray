use std::path::Path;

use xray::init_logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging(Path::new("."))?;

    Ok(())
}
