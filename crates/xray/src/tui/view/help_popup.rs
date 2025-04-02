use anyhow::Context as _;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};

use super::pane::{FIELD_KEY_STYLE, FIELD_VALUE_STYLE};
use super::ActivePane;
use crate::tui::store::AppState;

/// A simple help popup that displays all hotkeys and other useful information.
pub struct HelpPopup {}

impl HelpPopup {
    /// Renders the help popup.
    pub fn render(state: &AppState) -> anyhow::Result<impl Widget> {
        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(ratatui::widgets::Padding {
                left: 5,
                right: 0,
                top: 2,
                bottom: 0,
            })
            .title(Line::from("  Help  ").centered());

        let mut hotkeys = get_common_hotkeys();
        get_hotkeys_for_active_pane(&mut hotkeys, state.active_pane);
        // Make sure that the keys are sorted in the descending hotkey length order
        hotkeys.sort_by(|(hk_a, _), (hk_b, _)| hk_b.len().cmp(&hk_a.len()));

        // We need this to pad shorter hotkeys and make the list look less ugly
        let longest_hotkey = hotkeys
            .iter()
            .map(|(hotkey, _)| hotkey.len())
            .max_by(|a, b| a.cmp(b))
            .context("bug: vec with hotkeys is somehow empty")?;

        let lines = hotkeys
            .into_iter()
            .map(|(hotkey, description)| {
                Line::from(vec![
                    Span::styled(format!("{:>longest_hotkey$}  ", hotkey), FIELD_KEY_STYLE),
                    Span::styled(description, FIELD_VALUE_STYLE),
                ])
            })
            .collect::<Vec<_>>();

        Ok(Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .block(block))
    }
}

fn get_common_hotkeys() -> Vec<(&'static str, &'static str)> {
    vec![
        ("down, j", "Move cursor down"),
        ("up, k", "Move cursor up"),
        ("s-tab", "Select previous pane"),
        ("tab", "Select next pane"),
        ("q", "Exit from the app"),
        ("1, 2, 3, 4", "Select the corresponding pane"),
    ]
}

fn get_hotkeys_for_active_pane(hotkeys: &mut Vec<(&'static str, &'static str)>, active_pane: ActivePane) {
    match active_pane {
        ActivePane::ImageInfo | ActivePane::LayerInfo => {
            hotkeys.push(("y", "Copy the selected value to the clipboard"))
        }
        ActivePane::LayerInspector => {
            hotkeys.push(("enter, l", "Toggle the selected directory"));
            hotkeys.push(("ctrl-f", "Show the filter popup"))
        }
        ActivePane::LayerSelector => {
            hotkeys.push(("left, h", "Scroll left"));
            hotkeys.push(("right, l", "Scroll right"));
        }
    }
}
