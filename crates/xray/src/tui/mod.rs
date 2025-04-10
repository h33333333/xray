use action::Direction;
use anyhow::Context;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::size;
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
    let size = size().context("failed to get the terminal's size")?;
    // Do the initial render of the interface
    dispatcher.dispatch(AppAction::Empty(size))?;

    loop {
        let event = event::read()?;
        let store = dispatcher.store();

        match event {
            // Re-render the interface when terminal window is resized
            Event::Resize(h, v) => dispatcher.dispatch(AppAction::Empty((h, v)))?,
            // If we are in the insert mode, we ignore all hotkeys except 'Enter' and 'CTRL-C'
            Event::Key(event) if store.is_in_insert_mode => {
                if event.code == KeyCode::Enter
                    || event.code == KeyCode::Esc
                    || (event.code == KeyCode::Char('c') && event.modifiers.intersects(KeyModifiers::CONTROL))
                {
                    dispatcher.dispatch(AppAction::ToggleInputMode)?;
                    continue;
                }

                if event.code == KeyCode::Backspace || event.code == KeyCode::Delete {
                    dispatcher.dispatch(AppAction::InputDeleteCharacter)?;
                    continue;
                }

                if event.code == KeyCode::Char('l') && event.modifiers.intersects(KeyModifiers::CONTROL) {
                    dispatcher.dispatch(AppAction::Interact)?;
                    continue;
                }

                if event.code == KeyCode::Tab || event.code == KeyCode::BackTab {
                    let direction = if event.code == KeyCode::Tab {
                        Direction::Forward
                    } else {
                        Direction::Backward
                    };
                    dispatcher.dispatch(AppAction::Move(direction))?;
                    continue;
                }

                if let KeyCode::Char(input) = event.code {
                    let input = if event.modifiers.intersects(KeyModifiers::SHIFT) {
                        input.to_ascii_uppercase()
                    } else {
                        input
                    };
                    dispatcher.dispatch(AppAction::InputCharacter(input))?;
                }
            }
            // Quit
            Event::Key(event)
                if event.code == KeyCode::Char('q')
                    || (event.code == KeyCode::Char('c') && event.modifiers.intersects(KeyModifiers::CONTROL)) =>
            {
                break Ok(());
            }
            // Select next pane
            Event::Key(event) if event.code == KeyCode::Tab => {
                dispatcher.dispatch(AppAction::TogglePane(Direction::Forward))?;
            }
            // Select previous pane
            Event::Key(event) if event.code == KeyCode::BackTab => {
                dispatcher.dispatch(AppAction::TogglePane(Direction::Backward))?;
            }
            // Scroll left
            Event::Key(event) if event.code == KeyCode::Char('h') || event.code == KeyCode::Left => {
                dispatcher.dispatch(AppAction::Scroll(Direction::Backward))?;
            }
            // Move down
            Event::Key(event) if event.code == KeyCode::Char('j') || event.code == KeyCode::Down => {
                dispatcher.dispatch(AppAction::Move(Direction::Forward))?;
            }
            // Move up
            Event::Key(event) if event.code == KeyCode::Char('k') || event.code == KeyCode::Up => {
                dispatcher.dispatch(AppAction::Move(Direction::Backward))?;
            }
            // Scroll right
            Event::Key(event) if event.code == KeyCode::Char('l') || event.code == KeyCode::Right => {
                dispatcher.dispatch(AppAction::Scroll(Direction::Forward))?;
            }
            // Interact within the current pane.
            Event::Key(event) if event.code == KeyCode::Enter || event.code == KeyCode::Char(' ') => {
                dispatcher.dispatch(AppAction::Interact)?;
            }

            // Copy the selected item to clipboard
            Event::Key(event) if event.code == KeyCode::Char('y') => {
                dispatcher.dispatch(AppAction::Copy)?;
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
            // Toggle path filter input
            Event::Key(event)
                if event.code == KeyCode::Char('f') && event.modifiers.intersects(KeyModifiers::CONTROL) =>
            {
                dispatcher.dispatch(AppAction::ToggleInputMode)?;
            }
            // Ignore everything else
            evt => tracing::trace!("Ignoring an event: {:?}", evt),
        }
    }
}
