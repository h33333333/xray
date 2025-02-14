use std::collections::VecDeque;
use std::path::Path;

use super::Tree;

pub struct NodeIter<'a> {
    queue: VecDeque<(&'a Tree, &'a Path, usize)>,
}

impl<'a> NodeIter<'a> {
    pub fn new(tree: &'a Tree) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back((tree, Path::new(""), 0));
        NodeIter { queue }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = (&'a Path, &'a Tree, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (next_node, path, depth) = self.queue.pop_front()?;

        if let Some(children) = next_node.children() {
            for (path, node) in children.iter().rev() {
                self.queue.push_front((node, path, depth + 1));
            }
        }

        Some((path, next_node, depth))
    }
}
