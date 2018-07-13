//! Cloud storage backends.

use std::path::Path;

use Result;

pub mod gs;

/// Abstract interface to different kinds of cloud storage backends.
pub trait CloudStorage {
    /// List all the files and subdirectories immediately present in `uri` if
    /// `uri` is a directory, or just return `uri` if it points to a file.
    fn list(&self, uri: &str) -> Result<Vec<String>>;

    /// Synchronize `uri` down to `local_path` recursively. Does not delete any
    /// existing destination files. The contents of `uri` should be exactly
    /// represented in `local_path`, without the trailing subdirectory name
    /// being inserted—this is a straight directory-to-directory sync.
    fn sync_down(&self, uri: &str, local_path: &Path) -> Result<()>;

    /// Synchronize `local_path` to `uri` recursively. Does not delete any
    /// existing destination files. The contents of `local_path` should be
    /// exactly represented in `uri`, without the trailing subdirectory name
    /// being inserted—this is a straight directory-to-directory sync.
    fn sync_up(&self, local_path: &Path, uri: &str) -> Result<()>;
}

impl CloudStorage {
    /// Get the storage backend for the specified URI.
    pub fn for_uri(uri: &str) -> Result<Box<dyn CloudStorage>> {
        if uri.starts_with("gs://") {
            Ok(Box::new(gs::GoogleCloudStorage::new()))
        } else {
            Err(format_err!("cannot find storage backend for {}", uri))
        }
    }
}
