#[derive(Debug, Clone)]
pub enum AppAction {
    /// An empty action that doesn't lead to any change in the app's state.
    /// Can be used to re-render the frame without changing any state.
    Empty,
    /// Switch the active pane to the next one.
    TogglePane(Direction),
    /// Move in the specified [Direction] within the currently selected [super::view::Pane].
    Move(Direction),
    /// Copy the currently selected field into the system clipboard.
    ///
    /// What is copied is up to the currently active pane.
    Copy,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Direction {
    #[default]
    Forward,
    Backward,
}
