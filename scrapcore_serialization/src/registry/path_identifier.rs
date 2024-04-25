use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

/// Opaque identifier for a path to be used in registry
///
/// Mostly used for error reporting, and for looking up assets by their name
#[derive(Debug, Clone)]
pub struct PathIdentifier {
    components: Vec<String>,
}

impl Display for PathIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.components.join("/"))
    }
}

impl PathIdentifier {
    /// Create a new path identifier from a list of components
    pub fn from_components<'a>(components: impl IntoIterator<Item = &'a str>) -> PathIdentifier {
        let components = components.into_iter().map(|s| s.to_string()).collect();
        Self { components }
    }

    /// Get the file name of the path
    pub fn file_name(&self) -> Option<&OsStr> {
        self.components
            .last()
            .and_then(|s| Path::file_name(s.as_ref()))
    }
}

impl From<PathBuf> for PathIdentifier {
    fn from(value: PathBuf) -> Self {
        Self::from(value.as_path())
    }
}

impl From<&Path> for PathIdentifier {
    fn from(path: &Path) -> Self {
        let components = path
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        Self { components }
    }
}
