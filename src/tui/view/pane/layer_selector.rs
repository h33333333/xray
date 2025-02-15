use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::style::layer_status_indicator_style;
use crate::parser::{Layer, LayerChangeSet, Sha256Digest};
use crate::tui::util::bytes_to_human_readable_units;

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
    pub selected_layers_changeset: LayerChangeSet,
}

impl LayerSelectorPane {
    pub fn new(digest: Sha256Digest, idx: usize, changeset: LayerChangeSet) -> Self {
        LayerSelectorPane {
            selected_layer_digest: digest,
            selected_layer_idx: idx,
            selected_layers_changeset: changeset,
        }
    }

    pub fn selected_layer(&self) -> (&Sha256Digest, usize) {
        (&self.selected_layer_digest, self.selected_layer_idx)
    }

    pub fn selected_layers_changeset(&self) -> &LayerChangeSet {
        &self.selected_layers_changeset
    }

    pub fn lines<'l>(
        &self,
        layers: impl IntoIterator<Item = (&'l Sha256Digest, &'l Layer)>,
        field_value_style: Style,
    ) -> Vec<Line<'l>> {
        layers
            .into_iter()
            .enumerate()
            .map(|(idx, (_, layer))| {
                let (layer_size, unit) = bytes_to_human_readable_units(layer.size);
                Line::from(vec![
                    // A colored block that acts as an indicator of the currently selected layer.
                    // It's also used to display the layers that are currently used to show aggregated changes.
                    Span::styled("  ", layer_status_indicator_style(idx, &self.selected_layer_idx)),
                    Span::styled(
                        format!(" {:>5.1} {:<2} {}", layer_size, unit.human_readable(), layer.created_by),
                        field_value_style,
                    ),
                ])
            })
            .collect::<Vec<_>>()
    }
}
