use std::cmp::Ordering;

use ratatui::style::{Color, Modifier, Style};

pub const FIELD_KEY_STYLE: Style = Style::new().add_modifier(Modifier::BOLD);
pub const FIELD_VALUE_STYLE: Style = Style::new().add_modifier(Modifier::ITALIC);
pub const ACTIVE_FIELD_STYLE: Style = Style::new().add_modifier(Modifier::UNDERLINED);
pub const FIELD_VALUE_DELIMITER: &str = ": ";

/// Returns the [Color] of the text that is rendered onto the terminal.
///
/// Can return different colors based on the status of a pane.
pub fn text_color(is_active: bool) -> Color {
    if is_active {
        Color::White
    } else {
        Color::Gray
    }
}

/// Returns a style for a [crate::parser::Layer] based on its comparsion with the currently selected layer.
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
