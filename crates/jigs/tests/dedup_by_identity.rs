use jigs::{fork, jig, jigs, Request, Response};

#[derive(Clone, Request)]
#[allow(dead_code)]
struct Req(u32);

#[derive(Clone, Response)]
#[allow(dead_code)]
struct Resp(Result<String, String>);

mod alpha {
    use super::{Req, Resp};
    use jigs::{jig, Response};

    #[jig]
    pub fn process(r: Req) -> Resp {
        Resp::ok(format!("alpha: {}", r.0))
    }
}

mod beta {
    use super::{Req, Resp};
    use jigs::{jig, Response};

    #[jig]
    pub fn process(r: Req) -> Resp {
        Resp::ok(format!("beta: {}", r.0))
    }
}

#[jig]
fn entry(req: Req) -> Resp {
    fork!(req,
        |n: &u32| *n > 50 => alpha::process,
        _ => beta::process,
    )
}

jigs!(entry);

#[test]
fn collect_includes_same_named_jigs_from_different_modules() {
    let modules: Vec<&str> = all_jigs()
        .filter(|m| m.name == "process")
        .map(|m| m.module)
        .collect();
    assert!(
        modules.len() == 2,
        "expected 2 'process' jigs, got {}: {:?}",
        modules.len(),
        modules
    );
}

#[test]
fn collect_still_deduplicates_same_jig() {
    let count = all_jigs().filter(|m| m.name == "entry").count();
    assert_eq!(count, 1, "entry should appear exactly once");
}
