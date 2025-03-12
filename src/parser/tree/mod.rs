mod iter;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use iter::TreeIter;

use super::{DirectoryState, FileState, Sha256Digest};

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
    Directory((DirMap, DirectoryState)),
}

impl Node {
    pub fn new_empty_dir() -> Self {
        Node::Directory((DirMap::default(), DirectoryState::Added))
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Node::Directory(..))
    }

    pub fn children(&self) -> Option<&DirMap> {
        match self {
            Node::Directory((children, _)) => Some(children),
            _ => None,
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            Node::File(FileState::Added(size) | FileState::Modified(size)) => *size,
            // FIXME: this is bad, as I will recalculate size of each directory on every re-render
            Node::Directory((children, _)) if !self.is_deleted() => {
                children.values().map(|tree| tree.node.size()).sum()
            }
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

    pub fn is_added(&self) -> bool {
        match self {
            Node::File(state) => matches!(state, FileState::Added(_)),
            Node::Directory((_, state)) => matches!(state, DirectoryState::Added),
        }
    }

    pub fn is_modified(&self) -> bool {
        match self {
            Node::File(state) => matches!(state, FileState::Modified(_)),
            Node::Directory((_, state)) => matches!(state, DirectoryState::Modified),
        }
    }

    pub fn is_deleted(&self) -> bool {
        match self {
            Node::File(state) => matches!(state, FileState::Deleted),
            Node::Directory((_, state)) => matches!(state, DirectoryState::Deleted),
        }
    }

    fn insert(&mut self, path: impl AsRef<Path>, new_node: Self, layer_digest: Sha256Digest) -> anyhow::Result<()> {
        let mut path_components = path.as_ref().iter();

        let Some(node_name) = path_components.next_back() else {
            // Replace the node
            *self = new_node;
            return Ok(());
        };

        let mut node = self;
        for component in path_components {
            node = if let Node::Directory((map, _)) = node {
                if !map.contains_key(Path::new(component)) {
                    map.insert(Path::new(component).into(), Tree::new(layer_digest));
                }
                let next_node = &mut map
                    .get_mut(Path::new(component))
                    .context("bug: this should be unreachable")?
                    .node;
                if !matches!(next_node, Node::Directory(_)) {
                    // Protect against cases where the final component is a hard link
                    *next_node = Node::new_empty_dir();
                }
                next_node
            } else {
                // HACK: this can happen when dealing with hard links.
                // We can just override a standard file entry with a directory and proceed as usual.
                *node = Node::new_empty_dir();
                let Node::Directory((map, _)) = node else {
                    anyhow::bail!("Should be unreachable");
                };
                map.insert(Path::new(component).into(), Tree::new(layer_digest));

                &mut map
                    .get_mut(Path::new(component))
                    .context("bug: this should be unreachable")?
                    .node
            }
        }

        let Node::Directory((map, _)) = node else {
            anyhow::bail!("final component before the file is not a directoy: {:?}", path.as_ref())
        };
        map.insert(node_name.into(), Tree::new_with_node(layer_digest, new_node));

        Ok(())
    }

    fn merge(mut self, other: Self, digest: &Sha256Digest) -> Self {
        match (&mut self, other) {
            (Node::Directory((left_children, left_state)), Node::Directory((right_children, right_state))) => {
                for (path, right_node) in right_children {
                    let updated_node = if let Some(left_node) = left_children.remove(&path) {
                        left_node.node.merge(right_node.node, digest)
                    } else {
                        right_node.node
                    };
                    left_children.insert(path, Tree::new_with_node(right_node.updated_in, updated_node));
                }
                let new_state = match (&left_state, &right_state) {
                    (_, DirectoryState::Added) => DirectoryState::Modified,
                    (_, _) => right_state,
                };
                *left_state = new_state;
            }
            (Node::File(left_state), Node::File(right_state)) => {
                let new_state = match (&left_state, &right_state) {
                    (_, FileState::Added(new_size)) => FileState::Modified(*new_size),
                    (_, _) => right_state,
                };
                *left_state = new_state;
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
            Node::Directory((children, state)) => {
                // Mark the directory itself as deleted
                *state = DirectoryState::Deleted;
                // Mark each children as deleted recursively
                tracing::debug!(?children, "test");
                for tree in children.values_mut() {
                    tree.updated_in = *digest;
                    tree.node.mark_as_deleted(digest);
                }
            }
            Node::File(state) => {
                *state = FileState::Deleted;
            }
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::new_empty_dir()
    }
}
