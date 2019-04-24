//! A JSON "pipeline spec" format loosely compatible with a subset of the
//! Pachyderm [Pipeline Specification][pipespec]. We implement just enough to
//! run our pre-existing Pachyderm jobs with light modification.
//!
//! [pipespec]: http://docs.pachyderm.io/en/latest/reference/pipeline_spec.html

use falconeri_common::{prelude::*, secret::Secret};

/// Represents a pipeline `*.json` file.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineSpec {
    /// Metadata about this pipeline.
    pub pipeline: Pipeline,
    /// Instructions on how to transform the data.
    pub transform: Transform,
    /// How much parallelism should we use?
    pub parallelism_spec: ParallelismSpec,
    /// How many resources should we allocate for each worker?
    pub resource_requests: ResourceRequests,
    /// EXTENSION: Kubernetes node selectors describing the nodes where we can
    /// run this job.
    #[serde(default)]
    pub node_selector: HashMap<String, String>,
    /// Specify our input data.
    pub input: Input,
    /// Where to put the data when we're done with it.
    pub egress: Egress,
}

/// Metadata about this pipeline.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
    /// The name of this pipeline. Also may be used to default various things.
    pub name: String,
}

/// Instructions on how to transform the data.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    /// The command to run, with arguments.
    pub cmd: Vec<String>,
    /// The Docker image to run.
    pub image: String,
    /// EXTENSION: When should we pull this image?
    pub image_pull_policy: Option<String>,
    /// Extra environment variables to pass in.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Kubernetes secrets to make available to our Docker containers.
    ///
    /// TODO: We currently also use this for secrets needed to access buckets,
    /// but that's not really a complete or well-thought-out solution, and we may
    /// want to declare secrets as part of our `Input::Atom` values.
    #[serde(default)]
    pub secrets: Vec<Secret>,
    /// The Kubernetes service account to use for this job.
    pub service_account: Option<String>,
}

/// How much parallelism should we use?
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParallelismSpec {
    /// The number of workers to run.
    pub constant: u32,
}

/// How many resources should we allocate for each worker?
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequests {
    /// The amount of memory to allocate for each worker. A hard limit. Uses
    /// standard `docker-compose` memory strings like `"200M"` (I think).
    pub memory: String,
    /// The amount of CPU to allocate for each worker. A soft limit; we can go
    /// above if more CPU is available.
    pub cpu: f32,
}

/// Specify our input data.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    /// Input from a cloud storage bucket.
    #[serde(alias = "pfs")]
    Atom {
        /// EXTENSION: URI from which to fetch input data.
        #[serde(rename = "URI")]
        uri: String,
        /// The repo name, used as to construct a path of the form
        /// `/pfs/$repo/`, which will be used to hold the downloaded data.
        repo: String,
        /// How to distribute the files in the repo over our workers.
        glob: Glob,
    },
    /// Cross product of two other inputs, producing every possible combination.
    Cross(Vec<Input>),
    /// Union of two other inputs
    Union(Vec<Input>),
}

/// How to distribute files from an input across workers. We only support two
/// kinds of glob patterns for now.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Glob {
    /// Put each top-level directory entry (file, subdir) its own datum.
    #[serde(rename = "/*")]
    TopLevelDirectoryEntries,

    /// Put the entire repo in a single datum.
    #[serde(rename = "/")]
    WholeRepo,
}

/// Where to put the data when we're done with it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Egress {
    /// A cloud bucket URI in which to place our output data.
    #[serde(rename = "URI")]
    pub uri: String,
}

#[test]
fn parse_nested_inputs() {
    let json = r#"
{
    "cross": [{
        "pfs": {
            "URI": "gs://example-bucket/dewey-decimal-categories/",
            "repo": "dewey-decimal-categories",
            "glob": "/"
        }
    }, {
        "union": [{
            "atom": {
                "URI": "gs://example-bucket/books/",
                "repo": "books",
                "glob": "/*"
            }
        }, {
            "atom": {
                "URI": "gs://example-bucket/more-books/",
                "repo": "more-books",
                "glob": "/*"
            }
        }]
    }]
}
"#;
    let parsed: Input = serde_json::from_str(json).expect("parse error");
    let expected = Input::Cross(vec![
        Input::Atom {
            uri: "gs://example-bucket/dewey-decimal-categories/".to_owned(),
            repo: "dewey-decimal-categories".to_owned(),
            glob: Glob::WholeRepo,
        },
        Input::Union(vec![
            Input::Atom {
                uri: "gs://example-bucket/books/".to_owned(),
                repo: "books".to_owned(),
                glob: Glob::TopLevelDirectoryEntries,
            },
            Input::Atom {
                uri: "gs://example-bucket/more-books/".to_owned(),
                repo: "more-books".to_owned(),
                glob: Glob::TopLevelDirectoryEntries,
            },
        ]),
    ]);
    assert_eq!(parsed, expected);
}

#[test]
fn parse_pipeline_spec() {
    use serde_json;

    let json = include_str!("example_pipeline_spec.json");
    let parsed: PipelineSpec = serde_json::from_str(json).expect("parse error");
    assert_eq!(parsed.pipeline.name, "book_words");
    assert_eq!(parsed.transform.cmd[0], "python3");
    assert_eq!(parsed.transform.env.get("VARNAME").unwrap(), "value");
    assert_eq!(parsed.transform.secrets.len(), 2);
    assert_eq!(
        parsed.transform.secrets[0],
        Secret::Mount {
            name: "ssl".to_owned(),
            mount_path: "/ssl".to_owned(),
        },
    );
    assert_eq!(
        parsed.transform.secrets[1],
        Secret::Env {
            name: "s3".to_owned(),
            key: "AWS_ACCESS_KEY_ID".to_owned(),
            env_var: "AWS_ACCESS_KEY_ID".to_owned(),
        },
    );
    assert_eq!(
        parsed.transform.service_account,
        Some("example-service".to_owned()),
    );
    assert_eq!(parsed.parallelism_spec.constant, 10);
    assert_eq!(parsed.resource_requests.memory, "500Mi");
    assert_eq!(parsed.resource_requests.cpu, 1.2);
    assert_eq!(parsed.node_selector["node_type"], "falconeri_worker");
    assert_eq!(parsed.transform.image, "somerepo/my_python_nlp");
    assert_eq!(
        parsed.input,
        Input::Atom {
            uri: "gs://example-bucket/books/".to_owned(),
            repo: "books".to_owned(),
            glob: Glob::TopLevelDirectoryEntries,
        }
    );
    assert_eq!(parsed.egress.uri, "gs://example-bucket/words/");
}
