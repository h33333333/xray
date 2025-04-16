use std::path::Path;

/// A small helper struct that contains a [Path] and returns its individual components and
/// can be restored to its original state if needed.
#[derive(Clone)]
pub(super) struct RestorablePathFilter<'a> {
    path: &'a Path,
    current_component_idx: u8,
    is_using_relative_path: bool,
}

impl<'a> RestorablePathFilter<'a> {
    pub(super) fn new(path: &'a Path) -> Self {
        RestorablePathFilter {
            path,
            current_component_idx: 0,
            is_using_relative_path: !path.starts_with("/"),
        }
    }

    pub(super) fn get_current_component(&self) -> Option<&Path> {
        self.path.iter().nth(self.current_component_idx as usize).map(Path::new)
    }

    pub(super) fn restore(&self) -> Self {
        RestorablePathFilter {
            path: self.path,
            current_component_idx: 0,
            is_using_relative_path: self.is_using_relative_path,
        }
    }

    pub(super) fn advance(&self) -> Self {
        RestorablePathFilter {
            path: self.path,
            current_component_idx: self.current_component_idx + 1,
            is_using_relative_path: self.is_using_relative_path,
        }
    }

    pub(super) fn strip_prefix(&mut self) {
        self.path = self.path.strip_prefix("/").ok().unwrap_or(self.path);
    }

    pub(super) fn is_using_relative_path(&self) -> bool {
        self.is_using_relative_path
    }
}
