//! Secrets used to access various resources.

/// A Kubernetes-managed secret used to access some resource, and how we should
/// map it into a container. Kubernetes secrets contain key-value pairs.
///
/// Note that this is used directly as part of the `PipelineSpec` format, so it
/// can't be changed without breaking a user-facing file format.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum Secret {
    /// A secret that should be mounted as a directory of files.
    Mount {
        /// The name of the Kubernetes secret to use.
        name: String,
        /// The directory path to mount it as.
        mount_path: String,
    },

    /// A secret which should have a single key extracted and mapped to
    /// an environment variable.
    Env {
        /// The name of the Kubernetes secret to use.
        name: String,
        /// The key within the secret to use.
        key: String,
        /// The environment variable name into which to place the value.
        env_var: String,
    },
}
