//! Mermaid flowchart rendering of the live jig inventory.
//!
//! Mermaid renders statically — there is no expand/collapse. To keep the
//! output readable we flatten the pipeline to leaf jigs: each composite jig
//! becomes a `subgraph` labelled with its name, containing its leaf
//! descendants. Edges connect leaves only, so a chain like
//! `a.then(composite).then(b)` becomes `a → first_leaf(composite)` and
//! `last_leaf(composite) → b`, with `composite`'s own internals laid out
//! inside its subgraph.

use jigs_core::{ChainKind, JigMeta};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

type Index = BTreeMap<&'static str, &'static JigMeta>;

/// Render the pipeline rooted at `entry` (or the first registered jig if
/// `None`) as a Mermaid `flowchart TD` document, without any surrounding
/// markdown fence.
pub fn to_mermaid(entry: Option<&str>) -> String {
    let all: Index = jigs_core::all_jigs().map(|m| (m.name, m)).collect();
    let entry = entry
        .map(str::to_string)
        .or_else(|| all.keys().next().map(|s| s.to_string()))
        .unwrap_or_default();
    render(&all, &entry)
}

/// Render the pipeline as a Markdown document with a Mermaid code fence,
/// suitable for committing alongside the HTML map.
pub fn to_markdown(entry: Option<&str>, title: &str) -> String {
    let mut s = String::new();
    writeln!(s, "# {title}\n").ok();
    writeln!(s, "```mermaid").ok();
    s.push_str(&to_mermaid(entry));
    writeln!(s, "```").ok();
    s
}

fn render(all: &Index, entry: &str) -> String {
    let mut out = String::from("flowchart TD\n");
    if entry.is_empty() {
        return out;
    }
    let mut leaves = Vec::new();
    collect_leaves(entry, all, &mut HashSet::new(), &mut leaves);
    for name in &leaves {
        let (open, close, label) = node_visual(name, all);
        writeln!(out, "  {name}{open}\"{label}\"{close}").ok();
    }
    out.push('\n');
    let homes = compute_leaf_homes(entry, all);
    emit(&mut out, all, entry, &homes, &mut HashSet::new(), 0, true);
    out.push_str(
        "\n  %% shape legend: rect = Request → Request, rhombus = switching (Request → Response/Branch), stadium = Response → Response\n",
    );
    out
}

/// Returns true if `m` is a fork dispatcher: every chain entry was added
/// via `fork!`. We treat such jigs as decision points rather than as
/// linear pipelines.
fn is_fork_dispatcher(m: &JigMeta) -> bool {
    !m.chain.is_empty() && m.chain.iter().all(|c| c.kind == ChainKind::Fork)
}

/// For each leaf jig that appears as a fork-arm of one or more
/// dispatchers, pick the shallowest such dispatcher in the inclusion
/// tree. That dispatcher becomes the leaf's "home" — the only place the
/// leaf will be re-mentioned inside a subgraph block. Shared leaves
/// (e.g., a `not_found` reused across dispatchers) end up at their LCA
/// instead of being silently captured by the first dispatcher that
/// references them.
fn compute_leaf_homes<'a>(entry: &'a str, all: &'a Index) -> HashMap<&'a str, &'a str> {
    let mut depths: HashMap<&'a str, usize> = HashMap::new();
    let mut leaf_parents: HashMap<&'a str, Vec<&'a str>> = HashMap::new();
    walk_for_homes(entry, all, 0, &mut depths, &mut leaf_parents);
    leaf_parents
        .into_iter()
        .filter_map(|(leaf, parents)| {
            parents
                .into_iter()
                .min_by_key(|p| depths.get(p).copied().unwrap_or(usize::MAX))
                .map(|p| (leaf, p))
        })
        .collect()
}

fn walk_for_homes<'a>(
    name: &'a str,
    all: &'a Index,
    depth: usize,
    depths: &mut HashMap<&'a str, usize>,
    leaf_parents: &mut HashMap<&'a str, Vec<&'a str>>,
) {
    if depths.contains_key(name) {
        return;
    }
    depths.insert(name, depth);
    let Some(m) = all.get(name) else { return };
    if m.chain.is_empty() {
        return;
    }
    for c in m.chain {
        if c.kind == ChainKind::Fork {
            let is_leaf = all.get(c.name).is_none_or(|cm| cm.chain.is_empty());
            if is_leaf {
                leaf_parents.entry(c.name).or_default().push(name);
            }
        }
        walk_for_homes(c.name, all, depth + 1, depths, leaf_parents);
    }
}

/// Pick the mermaid shape and label for a leaf node. Categories:
///   * Request → Request   ⇒ rectangle  `[..]`
///   * Request → switch    ⇒ rhombus    `{..}`  (Response, Branch, Pending)
///   * Response → Response ⇒ stadium    `([..])`
///
/// Externals or unrecognised pairs render as a flag/asymmetric shape `>..]`.
fn node_visual(name: &str, all: &Index) -> (&'static str, &'static str, String) {
    let Some(m) = all.get(name) else {
        return (">", "]", format!("{name}<br/><i>external</i>"));
    };
    let async_prefix = if m.is_async { "async " } else { "" };
    let (open, close, sig) = match (m.input, m.kind) {
        ("Request", "Request") => ("[", "]", "req → req"),
        ("Request", "Response") => ("{", "}", "req → res"),
        ("Request", "Branch") => ("{", "}", "req → branch"),
        ("Request", "Pending") => ("{", "}", "req → async"),
        ("Response", "Response") => ("([", "])", "res → res"),
        _ => ("[", "]", "?"),
    };
    (
        open,
        close,
        format!("{name}<br/><i>{async_prefix}{sig}</i>"),
    )
}

/// Walk the pipeline and accumulate the names of every leaf (chain-less)
/// jig that participates, so each can be declared exactly once.
fn collect_leaves(name: &str, all: &Index, seen: &mut HashSet<String>, out: &mut Vec<String>) {
    if !seen.insert(name.to_string()) {
        return;
    }
    match all.get(name) {
        Some(m) if !m.chain.is_empty() => {
            for c in m.chain {
                collect_leaves(c.name, all, seen, out);
            }
        }
        _ => out.push(name.to_string()),
    }
}

fn emit(
    out: &mut String,
    all: &Index,
    name: &str,
    homes: &HashMap<&str, &str>,
    seen: &mut HashSet<String>,
    depth: usize,
    is_root: bool,
) {
    if !seen.insert(name.to_string()) {
        return;
    }
    let Some(m) = all.get(name) else { return };
    if m.chain.is_empty() {
        return;
    }
    let pad = "  ".repeat(depth);
    if !is_root {
        writeln!(out, "{pad}subgraph {name} [\"{name}\"]").ok();
        writeln!(out, "{pad}  direction TB").ok();
    }
    let edge_pad = if is_root {
        String::new()
    } else {
        format!("{pad}  ")
    };
    for w in m.chain.windows(2) {
        // Sibling fork arms don't flow into each other — exactly one runs.
        if w[0].kind == ChainKind::Fork && w[1].kind == ChainKind::Fork {
            continue;
        }
        let from = last_leaf(w[0].name, all, &mut HashSet::new());
        let to = first_leaf(w[1].name, all, &mut HashSet::new());
        writeln!(out, "{edge_pad}{from} --> {to}").ok();
    }
    for c in m.chain {
        match all.get(c.name) {
            Some(cm) if !cm.chain.is_empty() => {
                emit(out, all, c.name, homes, seen, depth + 1, false);
            }
            // Re-mention the leaf only inside its assigned home — the
            // shallowest dispatcher that references it as an arm. This
            // lets mermaid place shared leaves at their LCA rather
            // than capturing them in the first dispatcher visited.
            _ if c.kind == ChainKind::Fork && homes.get(c.name).copied() == Some(name) => {
                writeln!(out, "{edge_pad}{}", c.name).ok();
            }
            _ => {}
        }
    }
    if !is_root {
        writeln!(out, "{pad}end").ok();
    }
}

fn first_leaf<'a>(name: &'a str, all: &'a Index, seen: &mut HashSet<&'a str>) -> &'a str {
    if !seen.insert(name) {
        return name;
    }
    match all.get(name) {
        Some(m) if !m.chain.is_empty() => {
            // Fork dispatchers have no single "first" leaf — the first thing
            // the request meets is the dispatcher subgraph border. Stop here
            // so the bridge edge lands on the subgraph, not on one arm.
            if is_fork_dispatcher(m) {
                name
            } else {
                first_leaf(m.chain[0].name, all, seen)
            }
        }
        _ => name,
    }
}

fn last_leaf<'a>(name: &'a str, all: &'a Index, seen: &mut HashSet<&'a str>) -> &'a str {
    if !seen.insert(name) {
        return name;
    }
    match all.get(name) {
        Some(m) if !m.chain.is_empty() => {
            if is_fork_dispatcher(m) {
                name
            } else {
                last_leaf(m.chain[m.chain.len() - 1].name, all, seen)
            }
        }
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jigs_core::ChainStep;

    fn fake(items: Vec<JigMeta>) -> Index {
        items
            .into_iter()
            .map(|m| {
                let r: &'static JigMeta = Box::leak(Box::new(m));
                (r.name, r)
            })
            .collect()
    }

    fn then_chain(names: &[&'static str]) -> &'static [ChainStep] {
        let v: Vec<ChainStep> = names
            .iter()
            .map(|n| ChainStep {
                name: n,
                kind: ChainKind::Then,
            })
            .collect();
        Box::leak(v.into_boxed_slice())
    }

    fn fork_chain(names: &[&'static str]) -> &'static [ChainStep] {
        let v: Vec<ChainStep> = names
            .iter()
            .map(|n| ChainStep {
                name: n,
                kind: ChainKind::Fork,
            })
            .collect();
        Box::leak(v.into_boxed_slice())
    }

    fn meta(name: &'static str, kind: &'static str, chain: &[&'static str]) -> JigMeta {
        meta_full(name, "Request", kind, false, then_chain(chain))
    }
    fn meta_full(
        name: &'static str,
        input: &'static str,
        kind: &'static str,
        is_async: bool,
        chain: &'static [ChainStep],
    ) -> JigMeta {
        JigMeta {
            name,
            file: "t.rs",
            line: 1,
            kind,
            input,
            is_async,
            chain,
        }
    }

    #[test]
    fn entry_is_not_a_floating_node() {
        let all = fake(vec![
            meta("root", "Response", &["a", "b"]),
            meta("a", "Request", &[]),
            meta("b", "Branch", &[]),
        ]);
        let m = render(&all, "root");
        assert!(
            !m.contains("root["),
            "entry should not be declared as a leaf node"
        );
        assert!(!m.contains("subgraph root"), "entry should not be wrapped");
        assert!(m.contains("a --> b"));
    }

    #[test]
    fn composite_child_inlines_into_parent_chain() {
        let all = fake(vec![
            meta("root", "Response", &["a", "sub", "b"]),
            meta("a", "Request", &[]),
            meta("b", "Response", &[]),
            meta("sub", "Response", &["x", "y"]),
            meta("x", "Request", &[]),
            meta("y", "Response", &[]),
        ]);
        let m = render(&all, "root");
        // edges from parent chain hop to the first/last leaf of the composite
        assert!(m.contains("a --> x"), "{m}");
        assert!(m.contains("y --> b"), "{m}");
        // composite is its own subgraph
        assert!(m.contains("subgraph sub"));
        assert!(m.contains("x --> y"));
        // composite is never declared as a leaf node
        assert!(!m.contains("sub[\""));
    }

    #[test]
    fn shape_varies_by_category() {
        let all = fake(vec![
            meta_full(
                "root",
                "Request",
                "Response",
                false,
                then_chain(&["a", "b", "c"]),
            ),
            meta_full("a", "Request", "Request", false, then_chain(&[])),
            meta_full("b", "Request", "Branch", false, then_chain(&[])),
            meta_full("c", "Response", "Response", false, then_chain(&[])),
        ]);
        let m = render(&all, "root");
        assert!(m.contains("a[\"a"), "Request→Request should be a rect: {m}");
        assert!(m.contains("b{\"b"), "switching should be a rhombus: {m}");
        assert!(
            m.contains("c([\"c"),
            "Response→Response should be a stadium: {m}"
        );
        assert!(m.contains("req → req"));
        assert!(m.contains("req → branch"));
        assert!(m.contains("res → res"));
    }

    #[test]
    fn async_prefix_in_label() {
        let all = fake(vec![
            meta_full("root", "Request", "Response", false, then_chain(&["a"])),
            meta_full("a", "Request", "Request", true, then_chain(&[])),
        ]);
        let m = render(&all, "root");
        assert!(m.contains("async req → req"), "async marker missing: {m}");
    }

    #[test]
    fn leaf_fork_arms_are_placed_inside_parent_subgraph() {
        let all = fake(vec![
            meta("entry", "Response", &["router"]),
            meta_full(
                "router",
                "Request",
                "Response",
                false,
                fork_chain(&["a", "b"]),
            ),
            meta("a", "Response", &[]),
            meta("b", "Response", &[]),
        ]);
        let m = render(&all, "entry");
        let after = m.split("subgraph router").nth(1).unwrap_or("");
        let block = after.split("end").next().unwrap_or("");
        assert!(block.contains("a"), "router subgraph should contain a: {m}");
        assert!(block.contains("b"), "router subgraph should contain b: {m}");
    }

    #[test]
    fn shared_leaf_arm_lives_at_lca() {
        // `not_found` is a fork arm of both inner dispatchers. The shallowest
        // dispatcher that references it should be the only one that captures
        // it visually.
        let all = fake(vec![
            meta("entry", "Response", &["outer"]),
            meta_full(
                "outer",
                "Request",
                "Response",
                false,
                fork_chain(&["inner_a", "inner_b", "not_found"]),
            ),
            meta_full(
                "inner_a",
                "Request",
                "Response",
                false,
                fork_chain(&["leaf_a", "not_found"]),
            ),
            meta_full(
                "inner_b",
                "Request",
                "Response",
                false,
                fork_chain(&["leaf_b", "not_found"]),
            ),
            meta("leaf_a", "Response", &[]),
            meta("leaf_b", "Response", &[]),
            meta("not_found", "Response", &[]),
        ]);
        let m = render(&all, "entry");
        // inner_a should not re-mention not_found (its inner block ends before
        // the matching `end`).
        let inner_a_block = m.split("subgraph inner_a").nth(1).unwrap_or("");
        let inner_a_block = inner_a_block.split("end").next().unwrap_or("");
        assert!(
            !inner_a_block.contains("not_found"),
            "not_found should NOT live inside inner_a: {m}"
        );
        // outer should re-mention not_found before its own closing `end`.
        let outer_block = m.split("subgraph outer").nth(1).unwrap_or("");
        let outer_block = outer_block
            .split("\n  end\n\n  %%")
            .next()
            .unwrap_or(outer_block);
        assert!(
            outer_block.contains("not_found"),
            "not_found should live inside outer (LCA): {m}"
        );
        assert!(inner_a_block.contains("leaf_a"), "{m}");
    }

    #[test]
    fn fork_arms_are_siblings_no_inter_arm_edges() {
        let all = fake(vec![
            meta_full(
                "router",
                "Request",
                "Response",
                false,
                fork_chain(&["a", "b", "c"]),
            ),
            meta("a", "Response", &[]),
            meta("b", "Response", &[]),
            meta("c", "Response", &[]),
        ]);
        let m = render(&all, "router");
        assert!(!m.contains("a --> b"), "no edge between fork siblings: {m}");
        assert!(!m.contains("b --> c"), "no edge between fork siblings: {m}");
    }

    #[test]
    fn no_styling_emitted() {
        let all = fake(vec![
            meta("root", "Response", &["g"]),
            meta("g", "Branch", &[]),
        ]);
        let m = render(&all, "root");
        assert!(!m.contains("classDef"));
        assert!(!m.contains("class g"));
    }
}
