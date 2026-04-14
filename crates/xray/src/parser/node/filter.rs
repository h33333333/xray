use std::borrow::Cow;
use std::path::Path;

use regex::Regex;

use super::RestorablePath;

/// Contains all filters that can be applied to file tree [Nodes](super::Node).
#[derive(Default)]
pub struct NodeFilters<'a, 'r> {
    /// A standard path filter.
    ///
    /// Suppots both relative and absolute paths.
    pub(super) path_filter: Option<RestorablePath<'a>>,
    /// Won't display size whose size is lower than the specified number of bytes.
    pub(super) node_size_filter: Option<u64>,
    /// A path filter that uses regular expressions.
    pub(super) path_regex: Option<Cow<'r, Regex>>,
    /// Show files changed in the layer with the provided index (i.e. added, deleted, or modified).
    ///
    /// NOTE: layer's index has to be provided by the caller, as node itself is not aware of the
    /// layer it's in.
    pub(super) show_nodes_changed_in_layer: Option<u8>,
    /// An inner filter (not controlled by the user) that determines whether a directory
    /// should be included if none of its children passed the filtering.
    pub(super) include_dir_if_no_children_remained: bool,
}

impl<'a, 'r> NodeFilters<'a, 'r> {
    /// Adds a path filter and returns a new instance.
    pub fn with_path_filter<'n>(self, filter: &'n Path) -> NodeFilters<'n, 'r> {
        NodeFilters {
            path_filter: Some(RestorablePath::new(filter)),
            node_size_filter: self.node_size_filter,
            path_regex: None,
            show_nodes_changed_in_layer: self.show_nodes_changed_in_layer,
            include_dir_if_no_children_remained: self
                .include_dir_if_no_children_remained,
        }
    }

    /// Adds a node size filter and returns a new instance.
    pub fn with_size_filter(self, filter: u64) -> Self {
        NodeFilters {
            path_filter: self.path_filter,
            node_size_filter: Some(filter),
            path_regex: self.path_regex,
            show_nodes_changed_in_layer: self.show_nodes_changed_in_layer,
            include_dir_if_no_children_remained: self
                .include_dir_if_no_children_remained,
        }
    }

    /// Adds a new path filter with regular expressions and returns a new instance.
    pub fn with_regex<'n>(self, regex: Cow<'n, Regex>) -> NodeFilters<'a, 'n> {
        NodeFilters {
            path_filter: None,
            node_size_filter: self.node_size_filter,
            path_regex: Some(regex),
            show_nodes_changed_in_layer: self.show_nodes_changed_in_layer,
            include_dir_if_no_children_remained: self
                .include_dir_if_no_children_remained,
        }
    }

    /// Makes the filter filter-out all files that weren't changed in the layer with provided index.
    pub fn with_show_files_changed_in_layer(self, layer_index: u8) -> Self {
        NodeFilters {
            path_filter: self.path_filter,
            node_size_filter: self.node_size_filter,
            path_regex: self.path_regex,
            show_nodes_changed_in_layer: Some(layer_index),
            include_dir_if_no_children_remained: self
                .include_dir_if_no_children_remained,
        }
    }

    /// Returns true if any supported filter is set.
    pub fn any(&self) -> bool {
        self.path_filter.is_some()
            || self.node_size_filter.is_some()
            || self.path_regex.is_some()
            || self.show_nodes_changed_in_layer.is_some()
    }

    /// Returns true if the only applied filter is showing only changed nodes.
    pub fn only_changed_nodes_filter(&self) -> bool {
        self.path_filter.is_none()
            && self.node_size_filter.is_none()
            && self.path_regex.is_none()
            && self.show_nodes_changed_in_layer.is_some()
    }

    /// Returns true if this filter contains any non-path filter (either node size or showing
    /// only changed files).
    pub fn any_non_path_filter(&self) -> bool {
        self.node_size_filter.is_some()
            || self.show_nodes_changed_in_layer.is_some()
    }

    /// Strips the leading slash from the path filter if it's present.
    pub(super) fn strip_path_filter_prefix(&mut self) {
        if let Some(path_filter) = self.path_filter.as_mut() {
            path_filter.strip_prefix();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::path::Path;

    use regex::Regex;

    use super::*;
    use crate::parser::{FileState, Node, NodeStatus};

    fn make_file(size: u64) -> super::super::InnerNode {
        super::super::InnerNode::File(FileState::new(
            NodeStatus::Added(size),
            None,
        ))
    }

    fn build_tree() -> Node {
        // Build:
        //   usr/
        //     bin/
        //       grep (50)
        //       find (30)
        //     lib/
        //       libc.so (1000)
        //   etc/
        //     hosts (10)
        //   tmp/
        //     cache (5)
        let mut root = Node::new(0);
        root.insert(
            &mut RestorablePath::new(Path::new("usr/bin/grep")),
            make_file(50),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("usr/bin/find")),
            make_file(30),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("usr/lib/libc.so")),
            make_file(1000),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("etc/hosts")),
            make_file(10),
            0,
        )
        .unwrap();
        root.insert(
            &mut RestorablePath::new(Path::new("tmp/cache")),
            make_file(5),
            1,
        )
        .unwrap();
        root
    }

    fn remaining_paths(node: &Node) -> Vec<String> {
        node.iter()
            .map(|(path, _, _, _)| path.to_string_lossy().to_string())
            .collect()
    }

    // --- Size filter ---

    #[test]
    fn filter_by_size_removes_small_files_and_empty_dirs() {
        let mut tree = build_tree();
        let filter = NodeFilters::default().with_size_filter(100);
        tree.filter(filter);

        let paths = remaining_paths(&tree);
        // Only libc.so (1000) and its ancestor dirs should survive
        assert!(paths.contains(&"libc.so".to_string()));
        assert!(paths.contains(&"usr".to_string()));
        assert!(paths.contains(&"lib".to_string()));
        // Dirs whose children were all filtered out should be gone
        assert!(!paths.contains(&"bin".to_string()));
        assert!(!paths.contains(&"etc".to_string()));
        assert!(!paths.contains(&"tmp".to_string()));
        assert!(!paths.contains(&"grep".to_string()));
        assert!(!paths.contains(&"hosts".to_string()));
    }

    #[test]
    fn filter_by_size_keeps_equal() {
        let mut tree = build_tree();
        let filter = NodeFilters::default().with_size_filter(50);
        tree.filter(filter);

        let paths = remaining_paths(&tree);
        assert!(paths.contains(&"grep".to_string()));
        assert!(paths.contains(&"libc.so".to_string()));
        assert!(!paths.contains(&"find".to_string())); // 30 < 50
    }

    // --- Layer change filter ---

    #[test]
    fn filter_by_layer_shows_only_that_layers_changes() {
        let mut tree = build_tree();
        // Layer 1 only has tmp/cache
        let filter = NodeFilters::default().with_show_files_changed_in_layer(1);
        tree.filter(filter);

        let paths = remaining_paths(&tree);
        assert!(paths.contains(&"cache".to_string()));
        // Layer 0 files should be gone
        assert!(!paths.contains(&"grep".to_string()));
        assert!(!paths.contains(&"libc.so".to_string()));
    }

    // --- Path filter (absolute) ---

    #[test]
    fn filter_by_absolute_path() {
        let mut tree = build_tree();
        let filter =
            NodeFilters::default().with_path_filter(Path::new("/usr/bin"));
        tree.filter(filter);

        let paths = remaining_paths(&tree);
        // Matched files and their ancestor dirs are retained
        assert!(paths.contains(&"usr".to_string()));
        assert!(paths.contains(&"bin".to_string()));
        assert!(paths.contains(&"grep".to_string()));
        assert!(paths.contains(&"find".to_string()));
        // Unmatched subtrees are gone
        assert!(!paths.contains(&"lib".to_string()));
        assert!(!paths.contains(&"etc".to_string()));
        assert!(!paths.contains(&"hosts".to_string()));
    }

    // --- Path filter (relative / partial match) ---

    #[test]
    fn filter_by_relative_path_matches_partial() {
        let mut tree = build_tree();
        let filter = NodeFilters::default().with_path_filter(Path::new("bin"));
        tree.filter(filter);

        let paths = remaining_paths(&tree);
        assert!(paths.contains(&"grep".to_string()));
        assert!(paths.contains(&"find".to_string()));
    }

    // --- Regex filter ---

    #[test]
    fn filter_by_regex_matches_filenames() {
        let mut tree = build_tree();
        let regex = Regex::new(r"\.so$").unwrap();
        let filter = NodeFilters::default().with_regex(Cow::Owned(regex));
        tree.filter(filter);

        let paths = remaining_paths(&tree);
        assert!(paths.contains(&"libc.so".to_string()));
        assert!(!paths.contains(&"grep".to_string()));
        assert!(!paths.contains(&"hosts".to_string()));
    }

    #[test]
    fn filter_by_regex_no_match_empties_tree() {
        let mut tree = build_tree();
        let regex = Regex::new(r"^zzz_nonexistent$").unwrap();
        let filter = NodeFilters::default().with_regex(Cow::Owned(regex));
        let has_nodes = tree.filter(filter);

        assert!(!has_nodes);
    }

    // --- No filter retains everything ---

    #[test]
    fn no_filter_retains_all() {
        let mut tree = build_tree();
        let filter = NodeFilters::default();
        tree.filter(filter);

        let paths = remaining_paths(&tree);
        // 5 dirs (usr, bin, lib, etc, tmp) + 5 files (grep, find, libc.so, hosts, cache)
        assert_eq!(paths.len(), 10);
    }

    // --- NodeFilters builder queries ---

    #[test]
    fn any_returns_false_when_empty() {
        let f = NodeFilters::default();
        assert!(!f.any());
    }

    #[test]
    fn any_returns_true_with_size_filter() {
        let f = NodeFilters::default().with_size_filter(100);
        assert!(f.any());
    }

    #[test]
    fn only_changed_nodes_filter_true_when_only_layer_set() {
        let f = NodeFilters::default().with_show_files_changed_in_layer(0);
        assert!(f.only_changed_nodes_filter());
    }

    #[test]
    fn only_changed_nodes_filter_false_with_size_too() {
        let f = NodeFilters::default()
            .with_size_filter(10)
            .with_show_files_changed_in_layer(0);
        assert!(!f.only_changed_nodes_filter());
    }

    #[test]
    fn any_non_path_filter_false_with_only_path() {
        let f = NodeFilters::default().with_path_filter(Path::new("/usr"));
        assert!(!f.any_non_path_filter());
    }

    #[test]
    fn any_non_path_filter_false_with_only_regex() {
        let regex = Regex::new(r"foo").unwrap();
        let f = NodeFilters::default().with_regex(Cow::Owned(regex));
        assert!(!f.any_non_path_filter());
    }

    #[test]
    fn any_non_path_filter_true_with_size() {
        let f = NodeFilters::default().with_size_filter(100);
        assert!(f.any_non_path_filter());
    }

    #[test]
    fn any_non_path_filter_true_with_layer() {
        let f = NodeFilters::default().with_show_files_changed_in_layer(2);
        assert!(f.any_non_path_filter());
    }
}
