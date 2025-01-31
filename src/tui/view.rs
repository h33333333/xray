use anyhow::Context;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::Block;
use ratatui::{DefaultTerminal, Frame};

use super::store::{AppState, Store};

/// A Flux view that works with a specific [Store].
pub trait View<S: Store> {
    /// Updates [View] according to the latest changes in the [Store].
    fn on_update(&mut self, store: &S) -> anyhow::Result<()>;
}

pub struct App {
    terminal: DefaultTerminal,
}

impl App {
    /// Creates a new [App] instance and initializes the terminal.
    ///
    /// *Terminal is restored automatically in [App::drop]*.
    pub fn new() -> Self {
        App::default()
    }

    /// Renders a [Frame] to the [App::terminal] based on the current [AppState].
    fn render(&mut self, store: &AppState) -> anyhow::Result<()> {
        self.terminal
            .draw(|frame| render(frame, store))
            .context("failed to redraw the frame")?;

        Ok(())
    }
}

impl Default for App {
    /// Creates a new [App] instance and initializes the terminal.
    ///
    /// *Terminal is restored automatically in [App::drop]*.
    fn default() -> Self {
        let terminal = ratatui::init();

        App { terminal }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

impl View<AppState> for App {
    fn on_update(&mut self, store: &AppState) -> anyhow::Result<()> {
        // Update the terminal if state was updated
        self.render(store)
    }
}

fn render(frame: &mut Frame, _store: &AppState) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(frame.area());
    let [upper_left, lower_left] =
        Layout::vertical([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(left);
    frame.render_widget(Block::bordered().title("Image information"), upper_left);
    frame.render_widget(Block::bordered().title("Layers"), lower_left);
    frame.render_widget(Block::bordered().title("Layer changes"), right);
}
