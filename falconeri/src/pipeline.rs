//! A JSON "pipeline spec" format loosely compatible with a subset of the
//! Pachyderm [Pipeline Specification][pipespec]. We implement just enough to
//! run our pre-existing Pachyderm jobs with light modification.
//!
//! [pipespec]: http://docs.pachyderm.io/en/latest/reference/pipeline_spec.html

use falconeri_common::{prefix::*, secret::Secret};

/// Represents a pipeline *.json file.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineSpec {
    pub pipeline: Pipeline,
    pub transform: Transform,
    pub parallelism_spec: ParallelismSpec,
    pub resource_requests: ResourceRequests,
    // EXTENSION: Kubernetes node selectors describing the nodes where we can
    // run this job.
    #[serde(default)]
    pub node_selector: HashMap<String, String>,
    pub input: Input,
    pub egress: Egress,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub cmd: Vec<String>,
    pub image: String,
    // TODO: We currently also use this for secrets needed to access buckets,
    // but that's not really a complete or well-thought-out solution, and we may
    // want to declare secrets as part of our `Input::Atom` values.
    #[serde(default)]
    pub secrets: Vec<Secret>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParallelismSpec {
    pub constant: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequests {
    pub memory: String,
    pub cpu: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Atom {
        // EXTENSION: URI from which to fetch input data.
        #[serde(rename = "URI")]
        uri: String,
        repo: String,
        glob: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Egress {
    #[serde(rename = "URI")]
    pub uri: String,
}

#[test]
fn parse_pipeline_spec() {
    use serde_json;

    let json = include_str!("example_pipeline_spec.json");
    let parsed: PipelineSpec = serde_json::from_str(json).expect("parse error");
    assert_eq!(parsed.pipeline.name, "book_words");
    assert_eq!(parsed.transform.cmd[0], "python3");
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
            glob: "/*".to_owned(),
        }
    );
    assert_eq!(parsed.egress.uri, "gs://example-bucket/words/");
}
