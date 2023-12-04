use crate::storage::StorePrefix;
use derive_more::Display;
use std::path::PathBuf;
use thiserror::Error;

/// A Zarr hierarchy node path.
///
/// See <https://zarr-specs.readthedocs.io/en/latest/v3/core/v3.0.html#path>
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{_0:?}")]
pub struct NodePath(PathBuf);

/// An invalid node path.
#[derive(Debug, Error)]
#[error("invalid node path {0}")]
pub struct NodePathError(String);

impl NodePath {
    /// Create a new Zarr node path from `path`.
    ///
    /// # Errors
    ///
    /// Returns [`NodePathError`] if `path` is not valid according to [`NodePath::validate`()].
    pub fn new(path: &str) -> Result<Self, NodePathError> {
        if Self::validate(path) {
            Ok(Self(PathBuf::from(path)))
        } else {
            Err(NodePathError(path.to_string()))
        }
    }

    /// The root node.
    #[must_use]
    pub fn root() -> Self {
        Self(PathBuf::from("/"))
    }

    /// Extracts a string slice containing the node path `String`.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.to_str().unwrap()
    }

    /// Extracts the path as a [`std::path::Path`].
    #[must_use]
    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }

    /// Validates a path according to the following rules from the specification:
    /// - A path always starts with `/`, and
    /// - a non-root path cannot end with `/`, because node names must be non-empty and cannot contain `/`.
    ///
    /// Additionally, it checks that there are no empty nodes (i.e. a `//` substring).
    #[must_use]
    pub fn validate(path: &str) -> bool {
        path.eq("/") || (path.starts_with('/') && !path.ends_with('/') && !path.contains("//"))
    }
}

impl TryFrom<&str> for NodePath {
    type Error = NodePathError;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<&StorePrefix> for NodePath {
    type Error = NodePathError;

    fn try_from(prefix: &StorePrefix) -> Result<Self, Self::Error> {
        let path = "/".to_string() + prefix.as_str().strip_suffix('/').unwrap();
        Self::new(&path)
    }
}
