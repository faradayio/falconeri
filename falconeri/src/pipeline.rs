//! A JSON "pipeline spec" format loosely compatible with a subset of the
//! Pachyderm [Pipeline Specification][pipespec]. We implement just enough to
//! run our pre-existing Pachyderm jobs with light modification.
//!
//! [pipespec]: http://docs.pachyderm.io/en/latest/reference/pipeline_spec.html

use std::collections::HashMap;

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

    let json = r#"
{
  "pipeline": {
    "name": "book_words"
  },
  "transform": {
    "cmd": [ "python3", "/extract_words.py" ],
    "image": "somerepo/my_python_nlp"
  },
  "parallelism_spec": {
    "constant": 10
  },
  "resource_requests": {
    "memory": "500Mi",
    "cpu": 1.2
  },
  "node_selector": {
      "my_node_type": "falconeri_worker"
  },
  "input": {
    "atom": {
      "URI": "gs://example-bucket/books/",
      "repo": "books",
      "glob": "/*"
    }
  },
  "egress": {
    "URI": "gs://example-bucket/words/"
  }
}"#;

    let parsed: PipelineSpec =
        serde_json::from_str(json).expect("parse error");
    assert_eq!(parsed.pipeline.name, "book_words");
    assert_eq!(parsed.transform.cmd[0], "python3");
    assert_eq!(parsed.parallelism_spec.constant, 10);
    assert_eq!(parsed.resource_requests.memory, "500Mi");
    assert_eq!(parsed.resource_requests.cpu, 1.2);
    assert_eq!(parsed.node_selector["my_node_type"], "falconeri_worker");
    assert_eq!(parsed.transform.image, "somerepo/my_python_nlp");
    assert_eq!(parsed.input, Input::Atom {
        uri: "gs://example-bucket/books/".to_owned(),
        repo: "books".to_owned(),
        glob: "/*".to_owned(),
    });
    assert_eq!(parsed.egress.uri, "gs://example-bucket/words/");
}
