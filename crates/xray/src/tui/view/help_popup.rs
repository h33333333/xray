use anyhow::Context as _;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};

use super::ActivePane;
use super::pane::{
    FIELD_KEY_STYLE, FIELD_VALUE_STYLE, LayerInspectorNodeStyles,
};
use crate::tui::store::AppState;

const COLOR_GUIDE: &[(&[Color], &str)] = &[
    (
        &[
            LayerInspectorNodeStyles::get_added_node_style(true)
                .fg
                .expect("should be present"),
            LayerInspectorNodeStyles::get_added_node_style(false)
                .fg
                .expect("should be present"),
        ],
        "Added in the current layer",
    ),
    (
        &[
            LayerInspectorNodeStyles::get_modified_node_style(true)
                .fg
                .expect("should be present"),
            LayerInspectorNodeStyles::get_modified_node_style(false)
                .fg
                .expect("should be present"),
        ],
        "Modified in the current layer",
    ),
    (
        &[
            LayerInspectorNodeStyles::get_deleted_node_style(true)
                .fg
                .expect("should be present"),
            LayerInspectorNodeStyles::get_deleted_node_style(false)
                .fg
                .expect("should be present"),
        ],
        "Deleted in the current layer",
    ),
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
                top: 2,
                bottom: 0,
            })
            .title(Line::from("  Help  ").centered());

        let mut hotkeys = get_common_hotkeys();
        get_hotkeys_for_active_pane(&mut hotkeys, state.active_pane);
        // Make sure that the keys are sorted in the descending hotkey length order
        hotkeys.sort_by(|(hk_a, _), (hk_b, _)| hk_b.len().cmp(&hk_a.len()));

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
fn get_common_hotkeys() -> Vec<(&'static str, &'static str)> {
    vec![
        ("down, j", "move cursor down"),
        ("up, k", "move cursor up"),
        ("s-tab", "select previous pane"),
        ("tab", "select next pane"),
        ("q", "exit the app"),
        ("1, 2, 3, 4", "select the corresponding pane"),
    ]
}

/// Returns contextualized hotkeys that are relevant to the provided [ActivePane].
fn get_hotkeys_for_active_pane(
    hotkeys: &mut Vec<(&'static str, &'static str)>,
    active_pane: ActivePane,
) {
    match active_pane {
        ActivePane::ImageInfo | ActivePane::LayerInfo => {
            hotkeys.push(("y", "copy the selected value to the clipboard"));
        }
        ActivePane::LayerInspector => {
            hotkeys.push(("enter, space", "toggle the selected directory"));
            hotkeys.push(("ctrl-f", "show the filter popup"));
            hotkeys.push(("y", "copy path to the clipboard"));
            hotkeys.push(("c", "show only changed files"));
        }
        ActivePane::LayerSelector => {
            hotkeys.push(("left, h", "scroll left"));
            hotkeys.push(("right, l", "scroll right"));
        }
    }
}

/// Formats the "hotkeys" section in the help popup.
fn format_hotkeys_section(
    hotkeys: Vec<(&'static str, &'static str)>,
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
                FIELD_KEY_STYLE.fg(Color::LightBlue),
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
    .chain(COLOR_GUIDE.iter().map(|&(colors, description)| {
        let mut colors = colors
            .iter()
            .map(|&color| Span::styled("   ", Style::new().bg(color)))
            .intersperse(Span::styled(" or ", FIELD_VALUE_STYLE))
            .collect::<Vec<_>>();

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
