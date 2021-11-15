//! Tools for manipulating Kubernetes manifests.

use handlebars::Handlebars;

use crate::prelude::*;

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

    // Default escaping assumes we're in quote string.
    //handlebars.register_escape_fn(...)
    handlebars.register_escape_fn(yaml_escape);

    // Render our template and deploy it.
    handlebars
        .render_template(template_yml, params)
        .context("error rendering manifest template")
}

/// Escape a string as a YAML value. We assume the value we're escaping is
/// quoted in the YAML.
///
/// See  http://yaml.org/spec/1.2/spec.html#id2776092 for details.
fn yaml_escape(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '\"' => result.push_str("\\\""),
            '\'' => result.push_str("\\\'"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\0' => result.push_str("\\0"),
            ' ' => result.push(' '),
            c if c.is_ascii_graphic() => result.push(c),
            c if c as u32 <= 0xFFFF => {
                result.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => result.push_str(&format!("\\U{:08x}", c as u32)),
        }
    }
    result
}

#[test]
fn yaml_escape_handles_common_chars() {
    let examples = &[
        ("abc 123", "abc 123"),
        ("=", "="),
        ("\\", "\\\\"),
        ("\"", "\\\""),
        ("\'", "\\\'"),
        ("&", "&"),
        ("\n", "\\n"),
        ("\r", "\\r"),
        ("\t", "\\t"),
        ("\0", "\\0"),
        ("\u{0007}", "\\u0007"),
        ("\u{13000}", "\\U00013000"),
    ];
    for &(input, expected) in examples {
        assert_eq!(yaml_escape(input), expected);
    }
}
