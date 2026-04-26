use super::*;

#[test]
fn request_to_request_jig_keeps_request_type() {
    fn enrich(r: Request<i32>) -> Request<i32> {
        Request(r.0 + 1)
    }
    let out: Request<i32> = Request(1).then(enrich);
    assert_eq!(out.0, 2);
}

#[test]
fn request_to_response_jig_switches_to_response() {
    fn handle(r: Request<i32>) -> Response<String> {
        Response::ok(format!("got {}", r.0))
    }
    let out = Request(7).then(handle);
    assert_eq!(out.inner.unwrap(), "got 7");
}

#[test]
fn response_to_response_jig_keeps_response_type() {
    fn shout(r: Response<String>) -> Response<String> {
        Response::ok(r.inner.unwrap().to_uppercase())
    }
    let out = Response::ok("hi".to_string()).then(shout);
    assert_eq!(out.inner.unwrap(), "HI");
}

#[test]
fn errored_response_short_circuits_downstream_jigs() {
    fn never_called(_: Response<String>) -> Response<String> {
        panic!("should not run");
    }
    let out = Response::<String>::err("boom").then(never_called);
    assert_eq!(out.inner.unwrap_err(), "boom");
}

#[test]
fn branch_continue_runs_next_jig() {
    fn guard(r: Request<i32>) -> Branch<i32, String> {
        Branch::Continue(r)
    }
    fn handle(r: Request<i32>) -> Response<String> {
        Response::ok(format!("v={}", r.0))
    }
    let out = Request(3).then(guard).then(handle);
    assert_eq!(out.inner.unwrap(), "v=3");
}

#[test]
fn branch_done_short_circuits_to_response() {
    fn guard(_: Request<i32>) -> Branch<i32, String> {
        Branch::Done(Response::err("rejected"))
    }
    fn never_called(_: Request<i32>) -> Response<String> {
        panic!("should not run");
    }
    let out = Request(3).then(guard).then(never_called);
    assert_eq!(out.inner.unwrap_err(), "rejected");
}

#[test]
fn branch_then_req_to_req_stays_in_branch() {
    fn guard(r: Request<i32>) -> Branch<i32, String> {
        Branch::Continue(r)
    }
    fn bump(r: Request<i32>) -> Request<i32> {
        Request(r.0 + 10)
    }
    let out = Request(5).then(guard).then(bump);
    match out {
        Branch::Continue(r) => assert_eq!(r.0, 15),
        Branch::Done(_) => panic!("expected Continue"),
    }
}
