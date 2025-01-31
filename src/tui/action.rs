#[derive(Debug, Clone)]
pub enum AppAction {
    /// An empty action that doesn't lead to any change in the app's state.
    /// Can be used to re-render the frame without changing any state.
    Empty,
}
