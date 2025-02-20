use std::collections::{HashSet, VecDeque};
use std::path::Path;

use super::Tree;

pub struct NodeIter<'a> {
    queue: VecDeque<(&'a Tree, &'a Path, usize)>,
    active_levels: Option<HashSet<usize>>,
}

impl<'a> NodeIter<'a> {
    pub fn new(tree: &'a Tree, track_levels: bool) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back((tree, Path::new("."), 0));
        NodeIter {
            queue,
            active_levels: track_levels.then(HashSet::new),
        }
    }

    pub fn is_level_active(&self, level: usize) -> Option<bool> {
        self.active_levels.as_ref().map(|levels| levels.contains(&level))
    }

    pub fn enumerate(self) -> EnumeratedNodeIter<'a> {
        EnumeratedNodeIter::new(self)
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = (&'a Path, &'a Tree, usize, bool);

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
        if let Some(children) = next_node.children() {
            for (path, node) in children.iter().rev() {
                self.queue.push_front((node, path, depth + 1));
            }
        }

        Some((path, next_node, depth, is_level_active))
    }
}

pub struct EnumeratedNodeIter<'a> {
    inner: NodeIter<'a>,
    count: usize,
}

impl<'a> EnumeratedNodeIter<'a> {
    fn new(inner: NodeIter<'a>) -> Self {
        EnumeratedNodeIter { inner, count: 0 }
    }

    pub fn is_level_active(&self, level: usize) -> Option<bool> {
        self.inner.is_level_active(level)
    }
}

impl<'a> Iterator for EnumeratedNodeIter<'a> {
    type Item = (usize, (&'a Path, &'a Tree, usize, bool));

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.count;
        let item = self.inner.next()?;
        self.count += 1;
        Some((idx, item))
    }
}
