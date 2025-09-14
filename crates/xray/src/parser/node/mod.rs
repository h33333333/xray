mod filter;
mod inner_node;
mod iter;
mod util;

pub use filter::NodeFilters;
pub use inner_node::InnerNode;
use iter::TreeIter;
pub(super) use util::RestorablePath;

/// A single node in a file tree.
#[derive(Clone)]
pub struct Node {
    /// A 0-based index of the layer in which this node was last updated.
    ///
    /// NOTE: using indexes here assumes that the layers stay in the same order throughout the execution.
    pub updated_in: u8,
    /// Represents the actual file tree node (a file or a directory).
    pub inner: InnerNode,
}

impl Node {
    pub fn new(updated_in: u8) -> Self {
        Node {
            updated_in,
            inner: InnerNode::default(),
        }
    }

    pub fn new_with_inner(updated_in: u8, node: InnerNode) -> Self {
        Node {
            updated_in,
            inner: node,
        }
    }

    /// Inserts a new [InnerNode] at the provided path and updates the layer in which this node was last updated.
    pub fn insert(
        &mut self,
        path: &mut RestorablePath<'_>,
        new_node: InnerNode,
        layer_digest: u8,
    ) -> anyhow::Result<()> {
        self.updated_in = layer_digest;
        self.inner.insert(path, new_node, layer_digest)
    }

    /// Merges two [Nodes](Node).
    pub fn merge(mut self, other: Self) -> Self {
        self.updated_in = other.updated_in;
        self.inner = self.inner.merge(other.inner, other.updated_in);
        self
    }

    /// Applies the provided filter to this node.
    ///
    /// Returns true if there are any nodes left in the tree after filtering.
    pub fn filter(&mut self, mut filter: NodeFilters) -> bool {
        filter.strip_path_filter_prefix();
        self.inner.filter(filter)
    }

    /// Creates a new [iterator](TreeIter).
    pub fn iter(&self) -> TreeIter<'_> {
        TreeIter::new(self, false)
    }

    /// Creates a new [iterator](TreeIter) that also tracks active depth levels that are used when rendering the UI.
    pub fn iter_with_levels(&self) -> TreeIter<'_> {
        TreeIter::new(self, true)
    }

    /// Updates the index of a layer in which this node was last modified to the provided one recursively.
    pub(super) fn set_layer_recursively(&mut self, new_layer_idx: u8) {
        self.updated_in = new_layer_idx;
        // Update the child nodes (if any)
        if let Some(state) = self.inner.dir_state_mut() {
            state
                .children
                .values_mut()
                .for_each(|node| node.set_layer_recursively(new_layer_idx));
        }
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.iter_with_levels();
        loop {
            let Some((path, _, depth, _)) = iter.next() else {
                break;
            };
            for level in 0..depth {
                if iter.is_level_active(level).unwrap_or_default() {
                    write!(f, "│   ")?;
                } else {
                    write!(f, "    ")?;
                }
            }
            if iter.is_level_active(depth).unwrap_or_default() {
                writeln!(f, "├── {}", path.display())?;
            } else {
                writeln!(f, "└── {}", path.display())?;
            }
        }
        Ok(())
    }
}
