/// Side effects that can be returned by the [super::Pane]'s action handlers.
pub enum SideEffect {
    /// Reset the selected node and node statuses in the layer inspector pane.
    ResetLayerInspector,
}
