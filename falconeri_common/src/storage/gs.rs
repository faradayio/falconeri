//! Support for Google Cloud Storage.

use std::{io::BufRead, process};

use prefix::*;
use secret::Secret;
use super::CloudStorage;

/// Backend for talking to Google Cloud Storage, currently based on `gsutil`.
pub struct GoogleCloudStorage {}

impl GoogleCloudStorage {
    /// Create a new `GoogleCloudStorage` backend.
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
        let mut paths = vec![];
        for line in output.stdout.lines() {
            let line = line?;
            paths.push(line.trim_right().to_owned());
        }
        Ok(paths)
    }

    fn sync_down(&self, uri: &str, local_path: &Path) -> Result<()> {
        trace!("downloading {} to {}", uri, local_path.display());
        let status = process::Command::new("gsutil")
            .args(&["-m", "cp", "-r"])
            .arg(uri)
            .arg(local_path)
            .status()
            .context("could not run gsutil")?;
        if !status.success() {
            return Err(format_err!("could not download {:?}: {}", uri, status));
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
