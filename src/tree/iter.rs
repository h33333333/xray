use std::collections::VecDeque;
use std::path::Path;

use super::Tree;

pub struct NodeIter<'a, F, D> {
    queue: VecDeque<(&'a Tree<F, D>, &'a Path)>,
}

impl<'a, F, D> NodeIter<'a, F, D> {
    pub fn new(tree: &'a Tree<F, D>) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back((tree, Path::new("")));
        NodeIter { queue }
    }
}

impl<'a, F, D> Iterator for NodeIter<'a, F, D> {
    type Item = (&'a Path, &'a Tree<F, D>);

    fn next(&mut self) -> Option<Self::Item> {
        let (next_node, path) = self.queue.pop_front()?;

        if let Some(children) = next_node.children() {
            for (path, node) in children.iter().rev() {
                self.queue.push_front((node, path));
            }
        }

        Some((path, next_node))
    }
}
