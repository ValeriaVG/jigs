use jigs::{jig, jigs, Branch, ChainKind, Merge, Request, Response, Step};

#[derive(Clone, Request)]
struct TestReq(u32);

#[derive(Clone, Request)]
struct GenericReq<T: Clone + Send + 'static>(T);

#[derive(Clone, Response)]
struct TestResp(Result<String, String>);

#[test]
fn derive_request_generic_struct_compiles() {
    let req = GenericReq(42u32);
    assert_eq!(*req.payload(), 42u32);
    let out: Branch<GenericReq<u32>, TestResp> = req.into_continue();
    assert!(matches!(out, Branch::Continue(_)));
}

#[test]
fn derive_request_generic_struct_merge_from_done() {
    let resp = TestResp::err("fail");
    let out: Branch<GenericReq<String>, TestResp> = GenericReq::<String>::from_done(resp);
    assert!(matches!(out, Branch::Done(_)));
}

#[test]
fn derive_request_generic_step() {
    let req = GenericReq(7u32);
    let out = block_on(Step::into_step(req));
    assert_eq!(out.0, 7u32);
}

#[derive(Clone, Response)]
struct GenericResp<T: Clone + Send + 'static>(Result<T, String>);

#[test]
fn derive_response_generic_struct_roundtrips() {
    let resp = GenericResp::ok(42u32);
    assert!(resp.is_ok());
    assert_eq!(resp.into_result().unwrap(), 42u32);
}

#[test]
fn derive_response_generic_struct_error_roundtrips() {
    let resp = GenericResp::<u32>::err("fail");
    assert!(resp.is_err());
    assert_eq!(resp.into_result().unwrap_err(), "fail");
}

#[test]
fn derive_response_generic_merge() {
    let resp = GenericResp::ok(99u64);
    let merged: GenericResp<u64> = resp.into_continue();
    assert!(merged.is_ok());
    let resp2 = GenericResp::<u64>::err("x");
    let merged2: GenericResp<u64> = GenericResp::from_done(resp2);
    assert!(merged2.is_err());
}

#[test]
fn derive_response_generic_step() {
    let resp = GenericResp::ok("hi".to_string());
    let out = block_on(Step::into_step(resp));
    assert!(out.is_ok());
}

#[derive(Clone, Response)]
struct TwoFieldResp {
    #[resp(ok)]
    data: Option<String>,
    #[resp(err)]
    error: String,
}

#[derive(Clone, Response)]
struct ReversedTwoFieldResp {
    error: String,
    #[resp(ok)]
    data: Option<String>,
}

#[test]
fn derive_response_two_field_reversed_only_ok_attr() {
    let resp = ReversedTwoFieldResp::ok("hello".into());
    assert!(resp.is_ok());
    assert_eq!(resp.into_result().unwrap(), "hello");

    let resp = ReversedTwoFieldResp::err("fail");
    assert!(resp.is_err());
    assert_eq!(resp.error_msg().as_deref(), Some("fail"));
    assert_eq!(resp.into_result().unwrap_err(), "fail");
}

#[derive(Clone, Response)]
struct ReversedTwoFieldResp2 {
    #[resp(err)]
    error: String,
    data: Option<String>,
}

#[test]
fn derive_response_two_field_reversed_only_err_attr() {
    let resp = ReversedTwoFieldResp2::ok("hello".into());
    assert!(resp.is_ok());
    assert_eq!(resp.into_result().unwrap(), "hello");

    let resp = ReversedTwoFieldResp2::err("fail");
    assert!(resp.is_err());
    assert_eq!(resp.error_msg().as_deref(), Some("fail"));
    assert_eq!(resp.into_result().unwrap_err(), "fail");
}

#[test]
fn derive_response_two_field_error_msg_does_not_mutate() {
    let resp = TwoFieldResp::err("boom");
    let msg = resp.error_msg();
    assert_eq!(msg.as_deref(), Some("boom"));
    let msg2 = resp.error_msg();
    assert_eq!(
        msg2.as_deref(),
        Some("boom"),
        "error_msg should not consume self"
    );
}

#[test]
fn derive_response_two_field_roundtrips() {
    let resp = TwoFieldResp::ok("hello".into());
    assert!(resp.is_ok());
    assert_eq!(resp.into_result().unwrap(), "hello");
}

#[test]
fn derive_response_two_field_error_roundtrips() {
    let resp = TwoFieldResp::err("fail");
    assert!(resp.is_err());
    assert_eq!(resp.into_result().unwrap_err(), "fail");
}

#[derive(Clone, Default, Request)]
#[req(field = "value")]
struct MultiFieldReq {
    value: u32,
    label: String,
}

#[test]
fn derive_request_field_attribute_roundtrips() {
    let req = MultiFieldReq::from_payload(42);
    assert_eq!(*req.payload(), 42);
    assert_eq!(req.label, String::default());
    let v = req.into_payload();
    assert_eq!(v, 42);
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll, Waker};
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

#[test]
fn derive_request_impls_step() {
    let fut = Step::into_step(TestReq(1));
    let out = block_on(fut);
    assert_eq!(out.0, 1);
}

#[test]
fn derive_response_impls_step() {
    let fut = Step::into_step(TestResp::ok("hi".into()));
    let out = block_on(fut);
    assert!(out.is_ok());
}

#[jig]
fn validate(req: TestReq) -> TestReq {
    req
}

#[jig]
fn finalize(_req: TestReq) -> TestResp {
    TestResp::ok("done".into())
}

#[jig]
fn fallback(_req: TestReq) -> TestResp {
    TestResp::ok("fallback".into())
}

#[jig]
fn router(req: TestReq) -> TestResp {
    jigs::fork!(req,
        |n: &u32| *n > 100 => finalize,
        _ => fallback,
    )
}

#[jig]
fn entry(req: TestReq) -> TestResp {
    req.then(validate).then(router)
}

jigs!(entry);

#[test]
fn macro_registers_jigs_via_jigdef() {
    let mut names: Vec<&str> = all_jigs().map(|m| m.name).collect();
    names.sort();
    assert_eq!(
        names,
        &["entry", "fallback", "finalize", "router", "validate"]
    );
}

#[test]
fn macro_records_chain_in_textual_order() {
    let entry_meta = find_jig("entry").expect("entry registered");
    let names: Vec<&str> = entry_meta.chain_names().collect();
    assert_eq!(names, &["validate", "router"]);
    for s in entry_meta.chain {
        assert_eq!(s.kind, ChainKind::Then);
    }
}

#[test]
fn fork_arms_appear_in_chain_metadata() {
    let router_meta = find_jig("router").expect("router registered");
    let names: Vec<&str> = router_meta.chain_names().collect();
    assert_eq!(
        names,
        &["finalize", "fallback"],
        "fork arms should be recorded in chain"
    );
    for s in router_meta.chain {
        assert_eq!(s.kind, ChainKind::Fork);
    }
}

#[test]
fn macro_records_return_kind() {
    assert_eq!(find_jig("validate").unwrap().kind, "Other");
    assert_eq!(find_jig("router").unwrap().kind, "Other");
    assert_eq!(find_jig("fallback").unwrap().kind, "Other");
}

#[test]
fn macro_records_payload_types() {
    let validate = find_jig("validate").unwrap();
    assert_eq!(validate.input_type, "TestReq");
    assert_eq!(validate.output_type, "TestReq");

    let router_meta = find_jig("router").unwrap();
    assert_eq!(router_meta.input_type, "TestReq");
    assert_eq!(router_meta.output_type, "TestResp");
}

#[derive(Clone, Response)]
enum EnumResp {
    #[resp(ok)]
    Success(String),
    #[resp(err)]
    Failure(String),
}

#[test]
fn derive_response_enum_roundtrips() {
    let resp = EnumResp::ok("hello".into());
    assert!(resp.is_ok());
    assert_eq!(resp.into_result().unwrap(), "hello");
}

#[test]
fn derive_response_enum_error_roundtrips() {
    let resp = EnumResp::err("boom");
    assert!(resp.is_err());
    assert_eq!(resp.into_result().unwrap_err(), "boom");
}

#[test]
fn derive_response_enum_error_msg_does_not_mutate() {
    let resp = EnumResp::err("boom");
    let msg = resp.error_msg();
    assert_eq!(msg.as_deref(), Some("boom"));
    let msg2 = resp.error_msg();
    assert_eq!(
        msg2.as_deref(),
        Some("boom"),
        "error_msg should not consume self"
    );
}

#[derive(Clone, Response)]
enum NamedFieldEnumResp {
    #[resp(ok)]
    Success { data: String },
    #[resp(err)]
    Failure { error: String },
}

#[test]
fn derive_response_enum_named_field_roundtrips() {
    let resp = NamedFieldEnumResp::ok("hello".into());
    assert!(resp.is_ok());
    assert_eq!(resp.into_result().unwrap(), "hello");
}

#[test]
fn derive_response_enum_named_field_error_roundtrips() {
    let resp = NamedFieldEnumResp::err("boom");
    assert!(resp.is_err());
    assert_eq!(resp.into_result().unwrap_err(), "boom");
}

#[test]
fn derive_response_enum_named_field_error_msg() {
    let resp = NamedFieldEnumResp::err("boom");
    assert_eq!(resp.error_msg().as_deref(), Some("boom"));
}

#[test]
fn map_html_includes_registered_jigs() {
    let html = ::jigs::map::to_html(all_jigs(), "test pipeline", None);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("\"entry\":\"entry\""));
    assert!(html.contains("\"validate\":{"));
    assert!(html.contains("\"router\":{"));
    assert!(html.contains("\"fallback\":{"));
}
