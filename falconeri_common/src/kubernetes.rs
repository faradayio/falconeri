//! Tools for talking to Kubernetes.

use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use serde::de::DeserializeOwned;
use serde_json;
use std::{iter, process::{Command, Stdio}};

use prefix::*;

/// Run `kubectl`, passing any output through to the console.
pub fn kubectl(args: &[&str]) -> Result<()> {
    let status = Command::new("kubectl")
        .args(args)
        .status()
        .with_context(|_| format!("error starting kubectl with {:?}", args))?;
    if !status.success() {
        return Err(format_err!("error running kubectl with {:?}", args));
    }
    Ok(())
}

/// Run `kubectl`, capture output as JSON, and parse it using the
/// specified type.
pub fn kubectl_parse_json<T: DeserializeOwned>(args: &[&str]) -> Result<T> {
    let output = Command::new("kubectl")
        .args(args)
        // Pass `stderr` through on console instead of capturing.
        .stderr(Stdio::inherit())
        .output()
        .with_context(|_| format!("error starting kubectl with {:?}", args))?;
    if !output.status.success() {
        return Err(format_err!("error running kubectl with {:?}", args));
    }
    Ok(serde_json::from_slice(&output.stdout)
        .with_context(|_| format!("error parsing output of kubectl {:?}", args))?)
}

/// Run `kubectl` with the specified input.
pub fn kubectl_with_input(args: &[&str], input: &str) -> Result<()> {
    let mut child = Command::new("kubectl")
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|_| format!("error starting kubectl with {:?}", args))?;
    write!(child.stdin.as_mut().expect("child stdin is missing"), "{}", input)
        .with_context(|_| format!("error writing intput to kubectl {:?}", args))?;
    let status = child.wait()
        .with_context(|_| format!("error running kubectl with {:?}", args))?;
    if !status.success() {
        return Err(format_err!("error running kubectl with {:?}", args));
    }
    Ok(())
}

/// Does `kubectl` exit successfully when called with the specified arguments?
pub fn kubectl_succeeds(args: &[&str]) -> Result<bool> {
    let output = Command::new("kubectl").args(args).output()?;
    Ok(output.status.success())
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
    iter::repeat(())
        // Note that this random distribution is biased, because we generate
        // both upper and lowercase letters and then convert to lowercase
        // later. This isn't a big deal for now.
        .map(|()| rng.sample(Alphanumeric))
        .take(5)
        .collect::<String>()
}
