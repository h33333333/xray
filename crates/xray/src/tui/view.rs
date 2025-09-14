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
pub use pane::{Pane, init_panes};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::widgets::Clear;
use ratatui::{DefaultTerminal, Frame};
pub use side_effect::SideEffect;

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
            .try_draw(|frame| render(frame, store).map_err(io::Error::other))
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
    // Render main panes
    for (pane, pane_area) in state.panes.iter() {
        frame.render_widget(
            pane.as_ref()
                .context("bug: pane wasn't returned back after an operation")?
                .render(state, pane_area.height, pane_area.width)
                .context("failed to render a frame")?,
            *pane_area,
        );
    }

    // Render the command bar
    frame.render_widget(
        CommandBar::render(state)
            .context("failed to redner the command bar")?,
        state.command_bar_area,
    );

    // Render the help popup if it's active
    if state.show_help_popup {
        let popup_area = popup_area(
            frame.area(),
            Some(Constraint::Length(23)),
            Some(Constraint::Length(75)),
        );
        clear_area(frame, popup_area);
        frame.render_widget(
            HelpPopup::render(state)
                .context("failed to render the help popup")?,
            popup_area,
        );
    }

    Ok(())
}

/// Returns a [Rect] that can be used to show a centered popup.
fn popup_area(
    area: Rect,
    vertical_constraints: impl IntoIterator<Item = Constraint>,
    horizontal_constraint: impl IntoIterator<Item = Constraint>,
) -> Rect {
    let vertical = Layout::vertical(vertical_constraints).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let horizontal =
        Layout::horizontal(horizontal_constraint).flex(Flex::Center);
    let [area] = horizontal.areas(area);
    area
}

/// Clears the provided area by rendering the [Clear] widget onto it.
fn clear_area(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
}
