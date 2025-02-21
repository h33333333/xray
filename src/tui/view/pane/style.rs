use std::cmp::Ordering;

use ratatui::style::{Color, Modifier, Style};

pub const FIELD_KEY_STYLE: Style = Style::new().add_modifier(Modifier::BOLD);
pub const FIELD_VALUE_STYLE: Style = Style::new();
pub const ACTIVE_FIELD_STYLE: Style = Style::new().add_modifier(Modifier::UNDERLINED);
pub const FIELD_VALUE_DELIMITER: &str = ": ";
pub const ACTIVE_INSPECTOR_NODE_STYLE: Style = Style::new().fg(Color::Black).bg(Color::White);

/// Returns the [Color] of the text based on whether its parent [Pane](super::Pane) is active.
pub fn text_color(pane_is_active: bool) -> Color {
    if pane_is_active {
        Color::White
    } else {
        Color::Gray
    }
}

/// Returns style for a [crate::parser::Layer] based on its position relative to the currently selected layer.
///
/// This is used in the [super::Pane::LayerSelector] pane.
pub fn layer_status_indicator_style(layer_idx: usize, selected_layer_idx: &usize) -> Style {
    let style = Style::default();
    match layer_idx.cmp(selected_layer_idx) {
        Ordering::Equal => style.bg(Color::LightGreen),
        Ordering::Less => style.bg(Color::LightMagenta),
        Ordering::Greater => style,
    }
}
