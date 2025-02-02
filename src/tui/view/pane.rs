use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Widget};

use crate::tui::store::AppState;

/// All panes that exist in the app.
///
/// Each variant can also hold all the state that a particular pane needs, as
/// these variants are created once during the app initialization and are then reused.
pub enum Pane {
    ImageInfo,
    LayerSelector,
    LayerInspector,
}

impl Pane {
    /// Returns a [Widget] that can be used to render the current pane onto the terminal.
    pub fn render(&self, state: &AppState) -> impl Widget {
        let border_style = if state.active_pane.is_pane_active(self) {
            Style::new().red()
        } else {
            Style::new().white()
        };

        Block::bordered().border_style(border_style).title(self.title())
    }

    /// Returns the pane's title.
    fn title(&self) -> &'static str {
        match self {
            Pane::ImageInfo => "Image information",
            Pane::LayerSelector => "Layers",
            Pane::LayerInspector => "Layer changes",
        }
    }
}

/// A helper enum that tracks the currently active pane and contains the relevant pane-related logic.
///
/// This logic was extracted from [Pane] to avoid having a copy of the currently active [Pane] in [AppState] and instead
/// use a simple and small enum that doesn't hold any pane-related state.
#[derive(Default, PartialEq, Eq)]
pub enum ActivePane {
    #[default]
    ImageInfo,
    LayerSelector,
    LayerInspector,
}

impl ActivePane {
    /// Changes the current pane to the next one.
    pub fn toggle(&mut self) {
        let next_pane = match self {
            ActivePane::ImageInfo => ActivePane::LayerSelector,
            ActivePane::LayerSelector => ActivePane::LayerInspector,
            ActivePane::LayerInspector => ActivePane::ImageInfo,
        };

        *self = next_pane;
    }

    /// Checks if the provided [Pane] is the currently active one.
    pub fn is_pane_active(&self, pane: &Pane) -> bool {
        match self {
            ActivePane::ImageInfo if matches!(pane, Pane::ImageInfo) => true,
            ActivePane::LayerSelector if matches!(pane, Pane::LayerSelector) => true,
            ActivePane::LayerInspector if matches!(pane, Pane::LayerInspector) => true,
            _ => false,
        }
    }
}
