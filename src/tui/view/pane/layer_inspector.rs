use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::Context;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::parser::LayerChangeSet;
use crate::tui::action::Direction;
use crate::tui::store::AppState;
use crate::tui::util::bytes_to_human_readable_units;

const BRANCH_INDICATOR_LENGTH: usize = 4;
const BRANCH_INDICATOR: &str = "│   ";
const BRANCH_SPACER: &str = "    ";
const ACTIVE_LEVEL_PREFIX: &str = "├─";
const INACTIVE_LEVEL_PREFIX: &str = "└─";
const COLLAPSED_NODE_STATUS_INDICATOR: &str = "⊕";
const EXPANDED_NODE_STATUS_INDICATOR: &str = "─";

// FIXME: reset state of this pane when selecting layers.
/// [super::Pane::LayerInspector]'s pane state.
#[derive(Debug, Default)]
pub struct LayerInspectorPane {
    /// Index of the currently selected node in the tree.
    pub current_node_idx: usize,
    /// Maps indexes of all nodes that are collapsed to the number of their children.
    pub collapsed_nodes: BTreeMap<usize, usize>,
}

impl LayerInspectorPane {
    /// Resets collapsed states and the current node index.
    pub fn reset(&mut self) {
        // TODO: make iter not expand collapsed directories
        // TODO: somehow show that a directory is collapsed when rendering
        // TODO: make iter support dynamic collapsing (like when user wants to collapse/expand all directories and we don't know their indexes)
        // TODO: track in which layer an entry was last modified
        // TODO: allow iter do path-based filtering self.current_node_idx = 0;
        self.collapsed_nodes.clear();
    }

    pub fn changeset_to_lines<'a>(
        &self,
        changeset: &'a LayerChangeSet,
        _changeset_size: usize,
        get_node_style: impl Fn(bool) -> Style,
        visible_rows: u16,
    ) -> anyhow::Result<Vec<Line<'a>>> {
        let mut lines = vec![];

        let current_node_idx = self.current_node_idx + 1 /* skip the top-level element */;

        let visible_rows: usize = visible_rows.into();
        let nodes_to_skip = self.nodes_to_skip_before_current_node(visible_rows);

        let mut iter = changeset.iter_with_levels().enumerate(); // HACK: mimic the `Skip` combinator
        iter.nth(nodes_to_skip);
        'outer: while let Some((idx, (path, node, depth, level_is_active))) = iter.next() {
            // Check if any parent of this node is collapsed
            for (node_idx, n_of_children) in self
                .collapsed_nodes
                .iter()
                .take_while(|(&node_idx, _)| node_idx < idx - 1)
            {
                if node_idx + n_of_children >= idx - 1 {
                    // Some parent of this node is collapsed, don't render it
                    continue 'outer;
                }
            }

            let (node_size, unit) = bytes_to_human_readable_units(node.size());
            let node_is_active = idx == current_node_idx;

            let mut spans = vec![Span::styled(
                format!("   {:>5.1} {:<2}   ", node_size, unit.human_readable()),
                get_node_style(node_is_active),
            )];

            let mut node_tree_branch = String::with_capacity((depth - 1) * BRANCH_INDICATOR_LENGTH);
            // Skip the "." node
            (1..depth).for_each(|depth| {
                let prefix = if iter.is_level_active(depth).unwrap_or(false) {
                    BRANCH_INDICATOR
                } else {
                    BRANCH_SPACER
                };
                node_tree_branch.push_str(prefix);
            });

            let node_name_prefix = if level_is_active {
                ACTIVE_LEVEL_PREFIX
            } else {
                INACTIVE_LEVEL_PREFIX
            };
            let status_prefix = if self.is_node_collapsed(idx - 1 /* account for the skipped "." node */) {
                COLLAPSED_NODE_STATUS_INDICATOR
            } else {
                EXPANDED_NODE_STATUS_INDICATOR
            };

            write!(
                &mut node_tree_branch,
                "{}{} {}",
                node_name_prefix,
                status_prefix,
                path.display()
            )
            .with_context(|| format!("failed to format a node {}", idx))?;

            spans.push(Span::styled(node_tree_branch, get_node_style(node_is_active)));
            lines.push(Line::from(spans));

            // No need to process more entries than we can display
            if lines.len() == visible_rows {
                break;
            }
        }

        Ok(lines)
    }

    pub fn move_within_pane(&mut self, direction: Direction, state: &AppState) -> anyhow::Result<()> {
        let (tree, total_nodes) = state.get_selected_layers_changeset()?;
        let total_nodes = total_nodes - 1 /* ignore the "." elemeent */;
        let n_of_current_node_child_nodes = self
            .is_current_node_collapsed()
            .then(|| {
                if let Some((_, (_, current_node, _, _))) = tree.iter().enumerate().nth(self.current_node_idx + 1) {
                    current_node.get_n_of_child_nodes()
                } else {
                    tracing::debug!(
                        current_node_idx = self.current_node_idx,
                        "Layer inspector: current node has invalid index"
                    );
                    None
                }
            })
            .flatten();

        match direction {
            Direction::Forward => {
                self.current_node_idx =
                    (self.current_node_idx + 1 + n_of_current_node_child_nodes.unwrap_or(0)) % total_nodes
            }
            Direction::Backward => {
                // Basic idx calculations
                let mut next_node_idx = self
                    .current_node_idx
                    .checked_sub(1)
                    .unwrap_or(total_nodes - 1 /* we need a zero-based index here */);

                // Iterate starting from the topmost nodes and find the first node that is collapsed and that the calculated next node is the child of.
                for (node_idx, n_of_children) in self
                    .collapsed_nodes
                    .iter()
                    .take_while(|(&node_idx, _)| node_idx < next_node_idx)
                {
                    if node_idx + n_of_children >= next_node_idx {
                        // If we find such a node, jump to it instead of a node at the calculated index.
                        next_node_idx = *node_idx;
                        break;
                    }
                }

                self.current_node_idx = next_node_idx;
            }
        }

        Ok(())
    }

    pub fn toggle_active_node(&mut self, state: &AppState) -> anyhow::Result<()> {
        let (tree, _) = state.get_selected_layers_changeset()?;
        let (_, (_, current_node, _, _)) = tree
            .iter()
            .enumerate()
            .nth(self.current_node_idx + 1)
            .context("bug: current node has invalid index")?;

        // Mark current directory as collapsed
        if current_node.is_dir() && self.collapsed_nodes.remove(&self.current_node_idx).is_none() {
            self.collapsed_nodes.insert(
                self.current_node_idx,
                current_node
                    .get_n_of_child_nodes()
                    .context("bug: should have been unreacheable")?,
            );
        }

        Ok(())
    }

    fn is_node_collapsed(&self, idx: usize) -> bool {
        self.collapsed_nodes.contains_key(&idx)
    }

    fn is_current_node_collapsed(&self) -> bool {
        self.is_node_collapsed(self.current_node_idx)
    }

    fn nodes_to_skip_before_current_node(&self, visible_rows: usize) -> usize {
        let mut hidden_nodes_before_current = 0;
        let mut iter = self
            .collapsed_nodes
            .iter()
            .take_while(|(&idx, _)| idx < self.current_node_idx);
        let mut next_item = iter.next();
        while let Some((idx, children)) = next_item {
            hidden_nodes_before_current += children;
            // Skip children of the item that we just processed
            next_item = iter.find(|(&next_idx, _)| next_idx > idx + children);
        }

        (self.current_node_idx - hidden_nodes_before_current + 1).saturating_sub(visible_rows)
    }
}
