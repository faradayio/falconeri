//! Support for Google Cloud Storage.

use std::future::Future;
use std::path::Path;

use google_cloud_auth::credentials::service_account;
use google_cloud_storage::client::{Storage, StorageControl};

use super::CloudStorage;
use crate::prelude::*;
use crate::secret::Secret;

/// Run an async function, either using the current Tokio runtime if available,
/// or creating a new one if not.
fn run_async<F, T>(future: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new()
                .context("failed to create Tokio runtime")?;
            rt.block_on(future)
        }
    }
}

/// Backend for talking to Google Cloud Storage using the native GCS SDK.
#[derive(Debug)]
pub struct GoogleCloudStorage {
    client: Storage,
    control: StorageControl,
}

impl GoogleCloudStorage {
    /// Create a new `GoogleCloudStorage` backend.
    #[allow(clippy::new_ret_no_self)]
    #[tracing::instrument(level = "trace")]
    pub fn new(_secrets: &[Secret]) -> Result<Self> {
        let (client, control) = run_async(async {
            let mut storage_builder = Storage::builder();
            let mut control_builder = StorageControl::builder();

            // Support GCLOUD_SERVICE_ACCOUNT_KEY (falconeri's current env var)
            if let Ok(key_json) = std::env::var("GCLOUD_SERVICE_ACCOUNT_KEY") {
                let json: serde_json::Value = serde_json::from_str(&key_json)
                    .context("failed to parse GCLOUD_SERVICE_ACCOUNT_KEY as JSON")?;
                let credentials = service_account::Builder::new(json)
                    .build()
                    .context("failed to build service account credentials")?;
                storage_builder =
                    storage_builder.with_credentials(credentials.clone());
                control_builder = control_builder.with_credentials(credentials);
            }
            // Otherwise falls back to standard ADC (GOOGLE_APPLICATION_CREDENTIALS or metadata service)

            let client = storage_builder.build().await?;
            let control = control_builder.build().await?;
            Ok::<_, anyhow::Error>((client, control))
        })?;

        Ok(GoogleCloudStorage { client, control })
    }
}

/// Parse a `gs://bucket/object` URI into bucket and object components.
fn parse_gs_uri(uri: &str) -> Result<(String, String)> {
    let url = url::Url::parse(uri)?;
    if url.scheme() != "gs" {
        return Err(format_err!("expected gs:// URL, got {}", uri));
    }
    let bucket = url
        .host_str()
        .ok_or_else(|| format_err!("no bucket in {}", uri))?
        .to_string();
    let object = url.path().trim_start_matches('/').to_string();
    Ok((bucket, object))
}

impl CloudStorage for GoogleCloudStorage {
    #[tracing::instrument(level = "trace")]
    fn list(&self, uri: &str) -> Result<Vec<String>> {
        trace!("listing {}", uri);

        let (bucket, prefix) = parse_gs_uri(uri)?;

        run_async(async {
            let mut results = Vec::new();
            let mut page_token: Option<String> = None;

            loop {
                let mut request = self
                    .control
                    .list_objects()
                    .set_parent(format!("projects/_/buckets/{}", bucket))
                    .set_prefix(prefix.clone());

                if let Some(token) = page_token {
                    request = request.set_page_token(token);
                }

                let response = request.send().await?;

                for object in response.objects {
                    results.push(format!("gs://{}/{}", bucket, object.name));
                }

                page_token = if response.next_page_token.is_empty() {
                    None
                } else {
                    Some(response.next_page_token)
                };
                if page_token.is_none() {
                    break;
                }
            }

            Ok(results)
        })
    }

    #[tracing::instrument(level = "trace")]
    fn sync_down(&self, uri: &str, local_path: &Path) -> Result<()> {
        if uri.ends_with('/') {
            // We have a directory. If our source URI ends in `/`, so should our
            // `local_path`, since we generate these ourselves.
            assert!(local_path
                .to_str()
                .expect("path should be UTF-8")
                .ends_with('/'));
            trace!("syncing {} to {}", uri, local_path.display());

            // List all files with this prefix and download each
            let files = self.list(uri)?;
            for file_uri in files {
                let (bucket, object) = parse_gs_uri(&file_uri)?;

                // Construct local path
                let relative = object.trim_start_matches(&parse_gs_uri(uri)?.1);
                let file_local_path = local_path.join(relative);

                // Download the file
                self.download_file(&bucket, &object, &file_local_path)?;
            }
            Ok(())
        } else {
            // We have a single file
            trace!("downloading {} to {}", uri, local_path.display());
            let (bucket, object) = parse_gs_uri(uri)?;
            self.download_file(&bucket, &object, local_path)
        }
    }

    #[tracing::instrument(level = "trace")]
    fn sync_up(&self, local_path: &Path, uri: &str) -> Result<()> {
        trace!("uploading {} to {}", local_path.display(), uri);

        let (bucket, prefix) = parse_gs_uri(uri)?;

        // Walk the directory and upload each file
        let entries = glob::glob(&format!("{}**/*", local_path.display()))
            .context("failed to read directory pattern")?;

        for entry in entries {
            let entry = entry?;
            if !entry.is_file() {
                continue;
            }

            // Calculate relative path and object name
            let relative = entry
                .strip_prefix(local_path)
                .context("failed to strip prefix")?;
            let object_name = if prefix.is_empty() {
                relative.to_str().unwrap().to_string()
            } else {
                format!(
                    "{}/{}",
                    prefix.trim_end_matches('/'),
                    relative.to_str().unwrap()
                )
            };

            self.upload_file(&bucket, &object_name, &entry)?;
        }

        Ok(())
    }
}

impl GoogleCloudStorage {
    /// Download a single file from GCS to local disk.
    fn download_file(
        &self,
        bucket: &str,
        object: &str,
        local_path: &Path,
    ) -> Result<()> {
        run_async(async {
            // Create parent directory if needed
            if let Some(parent) = local_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .context("cannot create local download directory")?;
            }

            // Format bucket name with projects/_/buckets/ prefix
            let bucket_path = format!("projects/_/buckets/{}", bucket);

            // Download with automatic decompression enabled - this only affects files
            // with Content-Encoding: gzip, not files that are natively .gz format
            let mut reader = self
                .client
                .read_object(&bucket_path, object)
                .with_automatic_decompression(true)
                .send()
                .await?;

            // Write to file
            let mut file = tokio::fs::File::create(local_path).await?;
            while let Some(chunk) = reader.next().await {
                let chunk = chunk?;
                tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            }

            Ok(())
        })
    }

    /// Upload a single file to GCS.
    fn upload_file(
        &self,
        bucket: &str,
        object: &str,
        local_path: &Path,
    ) -> Result<()> {
        run_async(async {
            let data = std::fs::read(local_path)
                .with_context(|| format!("failed to read {}", local_path.display()))?;

            // Format bucket name with projects/_/buckets/ prefix
            let bucket_path = format!("projects/_/buckets/{}", bucket);

            self.client
                .write_object(&bucket_path, object, bytes::Bytes::from(data))
                .send_buffered()
                .await?;

            Ok(())
        })
    }
}
