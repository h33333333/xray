use anyhow::Context;

use super::store::Store;
use super::view::View;

/// A Flux Dispatcher
pub struct Dispatcher<S, V> {
    store: S,
    view: V,
}

impl<S, V> Dispatcher<S, V> {
    /// Creates a new [Dispatcher] for the provided [Store] and [View].
    pub fn new(store: S, view: V) -> Self {
        Dispatcher { store, view }
    }
}

impl<S, V> Dispatcher<S, V>
where
    S: Store,
    V: View<S>,
{
    /// Calls the [Dispatcher::store] to handle the provided [Action].
    ///
    /// Also notifies the [Dispatcher::view] about the change so that it may update accordinglyfor.
    pub fn dispatch(&mut self, action: S::Action) -> anyhow::Result<()> {
        self.store.handle(action).context("failed to handle the action")?;
        self.view
            .on_update(&self.store)
            .context("failed to update the view with the latest data from the store")
    }

    /// Returns a reference to the [store](S).
    pub fn store(&self) -> &S {
        &self.store
    }
}
