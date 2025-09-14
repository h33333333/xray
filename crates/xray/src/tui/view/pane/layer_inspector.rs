use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use anyhow::Context;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::filter_popup::FilterPopup;
use crate::parser::LayerChangeSet;
use crate::tui::action::Direction;
use crate::tui::store::AppState;
use crate::tui::util::Unit;

const BRANCH_INDICATOR_LENGTH: usize = 4;
const BRANCH_INDICATOR: &str = "│   ";
const BRANCH_SPACER: &str = "    ";
const ACTIVE_LEVEL_PREFIX: &str = "├─";
const INACTIVE_LEVEL_PREFIX: &str = "└─";
const COLLAPSED_NODE_STATUS_INDICATOR: &str = "⊕";
const EXPANDED_NODE_STATUS_INDICATOR: &str = "─";

/// [super::Pane::LayerInspector]'s pane state.
#[derive(Debug, Default)]
pub struct LayerInspectorPane {
    /// Index of the currently selected node in the tree
    current_node_idx: usize,
    /// Number of collapsed nodes before the current one
    collapsed_nodes_before_current: usize,
    /// Maps indexes of all nodes that are collapsed to the number of their children
    collapsed_nodes: BTreeMap<usize, usize>,
    /// The filter popup state
    filter_popup: FilterPopup,
    /// Whether we are showing the filter popup and are accepting the user's input
    is_showing_filter_popup: bool,
    /// Current aggregated changeset with all user-selected filters applied
    filtered_changeset: Option<(LayerChangeSet, usize)>,
}

impl LayerInspectorPane {
    /// The main entrypoint for rendering this pane.
    ///
    /// It processes the current state and returns back the lines that should be rendered in the pane.
    pub fn changeset_to_lines<'a>(
        &self,
        changeset: &'a LayerChangeSet,
        get_node_style: impl Fn(bool, u8, bool, bool) -> Style,
        visible_rows: u16,
    ) -> anyhow::Result<Vec<Line<'a>>> {
        let mut lines = vec![];

        let visible_rows: usize = visible_rows.into();
        let nodes_to_skip =
            self.nodes_to_skip_before_current_node(visible_rows);

        let changeset =
            if let Some((changeset, _)) = self.filtered_changeset.as_ref() {
                // Use filtered changeset if present
                changeset
            } else {
                changeset
            };

        let mut iter = changeset.iter_with_levels().enumerate();
        if nodes_to_skip != 0 {
            // HACK: mimic the `Skip` combinator
            iter.nth(
                nodes_to_skip - 1, /* this requires a 0-based index */
            );
        }
        'outer: while let Some((idx, (path, node, depth, level_is_active))) =
            iter.next()
        {
            // Check if any parent of this node is collapsed
            for (node_idx, n_of_children) in self
                .collapsed_nodes
                .iter()
                .take_while(|(&node_idx, _)| node_idx < idx)
            {
                if node_idx + n_of_children >= idx {
                    // Some parent of this node is collapsed, don't render it
                    continue 'outer;
                }
            }

            let (node_size, unit) =
                Unit::bytes_to_human_readable_units(node.inner.size());
            let node_is_active =
                idx == self.current_node_idx && !self.is_showing_filter_popup;

            let mut node_tree_branch =
                String::with_capacity(depth * BRANCH_INDICATOR_LENGTH);
            (0..depth).for_each(|depth| {
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
            let status_prefix = if self.is_node_collapsed(idx) {
                COLLAPSED_NODE_STATUS_INDICATOR
            } else {
                EXPANDED_NODE_STATUS_INDICATOR
            };

            write!(&mut node_tree_branch, "{node_name_prefix}{status_prefix}")
                .with_context(|| format!("failed to format a node {idx}"))?;

            let mut spans = vec![
                Span::styled(
                    format!(
                        "   {:>5.1} {:<2}   ",
                        node_size,
                        unit.human_readable()
                    ),
                    get_node_style(
                        node_is_active,
                        node.updated_in,
                        node.inner.is_deleted(),
                        node.inner.is_modified(),
                    ),
                ),
                Span::styled(
                    node_tree_branch,
                    get_node_style(node_is_active, u8::MAX, false, false),
                ),
            ];

            let mut path = format!(" {}", path.display());
            if let Some(link) = node.inner.get_link() {
                write!(&mut path, " -> {}", link.display()).with_context(
                    || format!("failed to format a link {idx}"),
                )?;
            }

            spans.push(Span::styled(
                path,
                get_node_style(
                    node_is_active,
                    node.updated_in,
                    node.inner.is_deleted(),
                    node.inner.is_modified(),
                ),
            ));
            lines.push(Line::from(spans));

            // No need to process more entries than we can display
            if lines.len() == visible_rows {
                break;
            }
        }

        Ok(lines)
    }

    /// Resets collapsed states and the current node index.
    pub fn reset(&mut self) {
        self.current_node_idx = 0;
        self.collapsed_nodes_before_current = 0;
        self.collapsed_nodes.clear();
    }

    /// Updates [Self::filtered_changeset] by applying the active user-provided filters to the provided changeset.
    pub fn filter_current_changeset(
        &mut self,
        changeset: &LayerChangeSet,
        current_layer_idx: u8,
    ) {
        if !self.filter_popup.filters(current_layer_idx).any() {
            // If there are no filters, simply reset the filtered changeset and do nothing else
            self.filtered_changeset = None;
            return;
        };

        let mut filtered_changeset = changeset.clone();
        filtered_changeset.filter(self.filter_popup.filters(current_layer_idx));
        let n_of_nodes = filtered_changeset.iter().count();

        self.filtered_changeset = Some((filtered_changeset, n_of_nodes));
    }

    /// Moves cursor to the next visible node inside the pane.
    pub fn move_within_pane(
        &mut self,
        direction: Direction,
        state: &AppState,
    ) -> anyhow::Result<()> {
        if self.is_showing_filter_popup {
            // Apply this action to the filter popup if it's currently shown
            self.filter_popup.active_filter_input.toggle(direction);
            return Ok(());
        }

        let (tree, total_nodes) = if let Some((tree, total_nodes)) =
            self.filtered_changeset.as_ref()
        {
            // Use the filtered changeset if it's present
            (tree, *total_nodes)
        } else {
            state.get_aggregated_layers_changeset()?
        };

        if total_nodes == 0 {
            return Ok(());
        }

        let n_of_current_node_child_nodes = self
            .is_node_collapsed(self.current_node_idx)
            .then(|| {
                if let Some((_, (_, current_node, _, _))) =
                    tree.iter().enumerate().nth(self.current_node_idx)
                {
                    current_node.inner.get_n_of_child_nodes()
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
                let new_node_idx = (self.current_node_idx
                    + 1
                    + n_of_current_node_child_nodes.unwrap_or(0))
                    % total_nodes;
                self.current_node_idx = new_node_idx;

                let new_n_of_collapsed_nodes = if new_node_idx == 0 {
                    0
                } else {
                    self.collapsed_nodes_before_current
                        + n_of_current_node_child_nodes.unwrap_or(0)
                };
                self.collapsed_nodes_before_current = new_n_of_collapsed_nodes;
            }
            Direction::Backward => {
                // Basic idx calculations
                let mut next_node_idx =
                    self.current_node_idx.checked_sub(1).unwrap_or(
                        total_nodes - 1, /* we need a zero-based index here */
                    );

                let mut collapsed_nodes_before_next_node = 0;
                // Iterate starting from the topmost nodes and find the first node that is collapsed and that the calculated next node is the child of.
                let mut iter = self
                    .collapsed_nodes
                    .iter()
                    .take_while(|(&idx, _)| idx < next_node_idx);
                let mut next_item = iter.next();
                while let Some((node_idx, n_of_children)) = next_item {
                    // Check if the calculated next node is a child of the node we got on the current iteration
                    if node_idx + n_of_children >= next_node_idx {
                        // If we find such a node, jump to it instead of a node at the calculated index.
                        next_node_idx = *node_idx;
                        break;
                    }

                    // Calculate the number of collapsed nodes before the the calculated next node
                    collapsed_nodes_before_next_node += n_of_children;

                    // Find the next collapsed node that is either not a child of the node we got on the current iteration OR
                    // contains the calculated next node
                    next_item =
                        iter.find(|(&next_idx, &next_n_of_children)| {
                            next_idx > node_idx + n_of_children
                                || next_idx + next_n_of_children
                                    >= next_node_idx
                        });
                }

                self.current_node_idx = next_node_idx;
                self.collapsed_nodes_before_current =
                    collapsed_nodes_before_next_node;
            }
        }

        Ok(())
    }

    /// Collapses the current directory OR changes the settings of the currently active filter (if one is shown).
    ///
    /// Does nothing if the current entry is a file.
    pub fn toggle_active_node(
        &mut self,
        state: &AppState,
    ) -> anyhow::Result<()> {
        if self.is_showing_filter_popup {
            self.filter_popup.toggle_active_input();
            return Ok(());
        }

        let (tree, total_nodes) = if let Some((tree, total_nodes)) =
            self.filtered_changeset.as_ref()
        {
            // Use the filtered changeset if it's present
            (tree, *total_nodes)
        } else {
            state.get_aggregated_layers_changeset()?
        };

        if total_nodes == 0 {
            return Ok(());
        }

        let (_, (_, current_node, _, _)) = tree
            .iter()
            .enumerate()
            .nth(self.current_node_idx)
            .context("bug: current node has invalid index")?;

        // Mark current directory as collapsed
        if current_node.inner.is_dir()
            && self
                .collapsed_nodes
                .remove(&self.current_node_idx)
                .is_none()
        {
            self.collapsed_nodes.insert(
                self.current_node_idx,
                current_node
                    .inner
                    .get_n_of_child_nodes()
                    .context("bug: should have been unreacheable")?,
            );
        }

        Ok(())
    }

    /// Toggles the filter popup.
    pub fn toggle_filter_popup(&mut self) -> bool {
        self.is_showing_filter_popup = !self.is_showing_filter_popup;
        self.is_showing_filter_popup
    }

    /// Appends to the currently active filter in the filter popup.
    pub fn append_to_filter(&mut self, input: char) {
        self.filter_popup.append_to_filter(input);
    }

    /// Pops from the currently active filter in the filter popup.
    pub fn pop_from_filter(&mut self) {
        self.filter_popup.pop_from_filter();
    }

    /// Toggles the [FilterPopup::show_only_changed_files] filter.
    pub fn toggle_show_only_changed_files(&mut self) {
        self.filter_popup.toggle_show_only_changed_files();
    }

    /// Returns a filter popup if it should be shown on the screen.
    pub fn filter_popup(&self) -> Option<&FilterPopup> {
        self.is_showing_filter_popup.then_some(&self.filter_popup)
    }

    /// Returns a string representation of absolute path to the currently selected node.
    pub fn get_current_node_full_path(
        &self,
        state: &AppState,
    ) -> anyhow::Result<Cow<'static, str>> {
        let (tree, _) = if let Some((tree, total_nodes)) =
            self.filtered_changeset.as_ref()
        {
            // Use the filtered changeset if it's present
            (tree, *total_nodes)
        } else {
            state.get_aggregated_layers_changeset()?
        };

        // Reconstruct the path to the currently selected node
        let path = tree
            .iter()
            .enumerate()
            .filter(|(idx, (_, node, _, _))| {
                // We are interested in nodes that contain the current node or are the currently active node
                *idx == self.current_node_idx
                    || node.inner.get_n_of_child_nodes().is_some_and(
                        |n_of_nodes| {
                            idx + n_of_nodes >= self.current_node_idx
                                && *idx <= self.current_node_idx
                        },
                    )
            })
            .try_fold(String::new(), |mut acc, (_, (path, _, _, _))| {
                write!(acc, "/{}", path.display())?;
                Result::<String, fmt::Error>::Ok(acc)
            })?;

        Ok(path.into())
    }

    /// Returns true if node at the provided index is currently collapsed.
    fn is_node_collapsed(&self, idx: usize) -> bool {
        self.collapsed_nodes.contains_key(&idx)
    }

    /// Returns that total number of nodes that should be skipped in order to fit the current node into the screen.
    fn nodes_to_skip_before_current_node(&self, visible_rows: usize) -> usize {
        // Convert to zero-based index
        let visible_space = visible_rows.saturating_sub(1);
        // Calculate how many nodes we need to skip without accouting for any collapsed directories
        let base_skip_count = self.current_node_idx.saturating_sub(
            self.collapsed_nodes_before_current + visible_space,
        );

        if base_skip_count == 0 {
            // We don't need to skip any nodes
            return 0;
        }

        // Now we need to adjust the skip count to account for collapsed directories
        let mut adjusted_skip_count = base_skip_count;

        let mut iter = self
            .collapsed_nodes
            .iter()
            .take_while(|(&idx, _)| idx < self.current_node_idx);
        let mut next_item = iter.next();
        while let Some((idx, n_of_children)) = next_item {
            // We are interested in collapsed nodes that are within our adjusted skip range
            if *idx >= adjusted_skip_count {
                // We are not interested in the rest of the nodes, as they are outside the skip range
                break;
            }

            // Adjust the skip count by the number of children of this node
            adjusted_skip_count += n_of_children;
            // Find the next node that is not a child of the current one
            next_item =
                iter.find(|(&next_idx, _)| next_idx > idx + n_of_children);
        }

        adjusted_skip_count
    }
}
