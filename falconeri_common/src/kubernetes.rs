//! Tools for talking to Kubernetes.

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::de::{Deserialize, DeserializeOwned};
use serde_json;
use std::collections::HashSet;
use std::{
    env, iter,
    process::{Command, Stdio},
};

use crate::prelude::*;

/// Run `kubectl`, passing any output through to the console.
#[tracing::instrument(level = "trace")]
pub fn kubectl(args: &[&str]) -> Result<()> {
    let status = Command::new("kubectl")
        .args(args)
        .status()
        .with_context(|| format!("error starting kubectl with {:?}", args))?;
    if !status.success() {
        return Err(format_err!("error running kubectl with {:?}", args));
    }
    Ok(())
}

/// Run `kubectl`, capture output as JSON, and parse it using the
/// specified type.
#[tracing::instrument(level = "trace")]
pub fn kubectl_parse_json<T: DeserializeOwned>(args: &[&str]) -> Result<T> {
    let output = Command::new("kubectl")
        .args(args)
        // Pass `stderr` through on console instead of capturing.
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("error starting kubectl with {:?}", args))?;
    if !output.status.success() {
        return Err(format_err!("error running kubectl with {:?}", args));
    }
    serde_json::from_slice(&output.stdout)
        .with_context(|| format!("error parsing output of kubectl {:?}", args))
}

/// Run `kubectl` with the specified input.
#[tracing::instrument(level = "trace")]
pub fn kubectl_with_input(args: &[&str], input: &str) -> Result<()> {
    let mut child = Command::new("kubectl")
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("error starting kubectl with {:?}", args))?;
    write!(
        child.stdin.as_mut().expect("child stdin is missing"),
        "{}",
        input
    )
    .with_context(|| format!("error writing intput to kubectl {:?}", args))?;
    let status = child
        .wait()
        .with_context(|| format!("error running kubectl with {:?}", args))?;
    if !status.success() {
        return Err(format_err!("error running kubectl with {:?}", args));
    }
    Ok(())
}

/// Does `kubectl` exit successfully when called with the specified arguments?
#[tracing::instrument(level = "trace")]
pub fn kubectl_succeeds(args: &[&str]) -> Result<bool> {
    let output = Command::new("kubectl").args(args).output()?;
    Ok(output.status.success())
}

/// A Kubernetes secret (missing lots of fields).
#[derive(Debug, Deserialize)]
struct Secret<T> {
    /// Our secret data.
    ///
    /// We use some [serde magic][] to deserialize a parameterized type.
    ///
    /// [serde magic]: https://serde.rs/attr-bound.html
    #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    data: T,
}

/// Custom `serde` (de)serialization module for Base64-encoded strings. Use
/// with `#[serde(with = "base64_encoded_secret_string")]` to automatically
/// decode Base64-encoded fields.
pub mod base64_encoded_secret_string {
    use base64;
    use serde::de::{Deserialize, Deserializer, Error as DeError};
    use std::result;

    /// Deserialize a secret represented as a Base64-encoded UTF-8 string.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> result::Result<String, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        let bytes = base64::decode(&encoded).map_err(|err| {
            D::Error::custom(format!("could not base64-decode secret: {}", err))
        })?;
        let decoded = String::from_utf8(bytes).map_err(|err| {
            D::Error::custom(format!("could not UTF-8-decode secret: {}", err))
        })?;
        Ok(decoded)
    }
}

/// Fetch a secret and deserialize it as the specified type.
#[tracing::instrument(level = "trace")]
pub fn kubectl_secret<T: DeserializeOwned>(secret: &str) -> Result<T> {
    let secret: Secret<T> =
        kubectl_parse_json(&["get", "secret", secret, "-o", "json"])?;
    Ok(secret.data)
}

/// A list of items returned by Kubernetes.
#[derive(Deserialize)]
struct ItemsJson<T> {
    items: Vec<T>,
}

/// JSON describing a pod or similar resource.
#[derive(Deserialize)]
struct ResourceJson {
    // Kubernetes resource metadata.
    metadata: Option<MetadataJson>,
}

impl ResourceJson {
    /// Get the name of this resource, if any.
    fn name(&self) -> Option<&str> {
        let s = self.metadata.as_ref()?.name.as_ref()?;
        Some(&s[..])
    }
}
/// JSON describing resource metadata.
#[derive(Deserialize)]
struct MetadataJson {
    /// Resource name.
    name: Option<String>,
}

/// Get a set of currently running pod names.
#[tracing::instrument(level = "trace")]
pub fn get_running_pod_names() -> Result<HashSet<String>> {
    let pods = kubectl_parse_json::<ItemsJson<ResourceJson>>(&[
        "get",
        "pods",
        "--field-selector",
        "status.phase=Running",
        "--output=json",
    ])?;

    let mut names = HashSet::new();
    for pod in &pods.items {
        if let Some(name) = pod.name() {
            names.insert(name.to_owned());
        } else {
            warn!("found nameless pod");
        }
    }
    debug!("found {} running pods", names.len());
    trace!("running pods: {:?}", names);
    Ok(names)
}

/// Get a set of all job names present on the cluster.
#[tracing::instrument(level = "trace")]
pub fn get_all_job_names() -> Result<HashSet<String>> {
    let pods = kubectl_parse_json::<ItemsJson<ResourceJson>>(&[
        "get",
        "jobs",
        "--output=json",
    ])?;

    let mut names = HashSet::new();
    for pod in &pods.items {
        if let Some(name) = pod.name() {
            names.insert(name.to_owned());
        } else {
            warn!("found nameless job");
        }
    }
    debug!("found {} jobs", names.len());
    trace!("jobs: {:?}", names);
    Ok(names)
}

/// Deploy a manifest to our Kubernetes cluster.
pub fn deploy(manifest: &str) -> Result<()> {
    kubectl_with_input(&["apply", "-f", "-"], manifest)
}

/// Delete all resources specified in the manifest from our Kubernetes cluster.
pub fn undeploy(manifest: &str) -> Result<()> {
    kubectl_with_input(&["delete", "-f", "-"], manifest)
}

/// Does the specified resource exist?
pub fn resource_exists(resource_id: &str) -> Result<bool> {
    kubectl_succeeds(&["get", resource_id])
}

/// Delete the specified Kubernetes resource.
pub fn delete(resource_id: &str) -> Result<()> {
    kubectl(&["delete", resource_id])
}

/// Generate a hopefully unique tag for a Kubernetes resource. To keep
/// Kubernetes happy, this must be a legal DNS name component (but we have a
/// database constraint to enforce that).
pub fn resource_tag() -> String {
    let mut rng = thread_rng();
    let bytes = iter::repeat(())
        // Note that this random distribution is biased, because we generate
        // both upper and lowercase letters and then convert to lowercase
        // later. This isn't a big deal for now.
        .map(|()| rng.sample(Alphanumeric))
        .take(5)
        .collect::<Vec<u8>>();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Get the name of the current Kubernetes node.
pub fn node_name() -> Result<String> {
    env::var("FALCONERI_NODE_NAME").context("couldn't get FALCONERI_NODE_NAME")
}

/// Get the name of the current Kubernetes pod.
pub fn pod_name() -> Result<String> {
    env::var("FALCONERI_POD_NAME").context("couldn't get FALCONERI_POD_NAME")
}
