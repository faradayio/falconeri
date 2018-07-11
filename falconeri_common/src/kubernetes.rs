//! Tools for talking to Kubernetes.

use failure::ResultExt;
use serde::de::DeserializeOwned;
use serde_json;
use std::{io::Write, process::{Command, Stdio}};

use Result;

/// Run `kubectl`, passing any output through to the console.
pub fn kubectl(args: &[&str]) -> Result<()> {
    let status = Command::new("kubectl")
        .args(args)
        .status()
        .with_context(|_| format!("error starting kubectl with {:?}", args))?;
    if status.success() {
        Ok(())
    } else {
        Err(format_err!("error running kubectl with {:?}", args))
    }
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
    child.wait()
        .with_context(|_| format!("error running kubectl with {:?}", args))?;
    Ok(())
}

/// Deploy a manifest to our Kubernetes cluster.
pub fn deploy(manifest: &str) -> Result<()> {
    kubectl_with_input(&["apply", "-f", "-"], manifest)
}

/// Delete all resources specified in the manifest from our Kubernetes cluster.
pub fn undeploy(manifest: &str) -> Result<()> {
    kubectl_with_input(&["delete", "-f", "-"], manifest)
}

/// Delete the specified Kubernetes resource.
pub fn delete(resource_id: &str) -> Result<()> {
    kubectl(&["delete", resource_id])
}

