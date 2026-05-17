use jigs_core::JigMeta;
use std::collections::BTreeMap;

pub(crate) type Index = BTreeMap<&'static str, Vec<&'static JigMeta>>;

pub(crate) fn build_index(jigs: impl Iterator<Item = &'static JigMeta>) -> Index {
    let mut map: Index = BTreeMap::new();
    for m in jigs {
        map.entry(m.name).or_default().push(m);
    }
    map
}

pub(crate) fn resolve(name: &str, all: &Index) -> Option<&'static JigMeta> {
    if let Some(v) = all.get(name) {
        return v
            .iter()
            .max_by_key(|m| m.module.split("::").count())
            .copied();
    }
    if let Some(pos) = name.rfind("::") {
        let target_name = &name[pos + 2..];
        let prefix = name[..pos].strip_prefix("crate::").unwrap_or(&name[..pos]);
        if let Some(candidates) = all.get(target_name) {
            for m in candidates {
                if m.module == prefix
                    || m.module.ends_with(&format!("::{prefix}"))
                    || m.module.contains(&format!("::{prefix}"))
                {
                    return Some(m);
                }
            }
            for m in candidates {
                if m.file.contains(prefix) {
                    return Some(m);
                }
            }
            return candidates.first().copied();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(name: &'static str, module: &'static str) -> JigMeta {
        JigMeta {
            name,
            file: "",
            line: 0,
            kind: "Response",
            input: "Request",
            input_type: "",
            output_type: "",
            is_async: false,
            module,
            chain: &[],
        }
    }

    fn leak_meta(items: Vec<JigMeta>) -> Index {
        let mut map: Index = BTreeMap::new();
        for m in items {
            let r: &'static JigMeta = Box::leak(Box::new(m));
            map.entry(r.name).or_default().push(r);
        }
        map
    }

    #[test]
    fn resolve_picks_deepest_module_when_names_collide() {
        let all = leak_meta(vec![
            make_meta("validate", "crate"),
            make_meta("validate", "crate::features::auth"),
        ]);
        let resolved = resolve("validate", &all);
        assert_eq!(resolved.unwrap().module, "crate::features::auth");
    }

    #[test]
    fn resolve_qualified_matches_module_prefix() {
        let all = leak_meta(vec![
            make_meta("validate", "crate"),
            make_meta("validate", "crate::features::auth"),
        ]);
        let resolved = resolve("features::auth::validate", &all);
        assert_eq!(resolved.unwrap().module, "crate::features::auth");
    }

    #[test]
    fn resolve_does_not_match_suffix_without_boundary() {
        let all = leak_meta(vec![
            make_meta("handle", "crate::features::oauth"),
            make_meta("handle", "crate::features::auth"),
        ]);
        let resolved = resolve("auth::handle", &all);
        assert_eq!(resolved.unwrap().module, "crate::features::auth");
    }

    #[test]
    fn resolve_exact_module_match() {
        let all = leak_meta(vec![make_meta("handle", "auth")]);
        let resolved = resolve("auth::handle", &all);
        assert_eq!(resolved.unwrap().module, "auth");
    }
}
