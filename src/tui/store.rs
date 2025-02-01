use super::action::AppAction;
use super::view::Pane;

/// A Flux store that can handle a [Store::Action].
pub trait Store {
    /// A Flux action that this store supports and can handle.
    type Action;

    /// Handles the [Store::Action].
    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct AppState {
    pub active_pane: Pane,
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
