use ::jigs::{jig, jigs, Branch, ChainKind, Request, Response};

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

jigs!(entry);

#[test]
fn macro_registers_jigs_via_jigdef() {
    let mut names: Vec<&str> = all_jigs().map(|m| m.name).collect();
    names.sort();
    assert_eq!(names, &["entry", "finalize", "guard", "validate"]);
}

#[test]
fn macro_records_chain_in_textual_order() {
    let entry_meta = find_jig("entry").expect("entry registered");
    let names: Vec<&str> = entry_meta.chain_names().collect();
    assert_eq!(names, &["validate", "guard", "finalize"]);
    for s in entry_meta.chain {
        assert_eq!(s.kind, ChainKind::Then);
    }
}

#[test]
fn macro_records_return_kind() {
    assert_eq!(find_jig("validate").unwrap().kind, "Request");
    assert_eq!(find_jig("guard").unwrap().kind, "Branch");
    assert_eq!(find_jig("finalize").unwrap().kind, "Response");
}

#[test]
fn macro_records_payload_types() {
    let validate = find_jig("validate").unwrap();
    assert_eq!(validate.input_type, "u32");
    assert_eq!(validate.output_type, "u32");

    let guard_meta = find_jig("guard").unwrap();
    assert_eq!(guard_meta.input_type, "u32");
    assert_eq!(guard_meta.output_type, "Branch<u32,String>");

    let finalize = find_jig("finalize").unwrap();
    assert_eq!(finalize.input_type, "u32");
    assert_eq!(finalize.output_type, "String");
}

#[test]
fn map_html_includes_registered_jigs() {
    let html = ::jigs::map::to_html(all_jigs(), Some("entry"), "test pipeline", None);
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
