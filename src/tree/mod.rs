mod iter;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use iter::NodeIter;

pub type DirMap<F, D> = BTreeMap<PathBuf, Tree<F, D>>;

pub enum Tree<F = (), D = ()> {
    File(F),
    Directory((DirMap<F, D>, D)),
}

impl<F: std::fmt::Debug, D: std::fmt::Debug> std::fmt::Debug for Tree<F, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tree::File(state) => state.fmt(f),
            Tree::Directory((map, state)) => write!(f, "{:?}, {:?}", state, map),
        }
    }
}

impl<F: Default, D: Default> Tree<F, D> {
    pub fn new() -> Self {
        Tree::new_empty_dir(D::default())
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
                    map.insert(Path::new(component).into(), Tree::new_empty_dir(D::default()));
                }
                map.get_mut(Path::new(component))
                    .context("bug: this should be unreachable")?
            } else {
                anyhow::bail!("Didn't manage to insert a new node: some node in path is not a directory")
            }
        }

        if let Tree::Directory((map, _)) = node {
            map.insert(node_name.into(), new_node);
        }

        Ok(())
    }
}

impl<F, D> Tree<F, D> {
    pub fn new_empty_dir(dir_state: D) -> Self {
        Tree::Directory((DirMap::default(), dir_state))
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Tree::Directory(..))
    }

    pub fn children(&self) -> Option<&DirMap<F, D>> {
        match self {
            Tree::Directory((children, _)) => Some(children),
            _ => None,
        }
    }

    pub fn iter(&self) -> NodeIter<'_, F, D> {
        NodeIter::new(self)
    }
}
