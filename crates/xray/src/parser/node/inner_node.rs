use std::borrow::Cow;
use std::path::Path;

use anyhow::Context;

use super::{Node, NodeFilters, RestorablePath};
use crate::parser::{DirMap, DirectoryState, FileState, NodeStatus};

/// Represents the actual state of a file tree [nodes](super::Node).
#[derive(Clone)]
pub enum InnerNode {
    /// A file or a link.
    File(FileState),
    /// A directory with zero or more [nodes](super::Node) inside.
    Directory(DirectoryState),
}

impl InnerNode {
    /// Creates a new empty node of type [InnerNode::Directory] with status [NodeStatus::Added].
    pub fn new_empty_dir() -> Self {
        InnerNode::Directory(DirectoryState::new_empty())
    }

    /// Creates a new node of type [InnerNode::Directory] with status [NodeStatus::Added] using the provided size.
    pub fn new_dir_with_size(size: u64) -> Self {
        InnerNode::Directory(DirectoryState::new_with_size(size))
    }

    /// Returns `true` if this node's [status](NodeStatus) is [NodeStatus::Added].
    pub fn is_added(&self) -> bool {
        matches!(self.status(), NodeStatus::Added(_))
    }

    /// Returns `true` if this node's [status](NodeStatus) is [NodeStatus::Modified].
    pub fn is_modified(&self) -> bool {
        matches!(self.status(), NodeStatus::Modified(_))
    }

    /// Returns `true` if this node's [status](NodeStatus) is [NodeStatus::Deleted].
    pub fn is_deleted(&self) -> bool {
        matches!(self.status(), NodeStatus::Deleted)
    }

    /// Returns the size of a node unless it's status is [NodeStatus::Deleted], in which case
    /// it returns `0`.
    pub fn size(&self) -> u64 {
        match self.status() {
            NodeStatus::Added(size) | NodeStatus::Modified(size) => size,
            _ => 0,
        }
    }

    /// Returns `true` if this node is [InnerNode::Directory] and `false` otherwise.
    pub fn is_dir(&self) -> bool {
        matches!(self, InnerNode::Directory(..))
    }

    /// Returns a [DirMap] of children for this [InnerNode::Directory] or [Option::None] if the node is a [InnerNode::File].
    pub fn children(&self) -> Option<&DirMap> {
        match self {
            InnerNode::Directory(state) => Some(&state.children),
            _ => None,
        }
    }

    /// Returns the total number of children nodes for this [InnerNode::Directory] or [Option::None] if the node is a [InnerNode::File].
    pub fn get_n_of_child_nodes(&self) -> Option<usize> {
        let children = self.children()?;
        let mut n_of_children = children.len();
        for (_, child_node) in children.iter() {
            n_of_children +=
                child_node.inner.get_n_of_child_nodes().unwrap_or(0)
        }
        Some(n_of_children)
    }

    /// Returns a reference to the [FileState::actual_file] for this [InnerNode::File] or [Option::None] if the node is a [InnerNode::Directory].
    pub fn get_link(&self) -> Option<&Path> {
        match self {
            InnerNode::File(state) => state.actual_file.as_deref(),
            _ => None,
        }
    }

    /// Returns a mutable reference to the [DirectoryState] of this [InnerNode::Directory] or [Option::None] if the node is a [InnerNode::File].
    pub(super) fn dir_state_mut(&mut self) -> Option<&mut DirectoryState> {
        match self {
            InnerNode::Directory(state) => Some(state),
            _ => None,
        }
    }

    /// Filters this [InnerNode] using the provided filter.
    ///
    /// Returns `true` if there are one or more nodes remaining after the filtering.
    pub(super) fn filter(&mut self, filter: NodeFilters) -> bool {
        if !filter.any() {
            // No filters -> entry is always included
            return true;
        }

        // We ignore files here, as they are handled when processing children of directories
        if let InnerNode::Directory(state) = self {
            state.children.retain(|path, child| {
                // Size-based filtering
                if let Some(node_size_filter) = filter.node_size_filter {
                    if child.inner.size() < node_size_filter {
                        return false;
                    }
                }

                let is_dir_with_children = child
                    .inner
                    .children()
                    .map(|children| !children.is_empty())
                    .unwrap_or(false);

                // Path-based filtering
                let path_filter_for_child =
                    if let Some(path_filter) = filter.path_filter.as_ref() {
                        let is_filtered_out = if let Some(leftmost_part) =
                            path_filter.get_current_component()
                        {
                            path != Path::new(".")
                                && !path
                                    .as_os_str()
                                    .to_str()
                                    // We need to convert both paths to a str to check for a partial match using `contains`
                                    .and_then(|path| {
                                        leftmost_part.to_str().map(
                                            |leftmost_part| {
                                                path.contains(leftmost_part)
                                            },
                                        )
                                    })
                                    // If anything fails here, exclude the node
                                    .unwrap_or(true)
                        } else {
                            // If there is no longer a filter, then we simply retain the node
                            return true;
                        };

                        // Directories are filtered out based on their children when using relative path, so we don't filter them out here
                        if is_filtered_out
                            && (!path_filter.is_using_relative_path()
                                || !is_dir_with_children)
                        {
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

                        // Don't pass an empty filter if we've already used all of its components,
                        // use [Option::None] instead.
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

                // Filter the children
                child.filter(NodeFilters {
                    path_filter: path_filter_for_child,
                    node_size_filter: filter.node_size_filter,
                    path_regex: filter.path_regex.as_deref().map(Cow::Borrowed),
                })
            });

            // Return `true` if one or more children are present after filtering
            return !state.children.is_empty();
        }

        // Include the node if we didn't trigger any other branches
        true
    }

    /// Inserts a new node at the provided path.
    pub(super) fn insert(
        &mut self,
        path: &mut RestorablePath<'_>,
        new_node: Self,
        layer_digest: u8,
    ) -> anyhow::Result<()> {
        let Some(current_path_component) = path.get_current_component() else {
            // Replace the node, as there are no more path components
            *self = new_node;
            return Ok(());
        };

        if let InnerNode::Directory(state) = self {
            if !state.children.contains_key(current_path_component) {
                state.children.insert(
                    current_path_component.into(),
                    Node::new_with_inner(
                        layer_digest,
                        InnerNode::Directory(DirectoryState::new_empty()),
                    ),
                );
            }
        } else {
            // NOTE: This happened in some images when I was testing the app.
            // Some images change type of a node from directory to link back and forth before actually
            // creating any children inside the directory.
            //
            // Thus, we may need to replace a node before appending other nodes to it.
            let mut dir_state = DirectoryState::new_with_size(new_node.size());
            dir_state.children.insert(
                current_path_component.into(),
                Node::new_with_inner(
                    layer_digest,
                    InnerNode::Directory(DirectoryState::new_empty()),
                ),
            );
            *self = InnerNode::Directory(dir_state);
        };

        // Update the size
        self.increase_dir_size(new_node.size());
        let next_node = self
            .children_mut()
            .context("impossible: we made sure this component is a directory")?
            .get_mut(current_path_component)
            .context("impossible: we inserted the missing component above")?;

        next_node.insert(&mut path.advance(), new_node, layer_digest)
    }

    /// Recursively merges two [nodes](InnerNode) together and returns the result.
    pub(super) fn merge(mut self, other: Self, digest: u8) -> Self {
        match (&mut self, other) {
            // Both nodes are directories
            (
                InnerNode::Directory(left_state),
                InnerNode::Directory(right_state),
            ) => {
                for (path, right_node) in right_state.children {
                    // If a node if present in both left and right parent node, we need to merge the two.
                    // Otherwise, we use the node from the right parent as is.
                    let updated_node = if let Some(left_node) =
                        left_state.children.remove(&path)
                    {
                        left_node.inner.merge(right_node.inner, digest)
                    } else {
                        right_node.inner
                    };
                    // Insert the updated node back into the left parent node
                    left_state.children.insert(
                        path,
                        Node::new_with_inner(
                            right_node.updated_in,
                            updated_node,
                        ),
                    );
                }
                // Update the node status accordingly
                let new_status = match (&left_state.status, &right_state.status)
                {
                    (_, NodeStatus::Added(_)) => {
                        // Calculate the updated directory size after merging
                        NodeStatus::Modified(
                            left_state
                                .children
                                .values()
                                .map(|tree| tree.inner.size())
                                .sum(),
                        )
                    }
                    (_, _) => right_state.status,
                };
                left_state.status = new_status;
            }
            // Both nodes are files
            (InnerNode::File(left_state), InnerNode::File(right_state)) => {
                // Update the node status accordingly
                let new_status = match (&left_state.status, &right_state.status)
                {
                    (_, NodeStatus::Added(new_size)) => {
                        NodeStatus::Modified(*new_size)
                    }
                    (_, _) => right_state.status,
                };
                left_state.status = new_status;
            }
            // Nodes are of different type
            (left_node, right_node) => {
                // Check if a directory was deleted using a whiteout file
                if left_node.is_dir() && right_node.is_deleted() {
                    left_node.mark_as_deleted(digest);
                } else {
                    // Can only happen if the type of a node has changed.
                    // If this happens, then we simply want to replace the node altogether.
                    *left_node = right_node
                }
            }
        };

        self
    }

    /// Changes [NodeStatus] of this node and its children (if any) to [NodeStatus::Deleted].
    pub(super) fn mark_as_deleted(&mut self, digest_idx: u8) {
        match self {
            InnerNode::Directory(state) => {
                // Mark the directory itself as deleted
                state.status = NodeStatus::Deleted;
                // Mark each child as 'deleted' recursively
                for tree in state.children.values_mut() {
                    tree.updated_in = digest_idx;
                    tree.inner.mark_as_deleted(digest_idx);
                }
            }
            InnerNode::File(state) => {
                state.status = NodeStatus::Deleted;
            }
        }
    }

    /// Returns a mutable reference to the [DirMap] of children for this [InnerNode::Directory] or [Option::None] if the node is a [InnerNode::File].
    fn children_mut(&mut self) -> Option<&mut DirMap> {
        match self {
            InnerNode::Directory(state) => Some(&mut state.children),
            _ => None,
        }
    }

    /// Increases size of a [InnerNode::Directory].
    ///
    /// # Note
    ///
    /// Ignores [InnerNode::File].
    fn increase_dir_size(&mut self, inc: u64) {
        let status = match self {
            InnerNode::File(_) => return,
            InnerNode::Directory(state) => &mut state.status,
        };

        match status {
            NodeStatus::Added(size) | NodeStatus::Modified(size) => {
                *size += inc
            }
            _ => (),
        }
    }

    /// Returns [NodeStatus] of this [InnerNode].
    fn status(&self) -> NodeStatus {
        match self {
            InnerNode::File(state) => state.status,
            InnerNode::Directory(state) => state.status,
        }
    }
}

impl Default for InnerNode {
    fn default() -> Self {
        InnerNode::new_empty_dir()
    }
}
