mod iter;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use iter::TreeIter;

use super::{DirectoryState, FileState, NodeStatus, Sha256Digest};

pub type DirMap = BTreeMap<PathBuf, Tree>;

#[derive(Clone)]
pub struct Tree {
    pub updated_in: Sha256Digest,
    pub node: Node,
}

impl Tree {
    pub fn new(updated_in: Sha256Digest) -> Self {
        Tree {
            updated_in,
            node: Node::default(),
        }
    }

    pub fn new_with_node(updated_in: Sha256Digest, node: Node) -> Self {
        Tree { updated_in, node }
    }

    pub fn insert(&mut self, path: impl AsRef<Path>, new_node: Node, layer_digest: Sha256Digest) -> anyhow::Result<()> {
        self.updated_in = layer_digest;
        self.node.insert(path, new_node, layer_digest)
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.updated_in = other.updated_in;
        self.node = self.node.merge(other.node, &other.updated_in);
        self
    }

    pub fn iter(&self) -> TreeIter<'_> {
        TreeIter::new(self, false)
    }

    pub fn iter_with_levels(&self) -> TreeIter<'_> {
        TreeIter::new(self, true)
    }
}

impl std::fmt::Debug for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.iter_with_levels();
        loop {
            let Some((path, _, depth, _)) = iter.next() else { break };
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

#[derive(Clone)]
pub enum Node {
    File(FileState),
    Directory(DirectoryState),
}

impl Node {
    pub fn new_dir_with_size(size: u64) -> Self {
        Node::Directory(DirectoryState::new_with_size(size))
    }

    pub fn new_empty_dir() -> Self {
        Node::Directory(DirectoryState::new_empty())
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Node::Directory(..))
    }

    pub fn children(&self) -> Option<&DirMap> {
        match self {
            Node::Directory(state) => Some(&state.children),
            _ => None,
        }
    }

    pub fn status(&self) -> NodeStatus {
        match self {
            Node::File(state) => state.status,
            Node::Directory(state) => state.status,
        }
    }

    pub fn size(&self) -> u64 {
        match self.status() {
            NodeStatus::Added(size) | NodeStatus::Modified(size) => size,
            _ => 0,
        }
    }

    pub fn get_n_of_child_nodes(&self) -> Option<usize> {
        let children = self.children()?;
        let mut n_of_children = children.len();
        for (_, child_node) in children.iter() {
            n_of_children += child_node.node.get_n_of_child_nodes().unwrap_or(0)
        }
        Some(n_of_children)
    }

    pub fn file_state(&self) -> Option<&FileState> {
        match self {
            Node::File(state) => Some(state),
            _ => None,
        }
    }

    pub fn dir_state_mut(&mut self) -> Option<&mut DirectoryState> {
        match self {
            Node::Directory(state) => Some(state),
            _ => None,
        }
    }

    pub fn get_link(&self) -> Option<&Path> {
        match self {
            Node::File(state) => state.actual_file.as_deref(),
            _ => None,
        }
    }

    pub fn is_added(&self) -> bool {
        matches!(self.status(), NodeStatus::Added(_))
    }

    pub fn is_modified(&self) -> bool {
        matches!(self.status(), NodeStatus::Modified(_))
    }

    pub fn is_deleted(&self) -> bool {
        matches!(self.status(), NodeStatus::Deleted)
    }

    // TODO: rewrite this function to use recursion
    fn insert(&mut self, path: impl AsRef<Path>, new_node: Self, layer_digest: Sha256Digest) -> anyhow::Result<()> {
        let mut path_components = path.as_ref().iter();

        let Some(node_name) = path_components.next_back() else {
            // Replace the node
            *self = new_node;
            return Ok(());
        };

        let directory =
            path_components.try_fold::<_, _, Result<&mut Node, anyhow::Error>>(self, |node, component| {
                let next_node = if let Node::Directory(state) = node {
                    if !state.children.contains_key(Path::new(component)) {
                        state.children.insert(
                            Path::new(component).into(),
                            Tree::new_with_node(
                                layer_digest,
                                Node::Directory(DirectoryState::new_with_size(new_node.size())),
                            ),
                        );
                    }
                    &mut state
                        .children
                        .get_mut(Path::new(component))
                        .context("impossible: we just inserted the node above")?
                        .node
                } else {
                    // NOTE: This happened in some images when I was testing the app.
                    // Some images change type of a node from directory to link back and forth before actually
                    // creating any children inside the directory. Thus, we may need to replace a node before
                    // appending other nodes to it.
                    let mut dir_state = DirectoryState::new_with_size(new_node.size());
                    dir_state.children.insert(
                        Path::new(component).into(),
                        Tree::new_with_node(
                            layer_digest,
                            Node::Directory(DirectoryState::new_with_size(new_node.size())),
                        ),
                    );
                    *node = Node::Directory(dir_state);

                    &mut node
                        .dir_state_mut()
                        .context("impossible: we created a directory above")?
                        .children
                        .get_mut(Path::new(component))
                        .context("impossible: we just inserted the node above")?
                        .node
                };
                Ok(next_node)
            })?;

        let state = if let Node::Directory(state) = directory {
            state
        } else {
            // Ensure that the last node before the new node is a directory
            *directory = Node::new_empty_dir();
            directory
                .dir_state_mut()
                .context("impossible: we created a directory above")?
        };

        // Update the directory size
        match &mut state.status {
            NodeStatus::Added(size) | NodeStatus::Modified(size) => *size += new_node.size(),
            _ => (),
        }

        state
            .children
            .insert(node_name.into(), Tree::new_with_node(layer_digest, new_node));

        Ok(())
    }

    fn merge(mut self, other: Self, digest: &Sha256Digest) -> Self {
        match (&mut self, other) {
            (Node::Directory(left_state), Node::Directory(right_state)) => {
                for (path, right_node) in right_state.children {
                    let updated_node = if let Some(left_node) = left_state.children.remove(&path) {
                        left_node.node.merge(right_node.node, digest)
                    } else {
                        right_node.node
                    };
                    left_state
                        .children
                        .insert(path, Tree::new_with_node(right_node.updated_in, updated_node));
                }
                let new_state = match (&left_state.status, &right_state.status) {
                    (_, NodeStatus::Added(_)) => {
                        // Calculate the updated directory size after merging
                        NodeStatus::Modified(left_state.children.values().map(|tree| tree.node.size()).sum())
                    }
                    (_, _) => right_state.status,
                };
                left_state.status = new_state;
            }
            (Node::File(left_state), Node::File(right_state)) => {
                let new_state = match (&left_state.status, &right_state.status) {
                    (_, NodeStatus::Added(new_size)) => NodeStatus::Modified(*new_size),
                    (_, _) => right_state.status,
                };
                left_state.status = new_state;
            }
            (left_node, right_node) => {
                // Check if a directory was deleted using a whiteout file
                if left_node.is_dir() && right_node.is_deleted() {
                    left_node.mark_as_deleted(digest);
                } else {
                    // Can only happen if type of a node has changed.
                    // If this happens, then we simply want to replace the node altogether.
                    *left_node = right_node
                }
            }
        }
        self
    }

    fn mark_as_deleted(&mut self, digest: &Sha256Digest) {
        match self {
            Node::Directory(state) => {
                // Mark the directory itself as deleted
                state.status = NodeStatus::Deleted;
                // Mark each children as deleted recursively
                for tree in state.children.values_mut() {
                    tree.updated_in = *digest;
                    tree.node.mark_as_deleted(digest);
                }
            }
            Node::File(state) => {
                state.status = NodeStatus::Deleted;
            }
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::new_empty_dir()
    }
}
