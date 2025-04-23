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
const LAYER_INFO_FIXED_LEN: usize = 10;

#[derive(Debug)]
/// [Pane::LayerSelector] pane's state.
pub struct LayerSelectorPane {
    /// The currently selected layer.
    pub selected_layer_digest: Sha256Digest,
    /// We store both its [LayerSelectorPane::selected_layer_digest] and the index in order to optimize the lookup.
    ///
    /// The index **must** be a valid index that points to an entry in [AppState::layers].
    pub selected_layer_idx: usize,
    /// Either a changeset of a single layer or an aggregated changeset of multiple layers, depending on the chosen display mode.
    /// Also contains stores the total number of entries (both files and directories) in this changeset.
    pub selected_layers_changeset: (LayerChangeSet, usize),
    /// Stores the longest layer creation command ([Layer::created_by]) from all layers in the current image.
    longest_layer_creation_command: usize,
    /// Current horizontal scroll offset
    scroll_offset: usize,
}

impl LayerSelectorPane {
    pub fn new(
        digest: Sha256Digest,
        idx: usize,
        changeset: LayerChangeSet,
        longest_layer_creation_command: usize,
    ) -> Self {
        let changeset_size = changeset.iter().count();
        LayerSelectorPane {
            selected_layer_digest: digest,
            selected_layer_idx: idx,
            selected_layers_changeset: (changeset, changeset_size),
            longest_layer_creation_command,
            scroll_offset: 0,
        }
    }

    pub fn selected_layer(&self) -> (&Sha256Digest, usize) {
        (&self.selected_layer_digest, self.selected_layer_idx)
    }

    pub fn selected_layers_changeset(&self) -> (&LayerChangeSet, usize) {
        (&self.selected_layers_changeset.0, self.selected_layers_changeset.1)
    }

    pub fn lines<'l>(
        &self,
        layers: impl IntoIterator<Item = (&'l Sha256Digest, &'l Layer)>,
        field_value_style: Style,
        visible_rows: u16,
        visible_cols: u16,
    ) -> Vec<Line<'l>> {
        // How many columns are left to display the command that created the layer
        let cols_for_created_by = Into::<usize>::into(visible_cols) - LAYER_INFO_FIXED_LEN - LAYER_STATUS_INDICATOR_LEN;

        layers
            .into_iter()
            .enumerate()
            // Always keep the selected layer visible
            .skip((self.selected_layer_idx + 1).saturating_sub(Into::<usize>::into(visible_rows)))
            .map(|(idx, (_, layer))| {
                let (layer_size, unit) = Unit::bytes_to_human_readable_units(layer.size);
                let created_by = if layer.created_by.len() > cols_for_created_by {
                    let (start, end) = if self.scroll_offset + cols_for_created_by >= layer.created_by.len() {
                        (layer.created_by.len() - cols_for_created_by, layer.created_by.len())
                    } else {
                        (self.scroll_offset, self.scroll_offset + cols_for_created_by)
                    };
                    &layer.created_by[start..end]
                } else {
                    layer.created_by.as_ref()
                };

                Line::from(vec![
                    // A colored block that acts as an indicator of the currently selected layer.
                    // It's also used to display the layers that are currently used to show aggregated changes.
                    Span::styled("  ", layer_status_indicator_style(idx, &self.selected_layer_idx)),
                    // Render per-layer information
                    Span::styled(
                        format!(" {:>5.1} {:<2} {}", layer_size, unit.human_readable(), created_by),
                        field_value_style,
                    ),
                ])
            })
            .collect::<Vec<_>>()
    }

    pub fn move_within_pane(&mut self, direction: Direction, state: &AppState) -> anyhow::Result<Option<SideEffect>> {
        let current_layer_idx = self.selected_layer_idx;

        if current_layer_idx == state.layers.len() - 1 && matches!(direction, Direction::Forward)
            || current_layer_idx == 0 && matches!(direction, Direction::Backward)
        {
            // Don't allow cycling through the layers endlessly
            return Ok(None);
        }

        let next_layer_idx = match direction {
            Direction::Forward => (current_layer_idx + 1) % state.layers.len(),
            Direction::Backward => (current_layer_idx + state.layers.len() - 1) % state.layers.len(),
        };

        let (digest, _) = state
            .layers
            .get_index(next_layer_idx)
            .context("unnable to find the next layer")?;

        let all_current_layers = state
            .layers
            .get_range(..next_layer_idx + 1)
            .context("bug: the next layer idx points to an invalid index")?;

        let mut aggregated_layers = all_current_layers
            .get_index(0)
            .and_then(|(_, layer)| layer.changeset.clone())
            .context("not a single layer in the all currently selected layers")?;

        for (_, layer) in all_current_layers.get_range(1..).into_iter().flatten() {
            if let Some(changeset) = layer.changeset.as_ref() {
                aggregated_layers = aggregated_layers.merge(changeset.clone())
            }
        }

        let aggregated_layers_changeset_size = aggregated_layers.iter().count();
        self.selected_layer_digest = *digest;
        self.selected_layer_idx = next_layer_idx;
        self.selected_layers_changeset = (aggregated_layers, aggregated_layers_changeset_size);

        Ok(Some(SideEffect::ChangesetUpdated))
    }

    pub fn scroll(&mut self, direction: Direction, pane_area: (u16, u16)) {
        // How many columns are left to display the command that created the layer
        let cols_for_created_by = Into::<usize>::into(pane_area.0) - LAYER_INFO_FIXED_LEN - LAYER_STATUS_INDICATOR_LEN - 2 /* borders */;

        let cap = if self.longest_layer_creation_command > cols_for_created_by {
            self.longest_layer_creation_command - cols_for_created_by + 1 /* show the last character */
        } else {
            1
        };

        self.scroll_offset = match direction {
            Direction::Forward => (self.scroll_offset + 1) % cap,
            Direction::Backward => self.scroll_offset.saturating_sub(1),
        };
    }
}
