use anyhow::Context as _;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::style::layer_status_indicator_style;
use crate::parser::{Layer, LayerChangeSet, Sha256Digest};
use crate::tui::action::Direction;
use crate::tui::store::AppState;
use crate::tui::util::Unit;
use crate::tui::view::SideEffect;

const LAYER_STATUS_INDICATOR_LEN: usize = 2;
/// Length of the fixed part (i.e. without the command that created the layer)
const LAYER_INFO_FIXED_LEN: usize = 16;

const NOT_SCROLLABLE_INDICATOR: &str = "  ";
const LEFT_SCROLLABLE_INDICATOR: &str = " ←";
const RIGHT_SCROLLABLE_INDICATOR: &str = "→ ";

/// [Pane::LayerSelector] pane's state.
#[derive(Debug)]
pub struct LayerSelectorPane {
    /// Index of the currently selected layer.
    ///
    /// The index **must** be a valid index that points to an entry in [AppState::layers].
    selected_layer_idx: usize,
    /// An aggregated changeset of all layers up to the current one (including it as well).
    ///
    /// The second value is the total number of entries (both files and directories) in this changeset.
    aggregated_layers_changeset: (LayerChangeSet, usize),
    /// Current horizontal scroll offset
    scroll_offset: usize,
}

impl LayerSelectorPane {
    pub fn new(idx: usize, changeset: LayerChangeSet) -> Self {
        let changeset_size = changeset.iter().count();
        LayerSelectorPane {
            selected_layer_idx: idx,
            aggregated_layers_changeset: (changeset, changeset_size),
            scroll_offset: 0,
        }
    }

    /// Returns index of the currently selected layer.
    pub fn selected_layer_idx(&self) -> usize {
        self.selected_layer_idx
    }

    /// Returns a reference to the aggregated changeset and the number of entries inside it.
    pub fn aggregated_layers_changeset(&self) -> (&LayerChangeSet, usize) {
        (
            &self.aggregated_layers_changeset.0,
            self.aggregated_layers_changeset.1,
        )
    }

    /// The main entrypoint for rendering this pane.
    ///
    /// It processes the current state and returns back the lines that should be rendered in the pane.
    pub fn lines<'l>(
        &self,
        layers: impl IntoIterator<Item = (&'l Sha256Digest, &'l Layer)>,
        field_value_style: Style,
        visible_rows: u16,
        visible_cols: u16,
    ) -> Vec<Line<'l>> {
        // How many columns are left to display the command that created the layer
        let cols_for_created_by = Into::<usize>::into(visible_cols)
            - LAYER_INFO_FIXED_LEN
            - LAYER_STATUS_INDICATOR_LEN;

        layers
            .into_iter()
            .enumerate()
            // Always keep the selected layer visible
            .skip(
                (self.selected_layer_idx + 1)
                    .saturating_sub(Into::<usize>::into(visible_rows)),
            )
            .map(|(idx, (_, layer))| {
                let is_selected_layer = idx == self.selected_layer_idx;
                let (layer_size, unit) =
                    Unit::bytes_to_human_readable_units(layer.size);
                let (created_by, is_scrollable_to_right) =
                    if layer.created_by.len() > cols_for_created_by {
                        let mut start = 0;
                        let mut end = cols_for_created_by;

                        if is_selected_layer
                            && self.scroll_offset + cols_for_created_by
                                >= layer.created_by.len()
                        {
                            start =
                                layer.created_by.len() - cols_for_created_by;
                            end = layer.created_by.len();
                        } else if is_selected_layer {
                            start = self.scroll_offset;
                            end = self.scroll_offset + cols_for_created_by;
                        };

                        (
                            &layer.created_by[start..end],
                            end != layer.created_by.len(),
                        )
                    } else {
                        (layer.created_by.as_ref(), false)
                    };

                let left_scrollable_indicator = if is_selected_layer
                    && self.scroll_offset != 0
                    && layer.created_by.len() > cols_for_created_by
                {
                    LEFT_SCROLLABLE_INDICATOR
                } else {
                    NOT_SCROLLABLE_INDICATOR
                };

                let right_scrollable_indicator =
                    if is_selected_layer && is_scrollable_to_right {
                        RIGHT_SCROLLABLE_INDICATOR
                    } else {
                        NOT_SCROLLABLE_INDICATOR
                    };

                Line::from(vec![
                    // A colored block that acts as an indicator of the currently selected layer.
                    // It's also used to display the layers that are currently used to show aggregated changes.
                    Span::styled(
                        "  ",
                        layer_status_indicator_style(
                            idx,
                            &self.selected_layer_idx,
                        ),
                    ),
                    // Render per-layer information
                    Span::styled(
                        format!(
                            " {:>5.1} {:<2} {} {} {}",
                            layer_size,
                            unit.human_readable(),
                            left_scrollable_indicator,
                            created_by,
                            right_scrollable_indicator
                        ),
                        field_value_style,
                    ),
                ])
            })
            .collect::<Vec<_>>()
    }

    /// Selects the next layer in the specified direction.
    pub fn move_within_pane(
        &mut self,
        direction: Direction,
        state: &AppState,
    ) -> anyhow::Result<Option<SideEffect>> {
        let current_layer_idx = self.selected_layer_idx;

        if current_layer_idx == state.layers.len() - 1
            && matches!(direction, Direction::Forward)
            || current_layer_idx == 0
                && matches!(direction, Direction::Backward)
        {
            // Don't allow cycling through the layers endlessly
            return Ok(None);
        }

        let next_layer_idx = match direction {
            Direction::Forward => (current_layer_idx + 1) % state.layers.len(),
            Direction::Backward => {
                (current_layer_idx + state.layers.len() - 1)
                    % state.layers.len()
            }
        };

        let all_current_layers =
            state.layers.get_range(..next_layer_idx + 1).context(
                "bug: the next layer idx points to an invalid index",
            )?;

        // Get the first changeset and use it as a base layer for merging the rest of the layers
        let mut aggregated_layers = all_current_layers
            .get_index(0)
            .and_then(|(_, layer)| layer.changeset.clone())
            .context("bug: not a single layer is selected")?;

        // Merge the rest of the layers with the first one
        for (_, layer) in
            all_current_layers.get_range(1..).into_iter().flatten()
        {
            if let Some(changeset) = layer.changeset.as_ref() {
                aggregated_layers = aggregated_layers.merge(changeset.clone())
            }
        }

        // Calculate the total number of entries in the new changeset
        let aggregated_layers_changeset_size = aggregated_layers.iter().count();
        self.selected_layer_idx = next_layer_idx;
        self.aggregated_layers_changeset =
            (aggregated_layers, aggregated_layers_changeset_size);
        // Reset the scroll offset as well
        self.scroll_offset = 0;

        Ok(Some(SideEffect::ChangesetUpdated))
    }

    /// Scrolls the currently selected pane entry horizontally in the specified direction.
    ///
    /// Requires providing the pane area to correctly cap the scroll.
    pub fn scroll(
        &mut self,
        direction: Direction,
        pane_area: (u16, u16),
        state: &AppState,
    ) -> anyhow::Result<()> {
        // How many columns are left to display the command that created the layer
        let cols_for_created_by = Into::<usize>::into(pane_area.0) - LAYER_INFO_FIXED_LEN - LAYER_STATUS_INDICATOR_LEN - 2 /* borders */;

        let (_, current_layer) = state
            .layers
            .get_index(self.selected_layer_idx)
            .context("bug: selected layer idx is invalid")?;
        let layer_creation_cmd_len = current_layer.created_by.len();

        let cap = if layer_creation_cmd_len > cols_for_created_by {
            layer_creation_cmd_len - cols_for_created_by + 1 /* show the last character */
        } else {
            1
        };

        self.scroll_offset = match direction {
            Direction::Forward => (self.scroll_offset + 1) % cap,
            Direction::Backward => self.scroll_offset.saturating_sub(1),
        };

        Ok(())
    }
}
