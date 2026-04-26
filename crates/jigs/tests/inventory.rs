use jigs::{jig, Branch, Request, Response};

#[jig]
fn validate(req: Request<u32>) -> Request<u32> {
    req
}

#[jig]
fn guard(req: Request<u32>) -> Branch<u32, String> {
    if req.0 > 0 {
        Branch::Continue(req)
    } else {
        Branch::Done(Response::ok("zero".into()))
    }
}

#[jig]
fn finalize(_req: Request<u32>) -> Response<String> {
    Response::ok("done".into())
}

#[jig]
fn entry(req: Request<u32>) -> Response<String> {
    req.then(validate).then(guard).then(finalize)
}

#[test]
fn macro_registers_jigs_in_inventory() {
    let mut names: Vec<&str> = jigs::all_jigs().map(|m| m.name).collect();
    names.sort();
    for expected in ["entry", "finalize", "guard", "validate"] {
        assert!(names.contains(&expected), "missing {expected} in {names:?}");
    }
}

#[test]
fn macro_records_chain_in_textual_order() {
    let entry_meta = jigs::find_jig("entry").expect("entry registered");
    let names: Vec<&str> = entry_meta.chain_names().collect();
    assert_eq!(names, &["validate", "guard", "finalize"]);
    for s in entry_meta.chain {
        assert_eq!(s.kind, jigs::ChainKind::Then);
    }
}

#[test]
fn macro_records_return_kind() {
    assert_eq!(jigs::find_jig("validate").unwrap().kind, "Request");
    assert_eq!(jigs::find_jig("guard").unwrap().kind, "Branch");
    assert_eq!(jigs::find_jig("finalize").unwrap().kind, "Response");
}

#[test]
fn map_html_includes_registered_jigs() {
    let html = jigs::map::to_html(Some("entry"), "test pipeline", None);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("\"entry\":\"entry\""));
    assert!(html.contains("\"validate\":{"));
    assert!(html.contains("\"guard\":{"));
    assert!(html.contains("\"finalize\":{"));
    assert!(
        html.contains("\"chain\":[\"validate\",\"guard\",\"finalize\"]")
            || html.contains("\"children\":[\"validate\",\"guard\",\"finalize\"]")
    );
}
