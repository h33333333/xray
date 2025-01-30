use std::collections::HashMap;

use super::actions::{Action, ActionType};
use super::store::{ActionHandler, Store};

#[derive(Default)]
/// A Flux Dispatcher that is using the *index pointer pattern* to store [handlers](ActionHandler).
pub struct Dispatcher<'a> {
    /// Contains all the [action handlers](ActionHandler) that were registered through this dispatcher.
    ///
    /// # Safety
    ///
    /// It's assumed that this vector is **append-only**.
    ///
    /// Deleting any item from it requires changing the indices in [Dispatcher::callback_registry].
    handler_registry: Vec<&'a dyn ActionHandler>,
    /// Contains a map of [action](super::actions::Action) types to the handlers from [Dispatcher::handlers].
    handlers: HashMap<ActionType, Vec<usize>>,
}

impl<'a> Dispatcher<'a> {
    /// Registers the provided [Store] within this [Dispatcher].
    pub fn register_store(&mut self, store: &dyn Store) {
        let handler_idx = self.handler_registry.len();
        store.get_relevant_actions().iter().copied().for_each(|action_type| {
            self.handlers.entry(action_type).or_default().push(handler_idx);
        })
    }

    /// Calls all registered [action handlers](ActionHandler) for the provided [Action].
    pub fn dispatch(&self, action: Action) -> anyhow::Result<()> {
        if let Some(callbacks_idxs) = self.handlers.get(&action.get_action_type()) {
            callbacks_idxs
                .iter()
                .try_for_each(|callback_idx| self.handler_registry[*callback_idx].handle(action.clone()))?;
        }

        Ok(())
    }
}
