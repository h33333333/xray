use std::cmp::Ordering;

use ratatui::style::{Color, Modifier, Style};

/// A style for a field's key (name).
pub const FIELD_KEY_STYLE: Style = Style::new().add_modifier(Modifier::BOLD);
/// A style for a field's value.
pub const FIELD_VALUE_STYLE: Style = Style::new();
/// A style of the currently selected field in case its parent pane is currently active.
pub const ACTIVE_FIELD_STYLE: Style =
    Style::new().add_modifier(Modifier::UNDERLINED);
/// A delimiter between the field's name and value.
pub const FIELD_VALUE_DELIMITER: &str = ": ";

pub struct LayerInspectorNodeStyles;

impl LayerInspectorNodeStyles {
    /// A style for a node that is currently selected.
    const SELECTED_NODE_STYLE: Style =
        Style::new().add_modifier(Modifier::REVERSED);

    /// A style for a node that was added in the current layer and is inside the active pane.
    const ACTIVE_PANE_ADDED_NODE_STYLE: Style = Style::new().fg(Color::Green);
    /// A style for a node that was modified in the current layer and is inside the active pane.
    const ACTIVE_PANE_MODIFIED_NODE_STYLE: Style =
        Style::new().fg(Color::Yellow);
    /// A style for a node that was deleted in the current layer and is inside the active pane.
    const ACTIVE_PANE_DELETED_NODE_STYLE: Style = Style::new().fg(Color::Red);

    /// A style for a node that was added in the current layer and is inside the inactive pane.
    const INACTIVE_PANE_ADDED_NODE_STYLE: Style =
        Style::new().fg(Color::Green).add_modifier(Modifier::DIM);
    /// A style for a node that was modified in the current layer and is inside the inactive pane.
    const INACTIVE_PANE_MODIFIED_NODE_STYLE: Style =
        Style::new().fg(Color::Yellow).add_modifier(Modifier::DIM);
    /// A style for a node that was deleted in the current layer and is inside the inactive pane.
    const INACTIVE_PANE_DELETED_NODE_STYLE: Style =
        Style::new().fg(Color::Red).add_modifier(Modifier::DIM);

    pub const fn get_selected_node_style() -> Style {
        Self::SELECTED_NODE_STYLE
    }

    pub const fn get_added_node_style(pane_is_active: bool) -> Style {
        if pane_is_active {
            return Self::ACTIVE_PANE_ADDED_NODE_STYLE;
        }
        Self::INACTIVE_PANE_ADDED_NODE_STYLE
    }

    pub const fn get_modified_node_style(pane_is_active: bool) -> Style {
        if pane_is_active {
            return Self::ACTIVE_PANE_MODIFIED_NODE_STYLE;
        }
        Self::INACTIVE_PANE_MODIFIED_NODE_STYLE
    }

    pub const fn get_deleted_node_style(pane_is_active: bool) -> Style {
        if pane_is_active {
            return Self::ACTIVE_PANE_DELETED_NODE_STYLE;
        }
        Self::INACTIVE_PANE_DELETED_NODE_STYLE
    }
}

/// Returns the base text [Style] based on whether the [Pane](super::Pane) is active.
pub fn text_style(pane_is_active: bool) -> Style {
    if pane_is_active {
        Style::new()
    } else {
        Style::new().add_modifier(Modifier::DIM)
    }
}

/// Returns style for a [crate::parser::Layer] based on its position relative to the currently selected layer.
///
/// This is used in the [super::Pane::LayerSelector] pane.
pub fn layer_status_indicator_style(
    layer_idx: usize,
    selected_layer_idx: &usize,
) -> Style {
    let style = Style::default();
    match layer_idx.cmp(selected_layer_idx) {
        Ordering::Equal => style.bg(Color::Green),
        Ordering::Less => style.bg(Color::Magenta),
        Ordering::Greater => style,
    }
}
