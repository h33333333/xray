use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::Context;
use crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::Block;
use ratatui::{DefaultTerminal, Frame};
use xray::{init_logging, Config, Parser};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging(Path::new("."))?;

    let config = Config::new()?;

    let image = File::open(config.image()).context("failed to open the image")?;
    let reader = BufReader::new(image);

    let parser = Parser::default();
    parser.parse_image(reader).context("failed to parse the image")?;

    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();

    result
}

fn run(mut terminal: DefaultTerminal) -> anyhow::Result<()> {
    loop {
        terminal.draw(render)?;
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(frame.area());
    let [upper_left, lower_left] =
        Layout::vertical([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(left);
    frame.render_widget(Block::bordered().title("Image information"), upper_left);
    frame.render_widget(Block::bordered().title("Layers"), lower_left);
    frame.render_widget(Block::bordered().title("Layer changes"), right);
}
