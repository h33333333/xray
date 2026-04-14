use std::borrow::Cow;

use anyhow::Context as _;
use crossterm_keybind::KeyBindTrait as _;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};

use super::ActivePane;
use super::pane::{FIELD_KEY_STYLE, FIELD_VALUE_STYLE};
use crate::keybindings::KeyAction;
use crate::tui::store::AppState;

const COLOR_GUIDE: &[(Color, &str)] = &[
    (Color::Green, "Added in the current layer"),
    (Color::Yellow, "Modified in the current layer"),
    (Color::Red, "Deleted in the current layer"),
];

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
                top: 1,
                bottom: 0,
            })
            .title(Line::from("  Help  ").centered());

        let mut hotkeys = get_common_hotkeys();
        get_hotkeys_for_active_pane(&mut hotkeys, state.active_pane);
        // Make sure that the keys are sorted in the descending hotkey length order
        hotkeys.sort_by_key(|(hk, _)| std::cmp::Reverse(hk.len()));

        let lines = format_color_guide_section()
            // Add a separator between the color guide and hotkeys
            .chain(chainable_blank_line())
            .chain(format_hotkeys_section(hotkeys)?)
            .collect::<Vec<_>>();

        Ok(Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .block(block))
    }
}

/// Returns hotkeys that are common to all panes.
fn get_common_hotkeys() -> Vec<(Cow<'static, str>, &'static str)> {
    vec![
        (
            KeyAction::Down.key_bindings_display().into(),
            "move cursor down",
        ),
        (
            KeyAction::Up.key_bindings_display().into(),
            "move cursor up",
        ),
        (
            KeyAction::PreviousItem.key_bindings_display().into(),
            "select previous pane",
        ),
        (
            KeyAction::NextItem.key_bindings_display().into(),
            "select next pane",
        ),
        (
            KeyAction::CloseActiveWindow.key_bindings_display().into(),
            "close the active window",
        ),
        (
            KeyAction::Exit.key_bindings_display().into(),
            "exit the app",
        ),
        ("1, 2, 3, 4".into(), "select the corresponding pane"),
    ]
}

/// Returns contextualized hotkeys that are relevant to the provided [ActivePane].
fn get_hotkeys_for_active_pane(
    hotkeys: &mut Vec<(Cow<'static, str>, &'static str)>,
    active_pane: ActivePane,
) {
    match active_pane {
        ActivePane::ImageInfo | ActivePane::LayerInfo => {
            hotkeys.push((
                KeyAction::Copy.key_bindings_display().into(),
                "copy the selected value to the clipboard",
            ));
        }
        ActivePane::LayerInspector => {
            hotkeys.push((
                KeyAction::Interact.key_bindings_display().into(),
                "toggle the selected directory",
            ));
            hotkeys.push((
                KeyAction::ToggleFilterPopup.key_bindings_display().into(),
                "show the filter popup",
            ));
            hotkeys.push((
                KeyAction::Copy.key_bindings_display().into(),
                "copy path to the clipboard",
            ));
            hotkeys.push((
                KeyAction::Subaction.key_bindings_display().into(),
                "show only changed files",
            ));
        }
        ActivePane::LayerSelector => {
            hotkeys.push((
                KeyAction::Backward.key_bindings_display().into(),
                "scroll left",
            ));
            hotkeys.push((
                KeyAction::Forward.key_bindings_display().into(),
                "scroll right",
            ));
        }
    }
}

/// Formats the "hotkeys" section in the help popup.
fn format_hotkeys_section(
    hotkeys: Vec<(Cow<'static, str>, &'static str)>,
) -> anyhow::Result<impl Iterator<Item = Line<'static>>> {
    // We need this to pad shorter hotkeys and make the list look less ugly
    let longest_hotkey = hotkeys
        .iter()
        .map(|(hotkey, _)| hotkey.len())
        .max_by(|a, b| a.cmp(b))
        .context("bug: vec with hotkeys is somehow empty")?;

    Ok(Some(
        Line::from(Span::styled("Hotkeys", FIELD_KEY_STYLE.italic()))
            .centered(),
    )
    .into_iter()
    .chain(chainable_blank_line())
    .chain(hotkeys.into_iter().map(move |(hotkey, description)| {
        Line::from(vec![
            Span::styled(
                format!("{hotkey:>longest_hotkey$}  "),
                // Make the hotkeys easier to see among the text
                FIELD_KEY_STYLE.fg(Color::Cyan),
            ),
            Span::styled(description, FIELD_VALUE_STYLE),
        ])
    })))
}

/// Formats the color guide section in the help popup.
fn format_color_guide_section() -> impl Iterator<Item = Line<'static>> {
    Some(
        Line::from(Span::styled(
            "Meaning of file tree colors",
            FIELD_KEY_STYLE.italic(),
        ))
        .centered(),
    )
    .into_iter()
    .chain(chainable_blank_line())
    .chain(COLOR_GUIDE.iter().map(|&(color, description)| {
        let mut colors = vec![Span::styled("   ", Style::new().bg(color))];
        // Separator between colors and their meaning
        colors.push(Span::styled("  ", FIELD_VALUE_STYLE));
        // Actual description
        colors.push(Span::styled(description, FIELD_VALUE_STYLE));

        Line::from(colors)
    }))
}

/// Returns an empty [Line] that can be chained in iterators to create breaks between widgets.
fn chainable_blank_line() -> Option<Line<'static>> {
    Some(Line::from(""))
}
