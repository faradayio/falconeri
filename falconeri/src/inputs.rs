//! Convert JSON `"input"` clauses to datums which will be assigned to workers.

use falconeri_common::{
    models::{NewDatum, NewInputFile},
    prelude::*,
    secret::Secret,
    storage::CloudStorage,
};

use crate::pipeline::{Glob, Input};

/// (Local helper type.) This is essentially just a `NewDatum` and a
/// `Vec<NewInputFile>`, but in a more convenient format that works better with
/// the algorithm in this file, so we don't need to carry around UUIDs
/// everywhere.
#[derive(Clone, Debug)]
struct DatumData {
    input_files: Vec<InputFileData>,
}

impl DatumData {
    /// Convert this into an actual `NewDatum` and a `Vec<NewInputFile>`.
    fn into_new_datum_and_input_files(
        self,
        job_id: Uuid,
    ) -> (NewDatum, Vec<NewInputFile>) {
        let datum_id = Uuid::new_v4();
        let datum = NewDatum {
            id: datum_id,
            job_id,
        };
        let input_files = self
            .input_files
            .into_iter()
            .map(|f| f.into_new_input_file(job_id, datum_id))
            .collect();
        (datum, input_files)
    }
}

/// (Local helper type.) This is essentially a `NewInputFile`, but in a more
/// convenient format.
#[derive(Clone, Debug)]
struct InputFileData {
    uri: String,
    local_path: String,
}

impl InputFileData {
    /// Convert this into an actual `NewInputFile`.
    fn into_new_input_file(self, job_id: Uuid, datum_id: Uuid) -> NewInputFile {
        NewInputFile {
            job_id,
            datum_id,
            uri: self.uri,
            local_path: self.local_path,
        }
    }
}

/// Given an `Input` from a JSON pipeline spec, convert to an actual set of
/// "datums" (work chunks) to be assigned to a worker.
///
/// Returns the datums and associated input files in a form well-suited to bulk
/// database insert.
pub fn input_to_datums(
    secrets: &[Secret],
    job_id: Uuid,
    input: &Input,
) -> Result<(Vec<NewDatum>, Vec<NewInputFile>)> {
    let mut all_datums = vec![];
    let mut all_input_files = vec![];
    for datum_data in input_to_datums_helper(secrets, input)? {
        let (datum, input_files) = datum_data.into_new_datum_and_input_files(job_id);
        all_datums.push(datum);
        all_input_files.extend(input_files);
    }
    Ok((all_datums, all_input_files))
}

/// Given an `Input` from a JSON pipeline spec, convert to an actual set of
/// "datums" (work chunks) to be assigned to a worker.
///
/// This is the internal helper version of `input_to_datums` that works on the
/// simpler `DatumData` instead of database-ready `NewDatum` records.
fn input_to_datums_helper(
    secrets: &[Secret],
    input: &Input,
) -> Result<Vec<DatumData>> {
    match input {
        Input::Atom { uri, repo, glob } => {
            atom_to_datums_helper(secrets, uri, repo, *glob)
        }
        Input::Cross(inputs) => cross_to_datums_helper(secrets, inputs),
        Input::Union(inputs) => {
            // Merge all our inputs. We could do this cleverly using `flat_map`
            // and `collect` to manage the errors, but it's clearer with a `for`
            // loop.
            let mut datums = vec![];
            for child in inputs {
                datums.extend(input_to_datums_helper(secrets, child)?);
            }
            Ok(datums)
        }
    }
}

/// Convert a single `Input::Atom` to a list of datums.
fn atom_to_datums_helper(
    secrets: &[Secret],
    uri: &str,
    repo: &str,
    glob: Glob,
) -> Result<Vec<DatumData>> {
    // Normalize our URI to always include a slash, because repositories must
    // currently be directories.
    let mut base = uri.to_owned();
    if !base.ends_with('/') {
        base.push_str("/");
    }

    // Figure out what files to process. We do this for _both_
    // `Glob::TopLevelDirectoryEntries` and `Glob::WholeRepo`, because we want
    // to verify that we can actually list the contents of a `Glob::WholeRepo`
    // _before_ spinning up a big cluster job.
    let storage = CloudStorage::for_uri(&uri, secrets)?;
    let file_uris = storage.list(uri)?;

    match glob {
        // Our input file is just the entire repo, as a directory.
        Glob::WholeRepo => Ok(vec![DatumData {
            input_files: vec![InputFileData {
                uri: base,
                local_path: format!("/pfs/{}", repo),
            }],
        }]),

        // Each top-level file or directory in `base` should be translated into
        // a separate datum.
        Glob::TopLevelDirectoryEntries => {
            let mut datums = vec![];
            for file_uri in file_uris {
                let local_path = uri_to_local_path(&file_uri, repo)?;
                datums.push(DatumData {
                    input_files: vec![InputFileData {
                        uri: file_uri,
                        local_path,
                    }],
                });
            }
            Ok(datums)
        }
    }
}

/// Convert a cross product into a list of datums.
///
/// SECURITY: This assumes it runs on reasonably trusted and plausible inputs.
/// You can cause a denial-of-service by calculating the cross product of
/// enormous repos, or by passing in so many repos that the stack overflows. But
/// since our input comes from a local user, this is fine for now.
fn cross_to_datums_helper(
    secrets: &[Secret],
    inputs: &[Input],
) -> Result<Vec<DatumData>> {
    match inputs.len() {
        // Base cases.
        0 => Ok(vec![]),
        1 => input_to_datums_helper(secrets, &inputs[0]),

        // Recursive case.
        n => {
            // Recursively calculate the cross product of all but our last input.
            let datums_0 = cross_to_datums_helper(secrets, &inputs[0..n - 1])?;

            // Process our last input.
            let datums_1 = input_to_datums_helper(secrets, &inputs[n - 1])?;

            // Build our cross product between the recursive `datums_0` and our
            // local `datums_1`.
            let mut output = vec![];
            for datum_0 in &datums_0 {
                for datum_1 in &datums_1 {
                    let input_files_0 = &datum_0.input_files;
                    let input_files_1 = &datum_1.input_files;
                    let len_0 = input_files_0.len();
                    let len_1 = input_files_1.len();
                    let mut combined = Vec::with_capacity(len_0 + len_1);
                    combined.extend(input_files_0.iter().cloned());
                    combined.extend(input_files_1.iter().cloned());
                    output.push(DatumData {
                        input_files: combined,
                    })
                }
            }
            Ok(output)
        }
    }
}

/// Given a URI and a repo name, construct a local path starting with "/pfs"
/// pointing to where we should download the file.
///
/// TODO: This will need to get fancier if we actually implement globs
/// correctly.
fn uri_to_local_path(uri: &str, repo: &str) -> Result<String> {
    let pos = uri
        .rfind('/')
        .ok_or_else(|| format_err!("No '/' in {:?}", uri))?;
    let basename = &uri[pos..];
    if basename.is_empty() {
        Err(format_err!("{:?} ends with '/'", uri))
    } else {
        Ok(format!("/pfs/{}{}", repo, basename))
    }
}

#[test]
fn uri_to_local_path_works() {
    let path = uri_to_local_path("gs://bucket/path/data1.csv", "myrepo").unwrap();
    assert_eq!(path, "/pfs/myrepo/data1.csv");
}
