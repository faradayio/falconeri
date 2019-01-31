//! Support for Google Cloud Storage.

use std::{collections::HashSet, fs, io::BufRead, process};

use super::CloudStorage;
use crate::prelude::*;
use crate::secret::Secret;

/// Backend for talking to Google Cloud Storage, currently based on `gsutil`.
pub struct GoogleCloudStorage {}

impl GoogleCloudStorage {
    /// Create a new `GoogleCloudStorage` backend.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(_secrets: &[Secret]) -> Result<Self> {
        // We don't yet know how to authenticate using secrets.
        Ok(GoogleCloudStorage {})
    }
}

impl CloudStorage for GoogleCloudStorage {
    fn list(&self, uri: &str) -> Result<Vec<String>> {
        trace!("listing {}", uri);
        // Shell out to gsutil to list the files we want to process.
        let output = process::Command::new("gsutil")
            .arg("ls")
            .arg(&uri)
            .stderr(process::Stdio::inherit())
            .output()
            .context("error running gsutil")?;
        if !output.status.success() {
            return Err(format_err!("could not list {:?}: {}", uri, output.status));
        }
        // `gsutil ls` is "eventually consistent", and seems to occasionally retun
        // duplicate entries.
        let mut paths = HashSet::new();
        for line in output.stdout.lines() {
            let line = line?;
            paths.insert(line.trim_right().to_owned());
        }
        Ok(paths.into_iter().collect())
    }

    fn sync_down(&self, uri: &str, local_path: &Path) -> Result<()> {
        if uri.ends_with('/') {
            // We have a directory. If our source URI ends in `/`, so should our
            // `local_path`, since we generate these ourselves.
            assert!(local_path
                .to_str()
                .expect("path should be UTF-8")
                .ends_with('/'));
            trace!("syncing {} to {}", uri, local_path.display());
            fs::create_dir_all(local_path)
                .context("cannot create local download directory")?;
            let status = process::Command::new("gsutil")
                .args(&["-m", "rsync"])
                .arg(uri)
                .arg(local_path)
                .status()
                .context("could not run gsutil rsync")?;
            if !status.success() {
                return Err(format_err!("could not download {:?}: {}", uri, status));
            }
        } else {
            // We have a file.
            trace!("downloading {} to {}", uri, local_path.display());
            if let Some(parent) = local_path.parent() {
                fs::create_dir_all(parent)
                    .context("cannot create local download directory")?;
            }
            let status = process::Command::new("gsutil")
                .args(&["-m", "cp", "-r"])
                .arg(uri)
                .arg(local_path)
                .status()
                .context("could not run gsutil cp")?;
            if !status.success() {
                return Err(format_err!("could not download {:?}: {}", uri, status));
            }
        }
        Ok(())
    }

    fn sync_up(&self, local_path: &Path, uri: &str) -> Result<()> {
        trace!("uploading {} to {}", local_path.display(), uri);
        let status = process::Command::new("gsutil")
            .args(&["-m", "rsync", "-r"])
            .arg(local_path)
            .arg(uri)
            .status()
            .context("could not run gsutil")?;
        if !status.success() {
            return Err(format_err!(
                "could not upload {}: {}",
                local_path.display(),
                status,
            ));
        }
        Ok(())
    }
}
