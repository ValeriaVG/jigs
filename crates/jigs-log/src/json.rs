use jigs_trace::Entry;

/// Render entries as newline-delimited JSON, one object per line.
///
/// Each object has fields: `name`, `depth`, `duration_ns`, `ok`, and `error`
/// (omitted when absent).
#[must_use]
pub fn render_ndjson(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        out.push('{');
        push_field_str(&mut out, "name", e.name());
        out.push(',');
        push_field_num(&mut out, "depth", e.depth as u128);
        out.push(',');
        push_field_num(&mut out, "duration_ns", e.duration.as_nanos());
        out.push(',');
        push_field_bool(&mut out, "ok", e.ok);
        if let Some(err) = &e.error {
            out.push(',');
            push_field_str(&mut out, "error", err);
        }
        out.push_str("}\n");
    }
    out
}

fn push_field_str(out: &mut String, key: &str, value: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    jigs_core::json::push_json_str(out, value, false);
}

fn push_field_num(out: &mut String, key: &str, value: u128) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

fn push_field_bool(out: &mut String, key: &str, value: bool) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":");
    out.push_str(if value { "true" } else { "false" });
}

#[cfg(test)]
mod tests {
    use super::*;
    use jigs_core::JigMeta;
    use std::time::Duration;

    fn meta(name: &'static str) -> &'static JigMeta {
        Box::leak(Box::new(JigMeta {
            name,
            file: "",
            line: 0,
            kind: "Response",
            input: "Request",
            input_type: "",
            output_type: "",
            is_async: false,
            module: "",
            chain: &[],
        }))
    }

    fn entry(name: &'static str, depth: usize, ok: bool, err: Option<&str>) -> Entry {
        Entry::new(
            meta(name),
            depth,
            Duration::from_nanos(1_500),
            ok,
            err.map(|s| s.to_string()),
        )
    }

    #[test]
    fn empty_input_renders_empty_string() {
        assert_eq!(render_ndjson(&[]), "");
    }

    #[test]
    fn one_entry_per_line_with_required_fields() {
        let entries = [entry("step", 2, true, None)];
        let out = render_ndjson(&entries);
        assert_eq!(
            out,
            "{\"name\":\"step\",\"depth\":2,\"duration_ns\":1500,\"ok\":true}\n"
        );
    }

    #[test]
    fn errors_are_included_and_escaped() {
        let entries = [entry("bad", 0, false, Some("quote: \" and \\ slash"))];
        let out = render_ndjson(&entries);
        assert!(out.contains("\"ok\":false"));
        assert!(out.contains("\"error\":\"quote: \\\" and \\\\ slash\""));
        assert!(out.ends_with("\n"));
    }

    #[test]
    fn newlines_in_strings_are_escaped() {
        let entries = [entry("x", 0, false, Some("line1\nline2"))];
        let out = render_ndjson(&entries);
        assert!(out.contains("line1\\nline2"));
        // Exactly one physical line.
        assert_eq!(out.matches('\n').count(), 1);
    }
}
