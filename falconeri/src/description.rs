//! Human-readable descriptions of an object.

use falconeri_common::Result;
use handlebars::Handlebars;
use serde::Serialize;

/// Render the specified textual description, filling in the supplied values
/// using [Handlebars][].
///
/// [Handlebars]: https://handlebarsjs.com/
pub fn render_description<T: Serialize>(
    template: &str,
    params: &T,
) -> Result<String> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    Ok(handlebars.render_template(template, &params)?)
}
