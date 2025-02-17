use std::collections::HashSet;

/// [super::Pane::LayerInspector]'s pane state.
#[derive(Debug, Default)]
pub struct LayerInspectorPane {
    /// Index of the currently selected node in the tree.
    pub current_node_idx: usize,
    /// Contains indexes of all nodes that are collapsed.
    pub collapsed_nodes: HashSet<usize>,
}

impl LayerInspectorPane {
    /// Resets collapsed states and the current node index.
    pub fn reset(&mut self) {
        // TODO: make iter not expand collapsed directories
        // TODO: somehow show that a directory is collapsed when rendering
        // TODO: make iter support dynamic collapsing (like when user wants to collapse/expand all directories and we don't know their indexes)
        // TODO: track in which layer an entry was last modified
        // TODO: allow iter do path-based filtering
        self.current_node_idx = 0;
        self.collapsed_nodes.clear();
    }
}
