use super::action::AppAction;
use super::view::{ActivePane, Pane};

/// A Flux store that can handle a [Store::Action].
pub trait Store {
    /// A Flux action that this store supports and can handle.
    type Action;

    /// Handles the [Store::Action].
    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()>;
}

pub struct AppState {
    /// By default, panes are placed as follows:
    ///     1. Upper left pane - image information pane.
    ///     2. Bottom left pane - layer selection pane.
    ///     3. Right pane - layer diff pane.
    pub panes: [Pane; 3],
    pub active_pane: ActivePane,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            panes: [Pane::ImageInfo, Pane::LayerSelector, Pane::LayerInspector],
            active_pane: ActivePane::default(),
        }
    }
}

impl Store for AppState {
    type Action = AppAction;

    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()> {
        match action {
            AppAction::Empty => tracing::trace!("Received an empty event"),
            AppAction::TogglePane => self.active_pane.toggle(),
        };

        Ok(())
    }
}
