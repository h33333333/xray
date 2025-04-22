use std::path::Path;

/// A small helper struct that contains a [Path] and can iterate over its components without changing the underlying [Path] instance.
///
/// The contained [Path] can be restored to its original state if needed.
#[derive(Clone)]
pub struct RestorablePath<'a> {
    path: &'a Path,
    current_component_idx: u8,
    is_using_relative_path: bool,
}

impl<'a> RestorablePath<'a> {
    /// Creates a new [RestorablePath] using the provided [Path].
    pub fn new(path: &'a Path) -> Self {
        RestorablePath {
            path,
            current_component_idx: 0,
            is_using_relative_path: !path.starts_with("/"),
        }
    }

    /// Returns the current [Self::path] component.
    pub(super) fn get_current_component(&self) -> Option<&Path> {
        self.path.iter().nth(self.current_component_idx as usize).map(Path::new)
    }

    /// Returns a new [RestorablePath] with its state reset to the original state of this instance.
    pub(super) fn restore(&self) -> Self {
        RestorablePath {
            path: self.path,
            current_component_idx: 0,
            is_using_relative_path: self.is_using_relative_path,
        }
    }

    /// Returns a new [RestorablePath] with its path component index increased by `1`.
    pub(super) fn advance(&self) -> Self {
        RestorablePath {
            path: self.path,
            current_component_idx: self.current_component_idx + 1,
            is_using_relative_path: self.is_using_relative_path,
        }
    }

    /// Strips the leading slash from the contained path.
    pub(super) fn strip_prefix(&mut self) {
        self.path = self.path.strip_prefix("/").ok().unwrap_or(self.path);
    }

    pub(super) fn is_using_relative_path(&self) -> bool {
        self.is_using_relative_path
    }
}
