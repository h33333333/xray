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
