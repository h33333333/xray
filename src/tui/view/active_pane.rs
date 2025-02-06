use super::Pane;
use crate::tui::action::Direction;

/// A helper enum that tracks the currently active pane and contains the relevant pane-related logic.
///
/// This logic was extracted from [Pane] to avoid having a copy of the currently active [Pane] in [AppState] and instead
/// use a simple and small enum that doesn't hold any pane-related state.
#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum ActivePane {
    #[default]
    ImageInfo,
    LayerInfo,
    LayerSelector,
    LayerInspector,
}

impl ActivePane {
    /// Returns an array of all panes in their cycling order.
    const PANE_ORDER: [ActivePane; 4] = [
        ActivePane::ImageInfo,
        ActivePane::LayerInfo,
        ActivePane::LayerSelector,
        ActivePane::LayerInspector,
    ];

    /// Changes the current pane to the next one.
    pub fn toggle(&mut self, direction: Direction) {
        let current_index = Self::PANE_ORDER.iter().position(|pane| pane == self).unwrap();

        let next_index = match direction {
            Direction::Forward => (current_index + 1) % Self::PANE_ORDER.len(),
            Direction::Backward => (current_index + Self::PANE_ORDER.len() - 1) % Self::PANE_ORDER.len(),
        };

        *self = Self::PANE_ORDER[next_index];
    }

    /// Checks if the provided [Pane] is the currently active one.
    pub fn is_pane_active(&self, pane: &Pane) -> bool {
        match self {
            ActivePane::ImageInfo if matches!(pane, Pane::ImageInfo(..)) => true,
            ActivePane::LayerSelector if matches!(pane, Pane::LayerSelector(..)) => true,
            ActivePane::LayerInfo if matches!(pane, Pane::LayerInfo(..)) => true,
            ActivePane::LayerInspector if matches!(pane, Pane::LayerInspector) => true,
            _ => false,
        }
    }

    /// Returns the pane's index in the [AppState::panes] array (aka it's position in the UI grid).
    pub fn pane_idx(&self) -> usize {
        *self as usize
    }
}
