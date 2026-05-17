use ::jigs::{jig, jigs, Branch, ChainKind, Request, Response};

#[derive(Clone, Request)]
#[allow(dead_code)]
struct TestReq(u32);

#[derive(Clone, Response)]
#[allow(dead_code)]
struct TestResp(Result<String, String>);

#[jig]
fn validate(req: TestReq) -> TestReq {
    req
}

#[jig]
fn guard(req: TestReq) -> Branch<TestReq, TestResp> {
    if req.0 > 0 {
        Branch::Continue(req)
    } else {
        Branch::Done(TestResp::ok("zero".into()))
    }
}

#[jig]
fn finalize(_req: TestReq) -> TestResp {
    TestResp::ok("done".into())
}

#[jig]
fn entry(req: TestReq) -> TestResp {
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
    assert_eq!(find_jig("validate").unwrap().kind, "Other");
    assert_eq!(find_jig("guard").unwrap().kind, "Branch");
    assert_eq!(find_jig("finalize").unwrap().kind, "Other");
}

#[test]
fn macro_records_payload_types() {
    let validate = find_jig("validate").unwrap();
    assert_eq!(validate.input_type, "TestReq");
    assert_eq!(validate.output_type, "TestReq");

    let guard_meta = find_jig("guard").unwrap();
    assert_eq!(guard_meta.input_type, "TestReq");
    assert_eq!(guard_meta.output_type, "Branch<TestReq,TestResp>");

    let finalize = find_jig("finalize").unwrap();
    assert_eq!(finalize.input_type, "TestReq");
    assert_eq!(finalize.output_type, "TestResp");
}

#[test]
fn map_html_includes_registered_jigs() {
    let html = ::jigs::map::to_html(all_jigs(), "test pipeline", None);
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
