use super::Pane;
use crate::render_order_enum;

// This logic was extracted from [Pane] to avoid having a copy of the currently active [Pane] in [AppState] and instead
// use a simple and small enum that doesn't hold any pane-related state.
render_order_enum!(ActivePane, ImageInfo, LayerInfo, LayerSelector, LayerInspector);

impl From<&Pane> for ActivePane {
    fn from(value: &Pane) -> Self {
        match value {
            Pane::ImageInfo(..) => ActivePane::ImageInfo,
            Pane::LayerInfo(..) => ActivePane::LayerInfo,
            Pane::LayerSelector(..) => ActivePane::LayerSelector,
            Pane::LayerInspector => ActivePane::LayerInspector,
        }
    }
}
