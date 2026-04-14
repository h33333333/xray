use crossterm_keybind::KeyBindTrait;
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, Widget};

use crate::keybindings::KeyAction;
use crate::tui::store::AppState;

/// A command bar that shows the most important hotkeys for the current [supper::Pane].
pub struct CommandBar {}

impl CommandBar {
    /// Renders the command bar.
    pub fn render(state: &AppState) -> anyhow::Result<impl Widget> {
        let action = if state.show_help_popup {
            "close"
        } else {
            "open"
        };

        Ok(Paragraph::new(format!(
            "{} - {action} help",
            KeyAction::ToggleHelp.key_bindings_display()
        ))
        .centered()
        .dim())
    }
}
