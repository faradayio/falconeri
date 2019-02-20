//! Tools for manipulating Kubernetes manifests.

use falconeri_common::prelude::*;
use handlebars::Handlebars;

/// Render the specified YAML manifest, filling in the supplied values
/// using [Handlebars][].
///
/// [Handlebars]: https://handlebarsjs.com/
pub fn render_manifest<T: Serialize>(
    template_yml: &str,
    params: &T,
) -> Result<String> {
    // Set up handlebars.
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    // TODO: Fix escaping as per http://yaml.org/spec/1.2/spec.html#id2776092.
    //handlebars.register_escape_fn(...)

    // Render our template and deploy it.
    Ok(handlebars
        .render_template(template_yml, params)
        .context("error rendering manifest template")?)
}
