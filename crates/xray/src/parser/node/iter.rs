use std::collections::{HashSet, VecDeque};
use std::path::Path;

use super::Node;

/// An iterator over [nodes](Node) of a file tree.
pub struct TreeIter<'a> {
    /// A queue of items that this iterator needs to process.
    queue: VecDeque<(&'a Node, &'a Path, usize)>,
    /// If present, tracks active depth levels (i.e. when a node is a children of a node that has siblings after it).
    ///
    /// This is used mostly during rendering the file tree to determine the correct branch prefixes and indicators for a node.
    active_levels: Option<HashSet<usize>>,
}

impl<'a> TreeIter<'a> {
    /// Mimics the `enumerate` method on most iterators and turns this iterator instance into a [EnumeratedNodeIter].
    pub fn enumerate(self) -> EnumeratedNodeIter<'a> {
        EnumeratedNodeIter::new(self)
    }

    /// Creates a new iterator.
    ///
    /// Pass `true` as the second parameter if you want this instance to track the active depth levels as well.
    pub(super) fn new(node: &'a Node, track_levels: bool) -> Self {
        let mut queue = VecDeque::new();

        if let Some(children) = node.inner.children() {
            queue.extend(children.iter().map(|(path, node)| (node, path.as_ref(), 0)));
        } else {
            // The node tree consists of a single file node.
            queue.push_back((node, Path::new("."), 0));
        }

        TreeIter {
            queue,
            active_levels: track_levels.then(HashSet::new),
        }
    }

    pub(super) fn is_level_active(&self, level: usize) -> Option<bool> {
        self.active_levels.as_ref().map(|levels| levels.contains(&level))
    }
}

impl<'a> Iterator for TreeIter<'a> {
    type Item = (&'a Path, &'a Node, usize, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let (next_node, path, depth) = self.queue.pop_front()?;

        let is_level_active = self
            .queue
            .front()
            .is_some_and(|(_, _, next_depth)| next_depth == &depth);

        if let Some(active_levels) = self.active_levels.as_mut() {
            if is_level_active {
                // Mark current level as active
                active_levels.insert(depth);
            } else {
                // Mark current level as inactive
                active_levels.remove(&depth);
            }
        }

        if let Some(children) = next_node.inner.children() {
            for (child_path, node) in children.iter().rev() {
                self.queue.push_front((node, child_path, depth + 1));
            }
        }

        Some((path, next_node, depth, is_level_active))
    }
}

/// Mimics the [std::iter::Enumerate] iterator but also allows getting information about the active depth levels.
pub struct EnumeratedNodeIter<'a> {
    inner: TreeIter<'a>,
    count: usize,
}

impl<'a> EnumeratedNodeIter<'a> {
    fn new(inner: TreeIter<'a>) -> Self {
        EnumeratedNodeIter { inner, count: 0 }
    }

    /// Returns `true` if the provided depth level is active (i.e. still has nodes after the current one).
    pub fn is_level_active(&self, level: usize) -> Option<bool> {
        self.inner.is_level_active(level)
    }
}

impl<'a> Iterator for EnumeratedNodeIter<'a> {
    type Item = (usize, (&'a Path, &'a Node, usize, bool));

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.count;
        let item = self.inner.next()?;
        self.count += 1;
        Some((idx, item))
    }
}
