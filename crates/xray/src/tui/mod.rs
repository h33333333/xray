use action::Direction;
use anyhow::Context;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::size;
use crossterm_keybind::KeyBindTrait as _;
use dispatcher::Dispatcher;
use store::AppState;
use view::App;

mod action;
pub use action::AppAction;

use crate::keybindings::KeyAction;
use crate::parser::Image;
mod dispatcher;
mod store;
mod util;
mod view;

pub type AppDispatcher = Dispatcher<AppState, App>;

impl AppDispatcher {
    /// Creates a new [AppDispatcher] from a parsed [Image].
    pub fn init(image: Image) -> anyhow::Result<Self> {
        let store = AppState::new(image)
            .context("failed to initialize the app state")?;
        let view = App::new();
        Ok(Dispatcher::new(store, view))
    }

    /// Consumes this [AppDispatcher] and starts listening for events until an explicit cancellation is requested from the user.
    pub fn run_until_stopped(mut self) -> anyhow::Result<()> {
        let size = size().context("failed to get the terminal's size")?;
        // Do the initial render of the interface
        self.dispatch(AppAction::Empty(size))?;

        'outer: loop {
            let event = event::read()?;

            // Ignore all key events on Windows besides presses to prevent duplicate events
            if cfg!(windows)
                && !matches!(
                    event,
                    Event::Key(KeyEvent {
                        kind: KeyEventKind::Press,
                        ..
                    })
                )
            {
                continue;
            }

            match event {
                // Re-render the interface when terminal window is resized.
                Event::Resize(h, v) => {
                    self.dispatch(AppAction::Empty((h, v)))?
                }
                // Keyboard-related events.
                Event::Key(event) => {
                    // Handle exit keybind before everything else.
                    if KeyAction::Exit.match_any(&event) {
                        break Ok(());
                    }

                    // This block handles insert mode, as it requires handling free text input.
                    if self.get_store().is_in_insert_mode {
                        // Close the popup.
                        if event.code == KeyCode::Enter
                            || event.code == KeyCode::Esc
                            || KeyAction::ToggleFilterPopup.match_any(&event)
                        {
                            self.dispatch(AppAction::ToggleInputMode)?;
                            continue;
                        }

                        // Delete a character.
                        if event.code == KeyCode::Backspace
                            || event.code == KeyCode::Delete
                        {
                            self.dispatch(AppAction::InputDeleteCharacter)?;
                            continue;
                        }

                        if KeyAction::FilterSubaction.match_any(&event) {
                            self.dispatch(AppAction::Interact)?;
                            continue;
                        }

                        if KeyAction::NextItem.match_any(&event) {
                            self.dispatch(AppAction::Move(Direction::Forward))?;
                            continue;
                        }
                        if KeyAction::PreviousItem.match_any(&event) {
                            self.dispatch(AppAction::Move(
                                Direction::Backward,
                            ))?;
                            continue;
                        }

                        let KeyCode::Char(mut input) = event.code else {
                            continue;
                        };

                        if event.modifiers.intersects(KeyModifiers::SHIFT) {
                            input = input.to_ascii_uppercase()
                        }
                        self.dispatch(AppAction::InputCharacter(input))?;
                        continue;
                    }

                    // This match handles unconfigurable keybinds.
                    match event {
                        // Select a pane by its index
                        KeyEvent {
                            code: KeyCode::Char(code @ ('1' | '2' | '3' | '4')),
                            ..
                        } => {
                            let index = code
                        .to_digit(10)
                        .context("conversion to digit shouldn't fail, as we are sure about the contents")?
                        as usize;
                            // Convert to a 0-based index.
                            self.dispatch(AppAction::SelectPane(index - 1))?;
                            continue;
                        }
                        _ => (),
                    };

                    // This handles all configurable keybinds.
                    for action in KeyAction::dispatch(&event) {
                        match action {
                            // Close help pane if it's active.
                            KeyAction::CloseActiveWindow
                                if self.get_store().show_help_popup =>
                            {
                                self.dispatch(AppAction::ToggleHelpPane)?;
                            }
                            KeyAction::CloseActiveWindow => {
                                break 'outer Ok(());
                            }
                            KeyAction::ToggleHelp => {
                                self.dispatch(AppAction::ToggleHelpPane)?;
                            }
                            KeyAction::ToggleFilterPopup => {
                                self.dispatch(AppAction::ToggleInputMode)?;
                            }
                            KeyAction::PreviousItem => {
                                self.dispatch(AppAction::TogglePane(
                                    Direction::Backward,
                                ))?;
                            }
                            KeyAction::NextItem => {
                                self.dispatch(AppAction::TogglePane(
                                    Direction::Forward,
                                ))?;
                            }
                            KeyAction::Interact => {
                                self.dispatch(AppAction::Interact)?;
                            }
                            KeyAction::Copy => {
                                self.dispatch(AppAction::Copy)?;
                            }
                            KeyAction::Subaction => {
                                self.dispatch(AppAction::Subaction)?;
                            }
                            KeyAction::Backward => {
                                self.dispatch(AppAction::Scroll(
                                    Direction::Backward,
                                ))?;
                            }
                            KeyAction::Down => {
                                self.dispatch(AppAction::Move(
                                    Direction::Forward,
                                ))?;
                            }
                            KeyAction::Up => {
                                self.dispatch(AppAction::Move(
                                    Direction::Backward,
                                ))?;
                            }
                            KeyAction::Forward => {
                                self.dispatch(AppAction::Scroll(
                                    Direction::Forward,
                                ))?;
                            }
                            // Everything else was handled before already.
                            _ => (),
                        }
                    }
                }
                // Ignore everything else.
                evt => tracing::trace!("Ignoring an event: {:?}", evt),
            }
        }
    }
}
