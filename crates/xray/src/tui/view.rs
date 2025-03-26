mod active_pane;
mod command_bar;
mod help_popup;
mod macros;
mod pane;
mod side_effect;
mod widgets;

use std::io;

pub use active_pane::ActivePane;
use anyhow::Context;
use command_bar::CommandBar;
use help_popup::HelpPopup;
pub use pane::{ImageInfoPane, LayerInfoPane, LayerInspectorPane, LayerSelectorPane, Pane};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::Clear;
use ratatui::{DefaultTerminal, Frame};
pub use side_effect::SideEffect;

use super::store::{AppState, Store};

type CommandBarArea = Rect;
type PaneAreas = [Rect; 4];

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
///
/// This function also renders the command bar below the main panes and
/// the help popup if it's currently visible.
fn render(frame: &mut Frame, state: &AppState) -> anyhow::Result<()> {
    let (pane_areas, command_bar) = split_layout(frame.area());

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

    frame.render_widget(
        CommandBar::render(state).context("failed to redner the command bar")?,
        command_bar,
    );

    if state.show_help_popup {
        let popup_area = popup_area(frame.area(), None, None);
        clear_area(frame, popup_area);
        frame.render_widget(
            HelpPopup::render(state).context("failed to render the help popup")?,
            popup_area,
        );
    }

    Ok(())
}

/// Splits the passed [Rect] into two equal columns, also splitting the first column into three vertical sections.
///
/// Returns an array that contains upper left, middle left, lower left, and right [Rect].
fn split_layout(initial_area: Rect) -> (PaneAreas, CommandBarArea) {
    let [main, command_bar] = Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)]).areas(initial_area);
    let [left, right] = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(70)]).areas(main);
    let [upper_left, middle_left, lower_left] =
        Layout::vertical([Constraint::Min(8), Constraint::Min(10), Constraint::Percentage(100)]).areas(left);

    ([upper_left, middle_left, lower_left, right], command_bar)
}

/// Returns a [Rect] that can be used to show a centered popup.
fn popup_area(area: Rect, vertical_constraint: Option<Constraint>, horizontal_constraint: Option<Constraint>) -> Rect {
    let vertical = Layout::vertical([vertical_constraint.unwrap_or(Constraint::Percentage(35))]).flex(Flex::Center);
    let horizontal =
        Layout::horizontal([horizontal_constraint.unwrap_or(Constraint::Percentage(35))]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

/// Clears the provided area by rendering the [Clear] widget onto it.
fn clear_area(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
}
