//! Cloud storage backends.

use crate::prelude::*;
use crate::secret::Secret;

pub mod gs;
pub mod s3;

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

impl dyn CloudStorage {
    /// Get the storage backend for the specified URI. If we know about any
    /// secrets, we can pass them as the `secrets` array, and the storage driver
    /// can check to see if there are any secrets it can use to authenticate.
    pub fn for_uri(uri: &str, secrets: &[Secret]) -> Result<Box<dyn CloudStorage>> {
        if uri.starts_with("gs://") {
            Ok(Box::new(gs::GoogleCloudStorage::new(secrets)?))
        } else if uri.starts_with("s3://") {
            Ok(Box::new(s3::S3Storage::new(secrets)?))
        } else {
            Err(format_err!("cannot find storage backend for {}", uri))
        }
    }
}
