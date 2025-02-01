mod pane;

use anyhow::Context;
pub use pane::Pane;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::{DefaultTerminal, Frame};

use super::store::{AppState, Store};

/// By default, panes are located as follows:
///     1. Upper left pane  - image information pane.
///     2. Bottom left pane - layer selection pane.
///     3. Right pane       - layer diff pane.
const PANE_ORDER: [Pane; 3] = [Pane::ImageInfo, Pane::LayerSelector, Pane::LayerInspector];

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

fn render(frame: &mut Frame, state: &AppState) {
    let rectangles = split_layout(frame.area());

    for (idx, pane) in PANE_ORDER.iter().enumerate() {
        let pane_area = rectangles[idx];
        frame.render_widget(pane.render(state), pane_area);
    }
}

/// Splits the passed area into two columns, also splitting the first column vertically.
///
/// Returns an array that contains upper left, lower left, and right [Rect].
fn split_layout(initial_area: Rect) -> [Rect; 3] {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(initial_area);
    let [upper_left, lower_left] =
        Layout::vertical([Constraint::Percentage(10), Constraint::Percentage(90)]).areas(left);

    [upper_left, lower_left, right]
}
