//! Support for AWS S3 storage.

use failure::ResultExt;
use regex::Regex;
use serde_json;
use std::process;

use prefix::*;
use super::CloudStorage;

/// An S3 secret fetched from Kubernetes. This can be fetched using
/// `kubernetes_secret`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub struct S3SecretData {
    /// Our `AWS_ACCESS_KEY_ID` value.
    pub aws_access_key_id: String,
    /// Our `AWS_SECRET_ACCESS_KEY` value.
    pub aws_secret_access_key: String,
}

/// Backend for talking to AWS S3, currently based on `awscli`.
pub struct S3Storage {
    secret_data: Option<S3SecretData>,
}

impl S3Storage {
    /// Create a new `S3Storage` backend.
    pub fn new() -> Self {
        S3Storage {
            secret_data: None,
        }
    }

    /// Build a `Command` object which calls the `aws` CLI tool, including any
    /// authentication that we happen to have.
    fn aws_command(&self) -> process::Command {
        let mut command = process::Command::new("aws");
        if let Some(secret_data) = &self.secret_data {
            command.env("AWS_ACCESS_KEY_ID", &secret_data.aws_access_key_id);
            command.env("AWS_SECRET_ACCESS_KEY", &secret_data.aws_secret_access_key);
        }
        command
    }
}

impl CloudStorage for S3Storage {
    fn list(&self, uri: &str) -> Result<Vec<String>> {
        trace!("listing {}", uri);

        let (bucket, key) = parse_s3_url(uri)?;
        let mut prefix = key.to_owned();
        if key != "" && !key.ends_with("/") {
            prefix.push_str("/");
        }

        // Use `aws` to list our bucket, and parse the results.parse_s3_url(
        let output_json = self.aws_command()
            .args(&["s3api", "list-objects-v2"])
            .arg("--bucket")
            .arg(bucket)
            .arg("--prefix")
            .arg(prefix)
            .stderr(process::Stdio::inherit())
            .output()
            .context("error running gsutil")?
            .stdout;
        let output: ListObjectsV2Output = serde_json::from_slice(&output_json)
            .context("error parsing list-objects-v2 output")?;

        // Fail if the bucket has too many entries to get in one call.
        //
        // TODO: Chain together multiple calls to `list-objects-v2`.
        if output.is_truncated.unwrap_or(false) {
            return Err(format_err!(
                "S3 prefix {:?} contains too many objects for this version",
                uri,
            ));
        }

        Ok(output.contents
            .into_iter()
            // Only include files.
            .filter(|obj| !obj.key.ends_with("/"))
            // Convert to URLs.
            .map(|obj| format!("s3://{}/{}", bucket, obj.key))
            .collect::<Vec<_>>())
    }

    fn sync_down(&self, uri: &str, local_path: &Path) -> Result<()> {
        trace!("downloading {} to {}", uri, local_path.display());
        unimplemented!()
    }

    fn sync_up(&self, local_path: &Path, uri: &str) -> Result<()> {
        trace!("uploading {} to {}", local_path.display(), uri);
        unimplemented!()
    }
}

/// Parse an S3 URL.
fn parse_s3_url(url: &str) -> Result<(&str, &str)> {
    // lazy_static allows us to compile this regex only once.
    lazy_static! {
        static ref RE: Regex =
            Regex::new("^s3://(?P<bucket>[^/]+)(?:/(?P<key>.*))?$")
                .expect("couldn't parse built-in regex");
    }

    let caps = RE.captures(url)
        .ok_or_else(|| format_err!("the URL {:?} could not be parsed", url))?;
    let bucket = caps.name("bucket")
        .expect("missing hard-coded capture???")
        .as_str();
    let key = caps.name("key")
        .map(|m| m.as_str())
        .unwrap_or("");

    Ok((bucket, key))
}

#[test]
fn url_parsing() {
    assert_eq!(parse_s3_url("s3://top-level").unwrap(), ("top-level", ""));
    assert_eq!(parse_s3_url("s3://top-level/").unwrap(), ("top-level", ""));
    assert_eq!(parse_s3_url("s3://top-level/path").unwrap(), ("top-level", "path"));
    assert_eq!(parse_s3_url("s3://top-level/path/").unwrap(), ("top-level", "path/"));
    assert!(parse_s3_url("gs://foo/").is_err());
}

/// Local, `serde`-compatible reimplementation of
/// [`rusoto_s3::ListObjectsV2Output`][rusoto].
///
/// [rusoto]:
/// https://rusoto.github.io/rusoto/rusoto_s3/struct.ListObjectsV2Output.html
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListObjectsV2Output {
    #[serde(default)]
    contents: Vec<Object>,
    is_truncated: Option<bool>,
}

/// Local, `serde`-compatible reimplementation of [`rusoto_s3::Output`][rusoto].
///
/// [rusoto]: https://rusoto.github.io/rusoto/rusoto_s3/struct.Object.html
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Object {
    key: String,
    size: i64,
}
