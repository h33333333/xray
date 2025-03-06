mod iter;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use iter::NodeIter;

use super::FileState;

pub type DirMap = BTreeMap<PathBuf, Tree>;

#[derive(Clone)]
pub enum Tree {
    File(FileState),
    Directory((DirMap, bool)),
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

impl Tree {
    pub fn new() -> Self {
        Tree::default()
    }

    pub fn insert(&mut self, path: impl AsRef<Path>, new_node: Self) -> anyhow::Result<()> {
        let mut path_components = path.as_ref().iter();

        let Some(node_name) = path_components.next_back() else {
            // Replace the node
            *self = new_node;
            return Ok(());
        };

        let mut node = self;
        for component in path_components {
            node = if let Tree::Directory((map, _)) = node {
                if !map.contains_key(Path::new(component)) {
                    map.insert(Path::new(component).into(), Tree::new_empty_dir());
                }
                let next_node = map
                    .get_mut(Path::new(component))
                    .context("bug: this should be unreachable")?;
                if !matches!(next_node, Tree::Directory(_)) {
                    // Protect against cases where the final component is a hard link
                    *next_node = Tree::new_empty_dir();
                }
                next_node
            } else {
                // HACK: this can happen when dealing with hard links.
                // We can just override a standard file entry with a directory and proceed as usual.
                *node = Tree::new_empty_dir();
                let Tree::Directory((map, _)) = node else {
                    anyhow::bail!("Should be unreachable");
                };
                map.insert(Path::new(component).into(), Tree::new_empty_dir());
                map.get_mut(Path::new(component))
                    .context("bug: this should be unreachable")?
            }
        }

        let Tree::Directory((map, _)) = node else {
            anyhow::bail!("final component before the file is not a directoy: {:?}", path.as_ref())
        };
        map.insert(node_name.into(), new_node);

        Ok(())
    }

    pub fn merge(mut self, other: Self) -> Self {
        match (&mut self, other) {
            (Tree::Directory((left_children, left_state)), Tree::Directory((right_children, right_state))) => {
                for (path, right_node) in right_children {
                    let updated_node = if let Some(left_node) = left_children.remove(&path) {
                        left_node.merge(right_node)
                    } else {
                        right_node
                    };
                    left_children.insert(path, updated_node);
                }
                *left_state = right_state;
            }
            (Tree::File(left_state), Tree::File(right_state)) => *left_state = right_state,
            (left_node, right_node) => {
                // Can only happen if type of a node has changed.
                // If this happens, then we simply want to replace the node altogether.
                *left_node = right_node
            }
        }
        self
    }
}

impl Tree {
    pub fn new_empty_dir() -> Self {
        Tree::Directory((DirMap::default(), false))
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Tree::Directory(..))
    }

    pub fn children(&self) -> Option<&DirMap> {
        match self {
            Tree::Directory((children, _)) => Some(children),
            _ => None,
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            Tree::File(FileState::Exists(size)) => *size,
            _ => 0,
        }
    }

    pub fn iter(&self) -> NodeIter<'_> {
        NodeIter::new(self, false)
    }

    pub fn iter_with_levels(&self) -> NodeIter<'_> {
        NodeIter::new(self, true)
    }

    pub fn get_n_of_child_nodes(&self) -> Option<usize> {
        let children = self.children()?;
        let mut n_of_children = children.len();
        for (_, child_node) in children.iter() {
            n_of_children += child_node.get_n_of_child_nodes().unwrap_or(0)
        }
        Some(n_of_children)
    }

    pub fn file_state(&self) -> Option<&FileState> {
        match self {
            Tree::File(state) => Some(state),
            _ => None,
        }
    }
}

impl Default for Tree {
    fn default() -> Self {
        Tree::new_empty_dir()
    }
}
