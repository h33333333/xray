/// Side effects that can be returned by the [super::Pane]'s action handlers.
pub enum SideEffect {
    /// The current changeset was updated.
    ChangesetUpdated,
    /// The node filters were updated.
    FiltersUpdated,
}
