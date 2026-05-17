use std::fmt::Write;

use jigs_trace::Entry;

/// Render entries as a depth-indented tree with status mark and duration.
#[must_use]
pub fn render_tree(entries: &[Entry]) -> String {
    let labels: Vec<String> = entries
        .iter()
        .map(|e| {
            let indent = if e.depth == 0 {
                String::new()
            } else {
                format!("{}└─ ", "  ".repeat(e.depth - 1))
            };
            format!("{}{}", indent, e.name())
        })
        .collect();
    let width = labels.iter().map(|l| l.chars().count()).max().unwrap_or(0);

    let mut out = String::new();
    for (label, e) in labels.iter().zip(entries) {
        let pad = width - label.chars().count();
        let mark = if e.ok { "ok" } else { "err" };
        let detail = match &e.error {
            Some(msg) => format!("ERROR: {msg}"),
            None => format!("{:?}", e.duration),
        };
        let _ = writeln!(out, "{}{}  {}  {}", label, " ".repeat(pad), mark, detail);
    }
    out
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
            Duration::from_micros(100),
            ok,
            err.map(|s| s.to_string()),
        )
    }

    #[test]
    fn empty_input_renders_empty_string() {
        assert_eq!(render_tree(&[]), "");
    }

    #[test]
    fn nested_entries_indent_by_depth() {
        let entries = [
            entry("handle", 0, true, None),
            entry("inner", 1, true, None),
        ];
        let out = render_tree(&entries);
        assert!(out.contains("handle"));
        let inner_line = out.lines().nth(1).unwrap();
        assert!(inner_line.starts_with("└─ inner"));
    }

    #[test]
    fn errors_are_rendered_with_message() {
        let entries = [entry("step", 0, false, Some("boom"))];
        let out = render_tree(&entries);
        assert!(out.contains("err"));
        assert!(out.contains("ERROR: boom"));
    }
}
