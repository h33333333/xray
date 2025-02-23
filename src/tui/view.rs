mod active_pane;
mod macros;
mod pane;

use std::io;

pub use active_pane::ActivePane;
use anyhow::Context;
pub use pane::{ImageInfoPane, LayerInfoPane, LayerInspectorPane, LayerSelectorPane, Pane};
use ratatui::layout::{Constraint, Layout, Rect};
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
            .try_draw(|frame| render(frame, store).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e))))
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

/// Renders the [AppState::panes] in the provided [Frame].
///
/// By default, panes are placed as follows:
///     1. Upper left pane - image information pane.
///     2. Middle left pane - layer information pane.
///     3. Bottom left pane - layer selection pane.
///     4. Right pane - layer diff pane.
fn render(frame: &mut Frame, state: &AppState) -> anyhow::Result<()> {
    let pane_areas = split_layout(frame.area());

    debug_assert_eq!(
        pane_areas.len(),
        state.panes.len(),
        "Each pane should have a corresponding rect that it will be rendered in"
    );

    // Panes are always sorted by the render order, so we can just zip rects and panes here,
    // as the order won't change during runtime.
    for (pane_area, pane) in pane_areas.into_iter().zip(state.panes.iter()) {
        frame.render_widget(
            pane.as_ref()
                .context("bug: pane wasn't returned back after an operation")?
                .render(state, pane_area.height)
                .context("failed to render a frame")?,
            pane_area,
        );
    }

    Ok(())
}

/// Splits the passed [Rect] into two equal columns, also splitting the first column into three vertical sections.
///
/// Returns an array that contains upper left, middle left, lower left, and right [Rect].
fn split_layout(initial_area: Rect) -> [Rect; 4] {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(initial_area);
    let [upper_left, middle_left, lower_left] =
        Layout::vertical([Constraint::Min(8), Constraint::Min(10), Constraint::Percentage(100)]).areas(left);

    [upper_left, middle_left, lower_left, right]
}
