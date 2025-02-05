use action::Direction;
use anyhow::Context;
use crossterm::event::{self, Event, KeyCode};
use dispatcher::Dispatcher;
use store::AppState;
use view::App;

mod action;
pub use action::AppAction;

use crate::parser::Image;
mod dispatcher;
mod store;
mod util;
mod view;

pub type AppDispatcher = Dispatcher<AppState, App>;

pub fn init_app_dispatcher(image: Image) -> anyhow::Result<AppDispatcher> {
    let store = AppState::new(image).context("failed to initialize the app state")?;
    let view = App::new();
    Ok(Dispatcher::new(store, view))
}

pub fn run(mut dispatcher: AppDispatcher) -> anyhow::Result<()> {
    // Do the initial render of the interface
    dispatcher.dispatch(AppAction::Empty)?;

    loop {
        let event = event::read()?;

        match event {
            // Re-render the interface when terminal window is resized
            Event::Resize(_, _) => dispatcher.dispatch(AppAction::Empty)?,
            Event::Key(event) if event.code == KeyCode::Char('q') => break Ok(()),
            Event::Key(event) if event.code == KeyCode::Tab => {
                dispatcher.dispatch(AppAction::TogglePane(Direction::Forward))?;
            }
            Event::Key(event) if event.code == KeyCode::BackTab => {
                dispatcher.dispatch(AppAction::TogglePane(Direction::Backward))?;
            }
            Event::Key(event) if event.code == KeyCode::Char('j') || event.code == KeyCode::Down => {
                dispatcher.dispatch(AppAction::Move(Direction::Forward))?;
            }
            Event::Key(event) if event.code == KeyCode::Char('k') || event.code == KeyCode::Up => {
                dispatcher.dispatch(AppAction::Move(Direction::Backward))?;
            }
            Event::Key(event) if event.code == KeyCode::Char('y') => {
                dispatcher.dispatch(AppAction::Copy)?;
            }
            evt => tracing::trace!("Ignoring an event: {:?}", evt),
        }
    }
}
