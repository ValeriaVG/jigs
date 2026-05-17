//! JSON string escaping utility shared by rendering crates.

use std::fmt::Write;

/// Append a JSON-escaped, double-quoted string to `out`.
///
/// Escapes the usual JSON characters (`"`, `\`, `\n`, `\r`, `\t`, and
/// control characters below U+0020). When `escape_html` is true,
/// also escapes `<` as `\u003c` to prevent `</script>` injection when
/// embedding JSON in HTML `<script>` tags.
pub fn push_json_str(out: &mut String, s: &str, escape_html: bool) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' if escape_html => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
