use super::actions::{Action, ActionType};
use super::dispatcher::Dispatcher;

pub trait Store: ActionHandler {
    /// Subscribes to updates for all the actions that this [Store] cares about.
    fn register(&self, dispatcher: &mut Dispatcher);
}

impl<T> Store for T
where
    T: ActionHandler,
{
    fn register(&self, dispatcher: &mut Dispatcher) {
        dispatcher.register_store(self)
    }
}

/// A handler for any number of the supported [actions](Action).
pub trait ActionHandler {
    /// Handles the provided [Action].
    ///
    /// Handlers are expected to handle only the [actions](Action) they care about and ignore the rest.
    ///
    /// A handler must use the interior mutability to modify any of its parts when handling an action.
    /// This is because views also need to reference the [ActionHandler] in order to read the data from it whenever it changes.
    fn handle(&self, action: Action) -> anyhow::Result<()>;

    /// Returns a list of all the [actions](Action) that this handler cares about.
    fn get_relevant_actions(&self) -> &'static [ActionType];
}
