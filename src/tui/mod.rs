use dispatcher::Dispatcher;
use store::AppState;
use view::App;

mod action;
pub use action::AppAction;

use crate::parser::Image;
mod dispatcher;
mod store;
mod view;

pub type AppDispatcher = Dispatcher<AppState, App>;

pub fn init_app_dispatcher(image: Image) -> AppDispatcher {
    let store = AppState::new(image);
    let view = App::new();
    Dispatcher::new(store, view)
}
