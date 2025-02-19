use std::collections::HashSet;

use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};

use crate::parser::LayerChangeSet;
use crate::tui::action::Direction;
use crate::tui::util::bytes_to_human_readable_units;

/// [super::Pane::LayerInspector]'s pane state.
#[derive(Debug, Default)]
pub struct LayerInspectorPane {
    /// Index of the currently selected node in the tree.
    pub current_node_idx: usize,
    /// Contains indexes of all nodes that are collapsed.
    pub collapsed_nodes: HashSet<usize>,
}

impl LayerInspectorPane {
    /// Resets collapsed states and the current node index.
    pub fn reset(&mut self) {
        // TODO: make iter not expand collapsed directories
        // TODO: somehow show that a directory is collapsed when rendering
        // TODO: make iter support dynamic collapsing (like when user wants to collapse/expand all directories and we don't know their indexes)
        // TODO: track in which layer an entry was last modified
        // TODO: allow iter do path-based filtering
        self.current_node_idx = 0;
        self.collapsed_nodes.clear();
    }

    pub fn changeset_to_lines<'a>(
        &self,
        changeset: &'a LayerChangeSet,
        changeset_size: usize,
        field_value_style: Style,
        visible_rows: u16,
    ) -> Vec<Line<'a>> {
        let mut lines = vec![];

        // FIXME: this is very bad (and also wrong). I need to move this logic to the `move_within_pane` method
        let current_node_idx = if self.current_node_idx >= changeset_size {
            self.current_node_idx % changeset_size
        } else {
            self.current_node_idx
        };

        let visible_rows: usize = visible_rows.into();

        let rows_to_skip = current_node_idx.saturating_sub(visible_rows);

        let mut active_levels = HashSet::new();
        let iter = changeset
            .iter_with_levels()
            .enumerate()
            // Also skip the "."
            .skip(rows_to_skip + 1)
            // No need to process more entries than we can display
            .take(visible_rows);
        for (idx, (path, node, depth, is_level_active)) in iter {
            if is_level_active {
                active_levels.insert(depth)
            } else {
                active_levels.remove(&depth)
            };

            let (node_size, unit) = bytes_to_human_readable_units(node.size());

            let mut spans = vec![Span::styled(
                format!("   {:>5.1} {:<2}   ", node_size, unit.human_readable()),
                field_value_style,
            )];
            spans.extend((1..depth).map(|depth| {
                // FIXME: this doesn't work because of skip
                // - create a new iterator and make it track the active levels instead
                let prefix = if active_levels.contains(&depth) {
                    "│   "
                } else {
                    "    "
                };
                Span::styled(prefix, field_value_style)
            }));

            let path = if is_level_active {
                format!("├── {}", path.display())
            } else {
                format!("└── {}", path.display())
            };

            let style = if idx == current_node_idx {
                field_value_style.bg(Color::White).fg(Color::Black)
            } else {
                field_value_style
            };

            spans.push(Span::styled(path, style.not_italic()));

            lines.push(Line::from(spans))
        }

        lines
    }

    pub fn move_within_pane(&mut self, direction: Direction) {
        match direction {
            Direction::Forward => self.current_node_idx = self.current_node_idx.saturating_add(1),
            Direction::Backward => self.current_node_idx = self.current_node_idx.saturating_sub(1),
        }
    }
}
