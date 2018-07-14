//! The `datum describe` subcommand.

use falconeri_common::{db, models::*, Result};
use uuid::Uuid;

use description::print_description;

/// Template for human-readable `describe` output.
const DESCRIBE_TEMPLATE: &str = include_str!("describe.txt.hbs");

/// Run the `datum describe` subcommand.
pub fn run(id: Uuid) -> Result<()> {
    // Look up our data in the database.
    let conn = db::connect(db::ConnectVia::Proxy)?;
    let datum = Datum::find(id, &conn)?;
    let input_files = datum.input_files(&conn)?;

    // Package into a params object.
    #[derive(Serialize)]
    struct Params {
        datum: Datum,
        input_files: Vec<InputFile>,
    }
    let params = Params { datum, input_files };

    // Print the description.
    print_description(DESCRIBE_TEMPLATE, &params)
}
