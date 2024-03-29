//! The `datum describe` subcommand.

use falconeri_common::{db, prelude::*};

use crate::description::render_description;

/// Template for human-readable `describe` output.
const DESCRIBE_TEMPLATE: &str = include_str!("describe.txt.hbs");

/// Template parameters.
#[derive(Serialize)]
struct Params {
    datum: Datum,
    input_files: Vec<InputFile>,
}

/// Run the `datum describe` subcommand.
pub fn run(id: Uuid) -> Result<()> {
    // Look up our data in the database.
    let mut conn = db::connect(ConnectVia::Proxy)?;
    let datum = Datum::find(id, &mut conn)?;
    let input_files = datum.input_files(&mut conn)?;

    // Package into a params object.
    let params = Params { datum, input_files };

    // Print the description.
    print!("{}", render_description(DESCRIBE_TEMPLATE, &params)?);
    Ok(())
}

#[test]
fn render_template() {
    let job = Job::factory();
    let datum = Datum::factory(&job);
    let input_file = InputFile::factory(&datum);
    let input_files = vec![input_file];
    let params = Params { datum, input_files };
    render_description(DESCRIBE_TEMPLATE, &params).expect("could not render template");
}
