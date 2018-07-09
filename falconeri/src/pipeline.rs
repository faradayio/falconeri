//! A JSON "pipeline spec" format loosely compatible with a subset of the
//! Pachyderm [Pipeline Specification][pipespec]. We implement just enough to
//! run our pre-existing Pachyderm jobs with light modification.
//!
//! [pipespec]: http://docs.pachyderm.io/en/latest/reference/pipeline_spec.html

/// Represents a pipeline *.json file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineSpec {
    pub pipeline: PipelineInfo,
    pub transform: TransformInfo,
    pub input: InputInfo,
    pub egress: EgressInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineInfo {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransformInfo {
    pub cmd: Vec<String>,
    pub image: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InputInfo {
    Atom {
        uri: String,
        repo: String,
        glob: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct EgressInfo {
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
  "input": {
    "atom": {
      "uri": "gs://example-bucket/books/",
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
    assert_eq!(parsed.transform.image, "somerepo/my_python_nlp");
    assert_eq!(parsed.input, InputInfo::Atom {
        uri: "gs://example-bucket/books/".to_owned(),
        repo: "books".to_owned(),
        glob: "/*".to_owned(),
    });
    assert_eq!(parsed.egress.uri, "gs://example-bucket/words/");
}
