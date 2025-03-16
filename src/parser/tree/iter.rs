use std::collections::{HashSet, VecDeque};
use std::path::Path;

use super::Tree;

pub struct TreeIter<'a, 'filter> {
    queue: VecDeque<(&'a Tree, &'a Path, usize, Option<&'filter Path>)>,
    active_levels: Option<HashSet<usize>>,
}

impl<'a, 'f> TreeIter<'a, 'f> {
    pub fn new(tree: &'a Tree, track_levels: bool, filter: Option<&'f Path>) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back((
            tree,
            Path::new("."),
            0,
            filter.and_then(|filter| filter.strip_prefix("/").ok()),
        ));
        TreeIter {
            queue,
            active_levels: track_levels.then(HashSet::new),
        }
    }

    pub fn is_level_active(&self, level: usize) -> Option<bool> {
        self.active_levels.as_ref().map(|levels| levels.contains(&level))
    }

    pub fn enumerate(self) -> EnumeratedNodeIter<'a, 'f> {
        EnumeratedNodeIter::new(self)
    }
}

impl<'a> Iterator for TreeIter<'a, '_> {
    type Item = (&'a Path, &'a Tree, usize, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let (next_node, path, depth, filter) = self.queue.pop_front()?;

        let is_level_active = self
            .queue
            .front()
            .is_some_and(|(_, _, next_depth, _)| next_depth == &depth);

        if let Some(active_levels) = self.active_levels.as_mut() {
            if is_level_active {
                // Mark current level as active
                active_levels.insert(depth);
            } else {
                // Mark current level as inactive
                active_levels.remove(&depth);
            }
        }

        let is_filtered_out = if let Some(remaining_path) = filter {
            // A node is includedd if either its path satisfies the leftmost part of the filter or it's the root node,
            // in which case we want to strip the lead `/` and continue filtering the actual nodes
            !remaining_path.starts_with(path) && path != Path::new(".")
        } else {
            false
        };

        // Do not do anything else with this node if it doesn't satisfy the filter
        if is_filtered_out {
            return self.next();
        }

        if let Some(children) = next_node.node.children() {
            for (child_path, node) in children.iter().rev() {
                let filter_for_child = filter
                    .and_then(|current_filter| {
                        if path != Path::new(".") {
                            current_filter.strip_prefix(path).ok()
                        } else {
                            // Pass the filter as is
                            Some(current_filter)
                        }
                    })
                    .filter(|new_filter| !new_filter.as_os_str().is_empty());

                self.queue.push_front((node, child_path, depth + 1, filter_for_child));
            }
        }

        Some((path, next_node, depth, is_level_active))
    }
}

pub struct EnumeratedNodeIter<'a, 'f> {
    inner: TreeIter<'a, 'f>,
    count: usize,
}

impl<'a, 'f> EnumeratedNodeIter<'a, 'f> {
    fn new(inner: TreeIter<'a, 'f>) -> Self {
        EnumeratedNodeIter { inner, count: 0 }
    }

    pub fn is_level_active(&self, level: usize) -> Option<bool> {
        self.inner.is_level_active(level)
    }
}

impl<'a> Iterator for EnumeratedNodeIter<'a, '_> {
    type Item = (usize, (&'a Path, &'a Tree, usize, bool));

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.count;
        let item = self.inner.next()?;
        self.count += 1;
        Some((idx, item))
    }
}
