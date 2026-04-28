//! Render the live `JigMeta` inventory as a single self-contained HTML page.

use jigs_core::JigMeta;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const TEMPLATE: &str = include_str!("template.html");

/// Render the pipeline rooted at `entry` (or the alphabetically first jig if
/// `None`) as a complete HTML document. `title` is shown in the page header
/// and `<title>` tag.
///
/// `editor` is an optional URL template containing `{line}` plus either
/// `{path}` (absolute file path, for local IDE handlers) or `{rel_path}`
/// (path relative to the workspace root, for repo URLs). When set, the
/// sidebar's file location becomes a link using the resolved template;
/// when `None`, it renders as plain text. Common templates:
///
/// - VS Code / Cursor / Windsurf: `vscode://file/{path}:{line}`
/// - VSCodium: `vscodium://file/{path}:{line}`
/// - JetBrains IDEs: `idea://open?file={path}&line={line}`
/// - Sublime Text: `subl://{path}:{line}`
/// - TextMate: `txmt://open/?url=file://{path}&line={line}`
/// - GitHub: `https://github.com/OWNER/REPO/blob/main/{rel_path}#L{line}`
pub fn to_html(entry: Option<&str>, title: &str, editor: Option<&str>) -> String {
    let all: BTreeMap<&'static str, &'static JigMeta> =
        jigs_core::all_jigs().map(|m| (m.name, m)).collect();
    let entry = entry
        .map(str::to_string)
        .or_else(|| all.keys().next().map(|s| s.to_string()))
        .unwrap_or_default();
    let visible = reachable(&all, &entry);
    let data = encode(&visible, &entry, title, editor);
    TEMPLATE
        .replace("__TITLE__", &esc_attr(title))
        .replace("__DATA__", &data)
}

fn reachable(
    all: &BTreeMap<&'static str, &'static JigMeta>,
    entry: &str,
) -> BTreeMap<String, &'static JigMeta> {
    let mut out = BTreeMap::new();
    let mut stack = vec![entry.to_string()];
    while let Some(name) = stack.pop() {
        if out.contains_key(&name) {
            continue;
        }
        if let Some(m) = all.get(name.as_str()) {
            for c in m.chain {
                stack.push(c.name.to_string());
            }
            out.insert(name, *m);
        }
    }
    out
}

fn encode(
    visible: &BTreeMap<String, &'static JigMeta>,
    entry: &str,
    title: &str,
    editor: Option<&str>,
) -> String {
    let mut s = String::new();
    s.push_str("{\"entry\":");
    push_json_str(&mut s, entry);
    s.push_str(",\"title\":");
    push_json_str(&mut s, title);
    s.push_str(",\"editor\":");
    match editor {
        Some(t) => push_json_str(&mut s, t),
        None => s.push_str("null"),
    }
    s.push_str(",\"nodes\":{");
    for (i, m) in visible.values().enumerate() {
        if i > 0 {
            s.push(',');
        }
        push_json_str(&mut s, m.name);
        s.push_str(":{\"file\":");
        push_json_str(&mut s, m.file);
        s.push_str(",\"line\":");
        s.push_str(&m.line.to_string());
        s.push_str(",\"kind\":");
        push_json_str(&mut s, m.kind);
        s.push_str(",\"input\":");
        push_json_str(&mut s, m.input);
        s.push_str(",\"async\":");
        s.push_str(if m.is_async { "true" } else { "false" });
        s.push_str(",\"file_abs\":");
        push_json_str(&mut s, &absolutize(m.file));
        s.push_str(",\"basename\":");
        push_json_str(&mut s, basename(m.file));
        s.push_str(",\"children\":[");
        for (j, c) in m.chain.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            push_json_str(&mut s, c.name);
        }
        s.push_str("],\"child_kinds\":[");
        for (j, c) in m.chain.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            let k = match c.kind {
                jigs_core::ChainKind::Then => "then",
                jigs_core::ChainKind::Fork => "fork",
            };
            push_json_str(&mut s, k);
        }
        s.push_str("]}");
    }
    s.push_str("}}");
    s
}

fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn basename(file: &str) -> &str {
    Path::new(file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file)
}

fn absolutize(file: &str) -> String {
    let p = Path::new(file);
    if p.is_absolute() {
        return file.to_string();
    }
    match std::env::current_dir() {
        Ok(cwd) => {
            let joined: PathBuf = cwd.join(p);
            joined.to_string_lossy().into_owned()
        }
        Err(_) => file.to_string(),
    }
}

fn esc_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(name: &'static str, kind: &'static str, chain: &[&'static str]) -> JigMeta {
        let v: Vec<jigs_core::ChainStep> = chain
            .iter()
            .map(|n| jigs_core::ChainStep {
                name: n,
                kind: jigs_core::ChainKind::Then,
            })
            .collect();
        let leaked: &'static [jigs_core::ChainStep] = Box::leak(v.into_boxed_slice());
        JigMeta {
            name,
            file: "test.rs",
            line: 1,
            kind,
            input: "Request",
            is_async: false,
            chain: leaked,
        }
    }

    fn fake(items: Vec<JigMeta>) -> BTreeMap<&'static str, &'static JigMeta> {
        let leaked: Vec<&'static JigMeta> = items
            .into_iter()
            .map(|m| Box::leak(Box::new(m)) as &'static _)
            .collect();
        leaked.into_iter().map(|m| (m.name, m)).collect()
    }

    #[test]
    fn reachable_filters_to_entry_subgraph() {
        let all = fake(vec![
            meta("root", "Response", &["a", "b"]),
            meta("a", "Request", &[]),
            meta("b", "Branch", &[]),
            meta("orphan", "Other", &[]),
        ]);
        let r = reachable(&all, "root");
        assert!(r.contains_key("root"));
        assert!(r.contains_key("a"));
        assert!(r.contains_key("b"));
        assert!(!r.contains_key("orphan"));
    }

    #[test]
    fn reachable_handles_cycles() {
        let all = fake(vec![meta("a", "Other", &["b"]), meta("b", "Other", &["a"])]);
        let r = reachable(&all, "a");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn encode_emits_structure() {
        let all = fake(vec![
            meta("root", "Response", &["a"]),
            meta("a", "Request", &[]),
        ]);
        let visible = reachable(&all, "root");
        let json = encode(&visible, "root", "demo", None);
        assert!(json.contains("\"entry\":\"root\""));
        assert!(json.contains("\"root\":{"));
        assert!(json.contains("\"children\":[\"a\"]"));
        assert!(json.contains("\"editor\":null"));
    }

    #[test]
    fn editor_template_is_embedded_when_set() {
        let all = fake(vec![meta("root", "Response", &[])]);
        let visible = reachable(&all, "root");
        let tmpl = "vscodium://file/{path}:{line}";
        let json = encode(&visible, "root", "demo", Some(tmpl));
        assert!(json.contains("\"editor\":\"vscodium://file/{path}:{line}\""));
    }

    #[test]
    fn json_escapes_script_close() {
        let all = fake(vec![meta("</script>", "Other", &[])]);
        let visible = reachable(&all, "</script>");
        let json = encode(&visible, "</script>", "t", None);
        assert!(!json.contains("</script>"));
        assert!(json.contains("\\u003c/script"));
    }
}
