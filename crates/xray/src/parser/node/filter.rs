use std::borrow::Cow;
use std::path::Path;

use regex::Regex;

use super::RestorablePathFilter;

/// Contains all filters that can be applied to file tree [Nodes](super::Node).
#[derive(Default)]
pub struct NodeFilters<'a, 'r> {
    pub(super) path_filter: Option<RestorablePathFilter<'a>>,
    pub(super) node_size_filter: Option<u64>,
    pub(super) path_regex: Option<Cow<'r, Regex>>,
}

impl<'a, 'r> NodeFilters<'a, 'r> {
    pub fn with_path_filter<'n>(self, filter: &'n Path) -> NodeFilters<'n, 'r> {
        NodeFilters {
            path_filter: Some(RestorablePathFilter::new(filter)),
            node_size_filter: self.node_size_filter,
            path_regex: None,
        }
    }

    pub fn with_size_filter(self, filter: u64) -> Self {
        NodeFilters {
            path_filter: self.path_filter,
            node_size_filter: Some(filter),
            path_regex: self.path_regex,
        }
    }

    pub fn with_regex<'n>(self, regex: Cow<'n, Regex>) -> NodeFilters<'a, 'n> {
        NodeFilters {
            path_filter: None,
            node_size_filter: self.node_size_filter,
            path_regex: Some(regex),
        }
    }

    /// Returns true if any supported filter is set.
    pub fn any(&self) -> bool {
        self.path_filter.is_some() || self.node_size_filter.is_some() || self.path_regex.is_some()
    }

    /// Strips the leading slash from the path filter if it's present.
    pub fn strip_path_filter_prefix(&mut self) {
        if let Some(path_filter) = self.path_filter.as_mut() {
            path_filter.strip_prefix();
        }
    }
}
