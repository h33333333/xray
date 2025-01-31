use dispatcher::Dispatcher;
use store::AppState;
use view::App;

mod action;
pub use action::AppAction;
mod dispatcher;
mod store;
mod view;

pub type AppDispatcher = Dispatcher<AppState, App>;

pub fn init_app_dispatcher() -> AppDispatcher {
    let store = AppState::default();
    let view = App::new();
    Dispatcher::new(store, view)
}
