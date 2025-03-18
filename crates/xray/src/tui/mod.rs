use action::Direction;
use anyhow::Context;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
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
            // Quit
            Event::Key(event) if event.code == KeyCode::Char('q') => break Ok(()),
            // Select next pane
            Event::Key(event) if event.code == KeyCode::Tab => {
                dispatcher.dispatch(AppAction::TogglePane(Direction::Forward))?;
            }
            // Select previous pane
            Event::Key(event) if event.code == KeyCode::BackTab => {
                dispatcher.dispatch(AppAction::TogglePane(Direction::Backward))?;
            }
            // Move down
            Event::Key(event) if event.code == KeyCode::Char('j') || event.code == KeyCode::Down => {
                dispatcher.dispatch(AppAction::Move(Direction::Forward))?;
            }
            // Move up
            Event::Key(event) if event.code == KeyCode::Char('k') || event.code == KeyCode::Up => {
                dispatcher.dispatch(AppAction::Move(Direction::Backward))?;
            }
            // Copy the selected item to clipboard
            Event::Key(event) if event.code == KeyCode::Char('y') => {
                dispatcher.dispatch(AppAction::Copy)?;
            }
            // Interact within current pane
            Event::Key(event) if event.code == KeyCode::Enter || event.code == KeyCode::Char('l') => {
                dispatcher.dispatch(AppAction::Interact)?;
            }
            // Toggle help
            Event::Key(event) if event.code == KeyCode::Char('/') => {
                dispatcher.dispatch(AppAction::ToggleHelpPane)?;
            }
            // Select a pane by its index
            Event::Key(KeyEvent {
                code: KeyCode::Char(code @ ('1' | '2' | '3' | '4')),
                ..
            }) => {
                let index = code
                    .to_digit(10)
                    .context("conversion to digit shouldn't fail, as we are sure about the contents")?
                    as usize;
                // Convert to a 0-based index
                dispatcher.dispatch(AppAction::SelectPane(index - 1))?;
            }
            // Ignore everything else
            evt => tracing::trace!("Ignoring an event: {:?}", evt),
        }
    }
}
