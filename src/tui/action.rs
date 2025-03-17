#[derive(Debug, Clone)]
pub enum AppAction {
    /// An empty action that doesn't lead to any change in the app's state.
    /// Can be used to re-render the frame without changing any state.
    Empty,
    /// Switch the active pane to the next one.
    TogglePane(Direction),
    /// Move in the specified [Direction] within the currently selected [super::view::Pane].
    Move(Direction),
    /// Interact with the currently selected element within the currently selected [super::view::Pane].
    Interact,
    /// Copy the currently selected field into the system clipboard.
    ///
    /// What is copied is up to the currently active pane.
    Copy,
    /// Show/hide the help pane.
    ToggleHelpPane,
    /// Select a specific pane by its index in the layout.
    SelectPane(usize),
}

#[derive(Debug, Default, Clone, Copy)]
/// Represents a direction in which the user wants to [AppAction::Move].
pub enum Direction {
    #[default]
    /// Move to the next entry in a pane.
    Forward,
    /// Move to the previous entry in a pane.
    Backward,
}
