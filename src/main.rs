use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Context;
use crossterm::event::{self, Event};
use xray::{init_app_dispatcher, init_logging, AppAction, AppDispatcher, Config, Parser};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging(Path::new("."))?;

    let config = Config::new()?;

    let image = File::open(config.image()).context("failed to open the image")?;
    let reader = BufReader::new(image);

    let parser = Parser::default();
    parser.parse_image(reader).context("failed to parse the image")?;

    run(init_app_dispatcher())
}

fn run(mut dispatcher: AppDispatcher) -> anyhow::Result<()> {
    // Do the initial render of the interface
    dispatcher.dispatch(AppAction::Empty)?;

    loop {
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}
