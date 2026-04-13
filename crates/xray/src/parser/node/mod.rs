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
        self.inner.filter(self.updated_in, filter)
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
        while let Some((path, _, depth, _)) = iter.next() {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::parser::{FileState, NodeStatus};

    // --- RestorablePath ---

    #[test]
    fn restorable_path_components_advance() {
        let path = Path::new("usr/local/bin");
        let rp = RestorablePath::new(path);
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("usr")
        );
        let rp = rp.advance();
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("local")
        );
        let rp = rp.advance();
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("bin")
        );
        let rp = rp.advance();
        assert!(rp.get_current_component().is_none());
    }

    #[test]
    fn restorable_path_restore_resets_to_start() {
        let path = Path::new("a/b/c");
        let rp = RestorablePath::new(path);
        let rp = rp.advance().advance();
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("c")
        );
        let rp = rp.restore();
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("a")
        );
    }

    #[test]
    fn restorable_path_relative_detection() {
        let relative = Path::new("usr/bin");
        let absolute = Path::new("/usr/bin");
        assert!(RestorablePath::new(relative).is_using_relative_path());
        assert!(!RestorablePath::new(absolute).is_using_relative_path());
    }

    #[test]
    fn restorable_path_strip_prefix_on_absolute() {
        let path = Path::new("/usr/bin");
        let mut rp = RestorablePath::new(path);
        rp.strip_prefix();
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("usr")
        );
    }

    #[test]
    fn restorable_path_strip_prefix_noop_on_relative() {
        let path = Path::new("usr/bin");
        let mut rp = RestorablePath::new(path);
        rp.strip_prefix();
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("usr")
        );
    }

    #[test]
    fn restorable_path_single_component() {
        let path = Path::new("file.txt");
        let rp = RestorablePath::new(path);
        assert_eq!(
            rp.get_current_component().unwrap(),
            Path::new("file.txt")
        );
        let rp = rp.advance();
        assert!(rp.get_current_component().is_none());
    }

    // --- InnerNode insert ---

    fn make_file_node(size: u64) -> InnerNode {
        InnerNode::File(FileState::new(NodeStatus::Added(size), None))
    }

    fn make_link_node(target: &str) -> InnerNode {
        InnerNode::File(FileState::new(
            NodeStatus::Added(0),
            Some(target.into()),
        ))
    }

    #[test]
    fn insert_single_file_at_root() {
        let mut root = Node::new(0);
        let file = make_file_node(100);
        root.insert(
            &mut RestorablePath::new(Path::new("hello.txt")),
            file,
            0,
        )
        .unwrap();

        let children = root.inner.children().unwrap();
        assert!(children.contains_key(Path::new("hello.txt")));
        assert_eq!(children[Path::new("hello.txt")].inner.size(), 100);
    }

    #[test]
    fn insert_nested_file_creates_intermediate_dirs() {
        let mut root = Node::new(0);
        let file = make_file_node(50);
        root.insert(
            &mut RestorablePath::new(Path::new("usr/local/bin/tool")),
            file,
            0,
        )
        .unwrap();

        let usr = &root.inner.children().unwrap()[Path::new("usr")];
        assert!(usr.inner.is_dir());
        let local = &usr.inner.children().unwrap()[Path::new("local")];
        assert!(local.inner.is_dir());
        let bin = &local.inner.children().unwrap()[Path::new("bin")];
        assert!(bin.inner.is_dir());
        let tool = &bin.inner.children().unwrap()[Path::new("tool")];
        assert_eq!(tool.inner.size(), 50);
    }

    #[test]
    fn insert_multiple_files_same_dir() {
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("bin/a")),
            make_file_node(10),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("bin/b")),
            make_file_node(20),
            0,
        )
        .unwrap();

        let bin = &root.inner.children().unwrap()[Path::new("bin")];
        let children = bin.inner.children().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[Path::new("a")].inner.size(), 10);
        assert_eq!(children[Path::new("b")].inner.size(), 20);
    }

    #[test]
    fn insert_replaces_file_with_dir_when_children_added() {
        let mut root = Node::new(0);
        // First insert a file at "lib"
        root.insert(
            &mut RestorablePath::new(Path::new("lib")),
            make_file_node(100),
            0,
        )
        .unwrap();
        assert!(!root.inner.children().unwrap()[Path::new("lib")].inner.is_dir());

        // Now insert a child under "lib" -- should convert it to a directory
        root.insert(
            &mut RestorablePath::new(Path::new("lib/foo.so")),
            make_file_node(200),
            1,
        )
        .unwrap();
        assert!(root.inner.children().unwrap()[Path::new("lib")].inner.is_dir());
    }

    // --- InnerNode merge ---

    #[test]
    fn merge_two_dirs_combines_children() {
        let mut left = Node::new(0);
        left.insert(
            &mut RestorablePath::new(Path::new("a")),
            make_file_node(10),
            0,
        )
        .unwrap();

        let mut right = Node::new(1);
        right
            .insert(
                &mut RestorablePath::new(Path::new("b")),
                make_file_node(20),
                1,
            )
            .unwrap();

        let merged = left.merge(right);
        let children = merged.inner.children().unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.contains_key(Path::new("a")));
        assert!(children.contains_key(Path::new("b")));
    }

    #[test]
    fn merge_same_file_becomes_modified() {
        let mut left = Node::new(0);
        left.insert(
            &mut RestorablePath::new(Path::new("f")),
            make_file_node(10),
            0,
        )
        .unwrap();

        let mut right = Node::new(1);
        right
            .insert(
                &mut RestorablePath::new(Path::new("f")),
                make_file_node(99),
                1,
            )
            .unwrap();

        let merged = left.merge(right);
        let f = &merged.inner.children().unwrap()[Path::new("f")];
        assert!(f.inner.is_modified());
        assert_eq!(f.inner.size(), 99);
    }

    #[test]
    fn merge_whiteout_deletes_directory() {
        let mut left = Node::new(0);
        left.insert(
            &mut RestorablePath::new(Path::new("dir/child")),
            make_file_node(10),
            0,
        )
        .unwrap();

        let mut right = Node::new(1);
        right
            .insert(
                &mut RestorablePath::new(Path::new("dir")),
                InnerNode::File(FileState::new(NodeStatus::Deleted, None)),
                1,
            )
            .unwrap();

        let merged = left.merge(right);
        let dir = &merged.inner.children().unwrap()[Path::new("dir")];
        assert!(dir.inner.is_deleted());
    }

    // --- InnerNode mark_as_deleted ---

    #[test]
    fn mark_as_deleted_recurses_through_children() {
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("a/b/c")),
            make_file_node(10),
            0,
        )
        .unwrap();

        root.inner.mark_as_deleted(2);

        // Walk down and verify everything is deleted
        let a = &root.inner.children().unwrap()[Path::new("a")];
        assert!(a.inner.is_deleted());
        assert_eq!(a.updated_in, 2);
        let b = &a.inner.children().unwrap()[Path::new("b")];
        assert!(b.inner.is_deleted());
        let c = &b.inner.children().unwrap()[Path::new("c")];
        assert!(c.inner.is_deleted());
    }

    // --- InnerNode misc ---

    #[test]
    fn get_n_of_child_nodes_counts_recursively() {
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("a")),
            make_file_node(1),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("b/c")),
            make_file_node(2),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("b/d")),
            make_file_node(3),
            0,
        )
        .unwrap();

        // root has children: a, b. b has children: c, d.
        // Total: a(1) + b(1) + c(1) + d(1) = 4
        assert_eq!(root.inner.get_n_of_child_nodes().unwrap(), 4);
    }

    #[test]
    fn link_node_reports_target() {
        let link = make_link_node("/usr/bin/real");
        assert_eq!(link.get_link().unwrap(), Path::new("/usr/bin/real"));
    }

    #[test]
    fn file_node_has_no_link() {
        let file = make_file_node(100);
        assert!(file.get_link().is_none());
    }

    // --- TreeIter ---

    #[test]
    fn iter_empty_tree() {
        let root = Node::new(0);
        let items: Vec<_> = root.iter().collect();
        assert!(items.is_empty());
    }

    #[test]
    fn iter_flat_dir() {
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("a")),
            make_file_node(1),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("b")),
            make_file_node(2),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("c")),
            make_file_node(3),
            0,
        )
        .unwrap();

        let paths: Vec<_> =
            root.iter().map(|(path, _, _, _)| path.to_owned()).collect();
        // BTreeMap ordering: a, b, c
        assert_eq!(paths, vec![
            Path::new("a").to_owned(),
            Path::new("b").to_owned(),
            Path::new("c").to_owned(),
        ]);
    }

    #[test]
    fn iter_nested_reports_depth() {
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("usr/bin/tool")),
            make_file_node(10),
            0,
        )
        .unwrap();

        let depths: Vec<_> =
            root.iter().map(|(_, _, depth, _)| depth).collect();
        // usr at depth 0, bin at depth 1, tool at depth 2
        assert_eq!(depths, vec![0, 1, 2]);
    }

    #[test]
    fn iter_with_levels_tracks_siblings() {
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("a")),
            make_file_node(1),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("b")),
            make_file_node(2),
            0,
        )
        .unwrap();

        let items: Vec<_> = root
            .iter_with_levels()
            .map(|(_, _, _, has_sibling)| has_sibling)
            .collect();
        // "a" has a sibling (b), "b" does not
        assert_eq!(items, vec![true, false]);
    }

    // --- set_layer_recursively ---

    #[test]
    fn set_layer_recursively_updates_all() {
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("x/y")),
            make_file_node(5),
            0,
        )
        .unwrap();

        root.set_layer_recursively(7);

        assert_eq!(root.updated_in, 7);
        let x = &root.inner.children().unwrap()[Path::new("x")];
        assert_eq!(x.updated_in, 7);
        let y = &x.inner.children().unwrap()[Path::new("y")];
        assert_eq!(y.updated_in, 7);
    }
}
