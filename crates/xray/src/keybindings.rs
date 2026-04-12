use std::path::Path;

use anyhow::Context;
use crossterm_keybind::{KeyBind, KeyBindTrait};

const KEYBINDS_FILE_NAME: &str = "keybinds.toml";

/// All configurable keybindings.
#[derive(KeyBind)]
pub enum KeyAction {
    /// <General>
    ///
    /// Format reference: https://github.com/yanganto/crossterm-keybind.

    /// Exit the application.
    // NOTE: crossterm reports Shift+q as "Shift+Q" on MacOS.
    #[keybindings["Control+c", "Q", "Shift+Q"]]
    Exit,
    /// Close active window (i.e. the help popup).
    ///
    /// Can also **close the application** if there are no active windows.
    #[keybindings["q"]]
    CloseActiveWindow,
    /// Toggle the help popup.
    #[keybindings["/"]]
    ToggleHelp,
    /// Toggle the filtering popup.
    #[keybindings["Control+f"]]
    ToggleFilterPopup,
    /// Do a subaction for the currently active filter in the filtering popup. This is currently used for:
    ///
    ///     1. Toggling between path and RegEx-based filtering.
    ///     2. Changing size units.
    #[keybindings["Control+l"]]
    FilterSubaction,

    /// <Context-dependent movements>

    /// Select previous item. This action is currently used for:
    ///
    ///     1. Cycling through panes.
    ///     2. Cycling through available filter types.
    // FIXME: Shift+BackTab was required to make it work on MacOS.
    #[keybindings["BackTab", "Shift+BackTab"]]
    PreviousItem,
    /// Select next item. This action is currently used for:
    ///
    ///     1. Cycling through panes.
    ///     2. Cycling through available filter types.
    #[keybindings["Tab"]]
    NextItem,
    /// Do an action within the currently active pane. This action is currently used for:
    ///
    ///     1. Toggling the currently selected node in the Layer Inspector pane.
    #[keybindings["Enter", " "]]
    Interact,
    /// Copies the currently selected item to the system clipboard if the clipboard is available
    /// and copying is supported in the current context.
    #[keybindings["y"]]
    Copy,
    /// Do the additional action within the currently active context. This action is currently used for:
    ///
    ///     1. Toggling the "show only changed files" filter within the Layer Inspector pane.
    #[keybindings["c"]]
    Subaction,

    /// <Movement>

    /// Move backwards within the currently active context.
    #[keybindings["h", "Left"]]
    Backward,
    /// Move downwards within the currently active context.
    #[keybindings["j", "Down"]]
    Down,
    /// Move upwards within the currently active context.
    #[keybindings["k", "Up"]]
    Up,
    /// Move onwards within the currently active context.
    #[keybindings["l", "Right"]]
    Forward,
}

pub fn init_keybindings(config_dir: &Path) -> anyhow::Result<()> {
    let mut path = config_dir.to_path_buf();
    path.push(KEYBINDS_FILE_NAME);

    if !std::fs::exists(&path)
        .context("failed to check existence of the keybindings file")?
    {
        // Create an example file first if it's missing.
        KeyAction::to_toml_example(&path)
            .context("failed to export an example config file")?;
    }

    KeyAction::init_and_load_file(Some(path))
        .context("failed to initialize keybindings from the config file")
}
