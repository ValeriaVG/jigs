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
fn errored_response_still_runs_downstream_jigs() {
    fn observe(r: Response<String>) -> Response<String> {
        assert!(r.is_err());
        r
    }
    let out = Response::<String>::err("boom").then(observe);
    assert_eq!(out.inner.unwrap_err(), "boom");
}

#[test]
fn response_jig_can_recover_from_error() {
    fn recover(r: Response<String>) -> Response<String> {
        if r.is_err() {
            Response::ok("fallback".to_string())
        } else {
            r
        }
    }
    let out = Response::<String>::err("boom").then(recover);
    assert_eq!(out.inner.unwrap(), "fallback");
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

fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    static VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );
    let raw = RawWaker::new(std::ptr::null(), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

// Mirror the macro's async expansion by hand to keep tests dep-free.
fn enrich(r: Request<i32>) -> Pending<impl std::future::Future<Output = Request<i32>>> {
    Pending(async move { Request(r.0 + 1) })
}

fn handle(r: Request<i32>) -> Pending<impl std::future::Future<Output = Response<String>>> {
    Pending(async move { Response::ok(format!("got {}", r.0)) })
}

#[test]
fn async_then_chains_with_sync_then() {
    let out = block_on(async {
        Request(1)
            .then(enrich)
            .then(|r: Request<i32>| Request(r.0 + 10))
            .then(handle)
            .await
    });
    assert_eq!(out.inner.unwrap(), "got 12");
}

#[test]
fn pending_threads_value_through_sync_jig() {
    fn shout(r: Response<String>) -> Response<String> {
        Response::ok(r.inner.unwrap().to_uppercase())
    }
    let out = block_on(async { Request(7).then(handle).then(shout).await });
    assert_eq!(out.inner.unwrap(), "GOT 7");
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
