use std::borrow::Cow;
use std::path::Path;

use anyhow::Context as _;

use super::{Node, NodeFilters};
use crate::parser::{DirMap, DirectoryState, FileState, NodeStatus};

#[derive(Clone)]
pub enum InnerNode {
    File(FileState),
    Directory(DirectoryState),
}

impl InnerNode {
    pub fn new_dir_with_size(size: u64) -> Self {
        InnerNode::Directory(DirectoryState::new_with_size(size))
    }

    pub fn new_empty_dir() -> Self {
        InnerNode::Directory(DirectoryState::new_empty())
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, InnerNode::Directory(..))
    }

    pub fn children(&self) -> Option<&DirMap> {
        match self {
            InnerNode::Directory(state) => Some(&state.children),
            _ => None,
        }
    }

    pub fn status(&self) -> NodeStatus {
        match self {
            InnerNode::File(state) => state.status,
            InnerNode::Directory(state) => state.status,
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
            n_of_children += child_node.inner.get_n_of_child_nodes().unwrap_or(0)
        }
        Some(n_of_children)
    }

    pub fn file_state(&self) -> Option<&FileState> {
        match self {
            InnerNode::File(state) => Some(state),
            _ => None,
        }
    }

    pub fn dir_state_mut(&mut self) -> Option<&mut DirectoryState> {
        match self {
            InnerNode::Directory(state) => Some(state),
            _ => None,
        }
    }

    pub fn get_link(&self) -> Option<&Path> {
        match self {
            InnerNode::File(state) => state.actual_file.as_deref(),
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

    pub(super) fn filter(&mut self, filter: NodeFilters) -> bool {
        if !filter.any() {
            // No filters -> entry is always included
            return true;
        }

        // We ignore files here, as they are handled when processing children of directories
        if let InnerNode::Directory(state) = self {
            state.children.retain(|path, child| {
                let is_dir_with_children = child
                    .inner
                    .children()
                    .map(|children| !children.is_empty())
                    .unwrap_or(false);

                // Size-based filtering
                if let Some(node_size_filter) = filter.node_size_filter {
                    if child.inner.size() < node_size_filter {
                        return false;
                    }
                }

                // Path-based filtering
                let path_filter_for_child = if let Some(path_filter) = filter.path_filter.as_ref() {
                    let is_filtered_out = if let Some(leftmost_part) = path_filter.get_current_component() {
                        path != Path::new(".")
                            && !path
                                .as_os_str()
                                .to_str()
                                // We need to convert both paths to a str to check for a partial match using `contains`
                                .and_then(|path| {
                                    leftmost_part.to_str().map(|leftmost_part| path.contains(leftmost_part))
                                })
                                // If anything fails here, exclude the node
                                .unwrap_or(true)
                    } else {
                        // If there is no longer a filter, then we simply retain the node
                        return true;
                    };

                    // Directories are filtered out based on their children when using relative path
                    if is_filtered_out && (!path_filter.is_using_relative_path() || !is_dir_with_children) {
                        // Exclude filtered out nodes (files, empty dirs, and mismatched dirs when using an absolute path)
                        return false;
                    }

                    let possibly_empty_filter = if path != Path::new(".") {
                        if is_filtered_out {
                            // This is only reachable when using relative paths, the current node is a dir, and it didn't pass the check.
                            // In this case, we reset the path filter back to its original state and check children of the current node.
                            path_filter.restore()
                        } else {
                            // In all other cases, we advance the current path filter component by 1
                            path_filter.advance()
                        }
                    } else {
                        // Pass the filter as is
                        path_filter.clone()
                    };

                    let filter = (!possibly_empty_filter
                        .get_current_component()
                        .map(|component| component.as_os_str().is_empty())
                        .unwrap_or(true))
                    .then_some(possibly_empty_filter);

                    if filter.is_none() && !is_filtered_out {
                        // This is only reachable if the current node matched the last component in the filter.
                        // In this case, we simply inclue the node and don't do any further child filtering (as their parent passed the check).
                        return true;
                    }

                    filter
                } else {
                    None
                };

                // Regex-based filtering
                if let Some(regex) = filter.path_regex.as_deref() {
                    let Some(path) = path.to_str() else {
                        // Exclude this node otherwise
                        return false;
                    };

                    if regex.is_match(path) {
                        // Include both directories and files if they satisfy the RegEx.
                        // We don't check children of directories in this case.
                        return true;
                    } else {
                        // If it's a dir with children, then we also check the children.
                        // Otherwise (if it's a file or an empty dir), we exclude the node immediately.
                        if !is_dir_with_children {
                            return false;
                        }
                    }
                }

                child.filter(NodeFilters {
                    path_filter: path_filter_for_child,
                    node_size_filter: filter.node_size_filter,
                    path_regex: filter.path_regex.as_deref().map(Cow::Borrowed),
                })
            });

            return !state.children.is_empty();
        }

        true
    }

    // TODO: rewrite this function to use recursion
    pub(super) fn insert(&mut self, path: impl AsRef<Path>, new_node: Self, layer_digest: u8) -> anyhow::Result<()> {
        let mut path_components = path.as_ref().iter();

        let Some(node_name) = path_components.next_back() else {
            // Replace the node
            *self = new_node;
            return Ok(());
        };

        let directory =
            path_components.try_fold::<_, _, Result<&mut InnerNode, anyhow::Error>>(self, |node, component| {
                let next_node = if let InnerNode::Directory(state) = node {
                    if !state.children.contains_key(Path::new(component)) {
                        state.children.insert(
                            Path::new(component).into(),
                            Node::new_with_node(layer_digest, InnerNode::Directory(DirectoryState::new_empty())),
                        );
                    }
                    let existing_node = &mut state
                        .children
                        .get_mut(Path::new(component))
                        .context("impossible: we just inserted the node above")?
                        .inner;

                    // Update the sizes
                    if let Some(state) = existing_node.dir_state_mut() {
                        match &mut state.status {
                            NodeStatus::Added(size) | NodeStatus::Modified(size) => *size += new_node.size(),
                            _ => (),
                        }
                    }

                    existing_node
                } else {
                    // NOTE: This happened in some images when I was testing the app.
                    // Some images change type of a node from directory to link back and forth before actually
                    // creating any children inside the directory. Thus, we may need to replace a node before
                    // appending other nodes to it.
                    let mut dir_state = DirectoryState::new_with_size(new_node.size());
                    dir_state.children.insert(
                        Path::new(component).into(),
                        Node::new_with_node(
                            layer_digest,
                            InnerNode::Directory(DirectoryState::new_with_size(new_node.size())),
                        ),
                    );
                    *node = InnerNode::Directory(dir_state);

                    &mut node
                        .dir_state_mut()
                        .context("impossible: we created a directory above")?
                        .children
                        .get_mut(Path::new(component))
                        .context("impossible: we just inserted the node above")?
                        .inner
                };
                Ok(next_node)
            })?;

        let state = if let InnerNode::Directory(state) = directory {
            state
        } else {
            // Ensure that the last node before the new node is a directory
            *directory = InnerNode::new_empty_dir();
            directory
                .dir_state_mut()
                .context("impossible: we created a directory above")?
        };

        state
            .children
            .insert(node_name.into(), Node::new_with_node(layer_digest, new_node));

        Ok(())
    }

    pub(super) fn merge(mut self, other: Self, digest: u8) -> Self {
        match (&mut self, other) {
            (InnerNode::Directory(left_state), InnerNode::Directory(right_state)) => {
                for (path, right_node) in right_state.children {
                    let updated_node = if let Some(left_node) = left_state.children.remove(&path) {
                        left_node.inner.merge(right_node.inner, digest)
                    } else {
                        right_node.inner
                    };
                    left_state
                        .children
                        .insert(path, Node::new_with_node(right_node.updated_in, updated_node));
                }
                let new_state = match (&left_state.status, &right_state.status) {
                    (_, NodeStatus::Added(_)) => {
                        // Calculate the updated directory size after merging
                        NodeStatus::Modified(left_state.children.values().map(|tree| tree.inner.size()).sum())
                    }
                    (_, _) => right_state.status,
                };
                left_state.status = new_state;
            }
            (InnerNode::File(left_state), InnerNode::File(right_state)) => {
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

    pub(super) fn mark_as_deleted(&mut self, digest: u8) {
        match self {
            InnerNode::Directory(state) => {
                // Mark the directory itself as deleted
                state.status = NodeStatus::Deleted;
                // Mark each children as deleted recursively
                for tree in state.children.values_mut() {
                    tree.updated_in = digest;
                    tree.inner.mark_as_deleted(digest);
                }
            }
            InnerNode::File(state) => {
                state.status = NodeStatus::Deleted;
            }
        }
    }
}

impl Default for InnerNode {
    fn default() -> Self {
        InnerNode::new_empty_dir()
    }
}
