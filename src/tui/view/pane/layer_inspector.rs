use std::collections::HashSet;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::parser::LayerChangeSet;
use crate::tui::action::Direction;
use crate::tui::store::AppState;
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
        _changeset_size: usize,
        get_node_style: impl Fn(bool) -> Style,
        visible_rows: u16,
    ) -> Vec<Line<'a>> {
        let mut lines = vec![];

        let current_node_idx = self.current_node_idx + 1 /* skip the top-level element */;

        // TODO: I need to make the directories collapsible
        let visible_rows: usize = visible_rows.into();
        let rows_to_skip = current_node_idx.saturating_sub(visible_rows);

        let mut iter = changeset.iter_with_levels().enumerate();
        // HACK: mimic the `Skip` combinator
        iter.nth(rows_to_skip);
        while let Some((idx, (path, node, depth, level_is_active))) = iter.next() {
            let (node_size, unit) = bytes_to_human_readable_units(node.size());
            let node_is_active = idx == current_node_idx;

            let mut spans = vec![Span::styled(
                format!("   {:>5.1} {:<2}   ", node_size, unit.human_readable()),
                get_node_style(node_is_active),
            )];

            // Skip the `0` depth, as it's only for the '.' node
            spans.extend((1..depth).map(|depth| {
                let prefix = if iter.is_level_active(depth).unwrap_or(false) {
                    "│   "
                } else {
                    "    "
                };
                Span::styled(prefix, get_node_style(node_is_active))
            }));

            let path = if level_is_active {
                format!("├── {}", path.display())
            } else {
                format!("└── {}", path.display())
            };

            spans.push(Span::styled(path, get_node_style(node_is_active)));
            lines.push(Line::from(spans));

            // No need to process more entries than we can display
            if lines.len() == visible_rows {
                break;
            }
        }

        lines
    }

    pub fn move_within_pane(&mut self, direction: Direction, state: &AppState) -> anyhow::Result<()> {
        let (_, total_nodes) = state.get_selected_layers_changeset()?;
        let total_nodes = total_nodes - 1 /* ignore the "." elemeent */;
        match direction {
            Direction::Forward => self.current_node_idx = (self.current_node_idx + 1) % total_nodes,
            Direction::Backward => {
                self.current_node_idx = self
                    .current_node_idx
                    .checked_sub(1)
                    .unwrap_or(total_nodes - 1 /* we need a zero-based index here */)
            }
        }

        Ok(())
    }
}
