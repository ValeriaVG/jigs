use super::*;

// ---- Custom request / response types used by all tests ----

#[derive(Debug, Clone, PartialEq)]
struct ReqI32(i32);

impl __Classify for ReqI32 {
    const KIND: &'static str = "Request";
}

impl Request for ReqI32 {
    type Payload = i32;
    fn payload(&self) -> &i32 {
        &self.0
    }
    fn into_payload(self) -> i32 {
        self.0
    }
    fn from_payload(payload: i32) -> Self {
        ReqI32(payload)
    }
}

impl Step for ReqI32 {
    type Out = ReqI32;
    type Fut = std::future::Ready<ReqI32>;
    fn into_step(self) -> Self::Fut {
        std::future::ready(self)
    }
}

impl<R: Response> Merge<R> for ReqI32 {
    type Merged = Branch<ReqI32, R>;
    fn into_continue(self) -> Self::Merged {
        Branch::Continue(self)
    }
    fn from_done(resp: R) -> Self::Merged {
        Branch::Done(resp)
    }
}

impl Status for ReqI32 {
    fn succeeded(&self) -> bool {
        true
    }
    fn error(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone)]
struct RespString(Result<String, String>);

impl __Classify for RespString {
    const KIND: &'static str = "Response";
}

impl RespString {
    fn ok(v: String) -> Self {
        RespString(Ok(v))
    }
    fn err(msg: impl Into<String>) -> Self {
        RespString(Err(msg.into()))
    }
}

impl Response for RespString {
    type Payload = String;
    fn ok(payload: String) -> Self {
        RespString::ok(payload)
    }
    fn err(msg: impl Into<String>) -> Self {
        RespString::err(msg)
    }
    fn is_ok(&self) -> bool {
        self.0.is_ok()
    }
    fn into_result(self) -> Result<String, String> {
        self.0
    }
    fn error_msg(&self) -> Option<String> {
        match &self.0 {
            Ok(_) => None,
            Err(e) => Some(e.clone()),
        }
    }
}

impl Step for RespString {
    type Out = RespString;
    type Fut = std::future::Ready<RespString>;
    fn into_step(self) -> Self::Fut {
        std::future::ready(self)
    }
}

impl Merge<RespString> for RespString {
    type Merged = RespString;
    fn into_continue(self) -> Self::Merged {
        self
    }
    fn from_done(resp: RespString) -> Self::Merged {
        resp
    }
}

impl Status for RespString {
    fn succeeded(&self) -> bool {
        self.is_ok()
    }
    fn error(&self) -> Option<String> {
        self.error_msg()
    }
}

#[test]
fn request_to_request_jig_keeps_request_type() {
    fn enrich(r: ReqI32) -> ReqI32 {
        ReqI32(r.0 + 1)
    }
    let out: ReqI32 = ReqI32(1).then(enrich);
    assert_eq!(out.0, 2);
}

#[test]
fn request_to_response_jig_switches_to_response() {
    fn handle(r: ReqI32) -> RespString {
        RespString::ok(format!("got {}", r.0))
    }
    let out = ReqI32(7).then(handle);
    assert_eq!(out.into_result().unwrap(), "got 7");
}

#[test]
fn response_to_response_jig_keeps_response_type() {
    fn shout(r: RespString) -> RespString {
        RespString::ok(r.into_result().unwrap().to_uppercase())
    }
    let out = RespString::ok("hi".to_string()).then(shout);
    assert_eq!(out.into_result().unwrap(), "HI");
}

#[test]
fn errored_response_still_runs_downstream_jigs() {
    fn observe(r: RespString) -> RespString {
        assert!(r.is_err());
        r
    }
    let out = RespString::err("boom").then(observe);
    assert_eq!(out.into_result().unwrap_err(), "boom");
}

#[test]
fn response_jig_can_recover_from_error() {
    fn recover(r: RespString) -> RespString {
        if r.is_err() {
            RespString::ok("fallback".to_string())
        } else {
            r
        }
    }
    let out = RespString::err("boom").then(recover);
    assert_eq!(out.into_result().unwrap(), "fallback");
}

#[test]
fn branch_continue_runs_next_jig() {
    fn guard(r: ReqI32) -> Branch<ReqI32, RespString> {
        Branch::Continue(r)
    }
    fn handle(r: ReqI32) -> RespString {
        RespString::ok(format!("v={}", r.0))
    }
    let out = ReqI32(3).then(guard).then(handle);
    assert_eq!(out.into_result().unwrap(), "v=3");
}

#[test]
fn branch_done_short_circuits_to_response() {
    fn guard(_: ReqI32) -> Branch<ReqI32, RespString> {
        Branch::Done(RespString::err("rejected"))
    }
    fn never_called(_: ReqI32) -> RespString {
        panic!("should not run")
    }
    let out = ReqI32(3).then(guard).then(never_called);
    assert_eq!(out.into_result().unwrap_err(), "rejected");
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

// Mirror the macro's async expansion by hand to keep tests dep-free.
fn enrich_async(r: ReqI32) -> Pending<impl std::future::Future<Output = ReqI32>> {
    Pending(async move { ReqI32(r.0 + 1) })
}

fn handle_async(r: ReqI32) -> Pending<impl std::future::Future<Output = RespString>> {
    Pending(async move { RespString::ok(format!("got {}", r.0)) })
}

#[test]
fn async_then_chains_with_sync_then() {
    let out = block_on(async {
        ReqI32(1)
            .then(enrich_async)
            .then(|r: ReqI32| ReqI32(r.0 + 10))
            .then(handle_async)
            .await
    });
    assert_eq!(out.into_result().unwrap(), "got 12");
}

#[test]
fn pending_threads_value_through_sync_jig() {
    fn shout(r: RespString) -> RespString {
        RespString::ok(r.into_result().unwrap().to_uppercase())
    }
    let out = block_on(async { ReqI32(7).then(handle_async).then(shout).await });
    assert_eq!(out.into_result().unwrap(), "GOT 7");
}

#[test]
fn block_on_polls_multi_yield_future() {
    struct YieldTwice(u32);
    impl std::future::Future for YieldTwice {
        type Output = u32;
        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<u32> {
            self.0 += 1;
            if self.0 >= 3 {
                std::task::Poll::Ready(self.0)
            } else {
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        }
    }
    let out = block_on(YieldTwice(0));
    assert_eq!(out, 3);
}

#[test]
fn block_on_resolves_ready_future() {
    let out = block_on(async { 42 });
    assert_eq!(out, 42);
}

#[test]
fn fork_picks_first_matching_arm() {
    fn even(r: ReqI32) -> RespString {
        RespString::ok(format!("even {}", r.0))
    }
    fn small(r: ReqI32) -> RespString {
        RespString::ok(format!("small {}", r.0))
    }
    fn fallback(r: ReqI32) -> RespString {
        RespString::ok(format!("fallback {}", r.0))
    }
    let out = crate::fork!(ReqI32(4),
        |n: &i32| *n % 2 == 0 => even,
        |n: &i32| *n < 100    => small,
        _ => fallback,
    );
    assert_eq!(out.into_result().unwrap(), "even 4");
}

#[test]
fn fork_falls_through_to_default() {
    fn never(_: ReqI32) -> RespString {
        panic!("should not run")
    }
    fn fallback(r: ReqI32) -> RespString {
        RespString::ok(format!("got {}", r.0))
    }
    let out = crate::fork!(ReqI32(3),
        |n: &i32| *n > 100 => never,
        |n: &i32| *n < 0   => never,
        _ => fallback,
    );
    assert_eq!(out.into_result().unwrap(), "got 3");
}

#[test]
fn fork_later_arms_are_skipped_after_match() {
    fn first(_: ReqI32) -> RespString {
        RespString::ok("first".into())
    }
    fn second(_: ReqI32) -> RespString {
        panic!("should not run after match")
    }
    let out = crate::fork!(ReqI32(1),
        |_: &i32| true => first,
        |_: &i32| true => second,
        _ => second,
    );
    assert_eq!(out.into_result().unwrap(), "first");
}

#[test]
fn branch_then_req_to_req_stays_in_branch() {
    fn guard(r: ReqI32) -> Branch<ReqI32, RespString> {
        Branch::Continue(r)
    }
    fn bump(r: ReqI32) -> ReqI32 {
        ReqI32(r.0 + 10)
    }
    let out = ReqI32(5).then(guard).then(bump);
    match out {
        Branch::Continue(r) => assert_eq!(r.0, 15),
        Branch::Done(_) => panic!("expected Continue"),
    }
}

#[test]
fn custom_request_implements_request_trait() {
    #[derive(Debug, PartialEq)]
    struct MyPayload(i32);

    struct MyReq(MyPayload);

    impl __Classify for MyReq {
        const KIND: &'static str = "Request";
    }

    impl Request for MyReq {
        type Payload = MyPayload;
        fn payload(&self) -> &MyPayload {
            &self.0
        }
        fn into_payload(self) -> MyPayload {
            self.0
        }
        fn from_payload(payload: MyPayload) -> Self {
            MyReq(payload)
        }
    }

    impl Merge<RespString> for MyReq {
        type Merged = Branch<MyReq, RespString>;
        fn into_continue(self) -> Self::Merged {
            Branch::Continue(self)
        }
        fn from_done(resp: RespString) -> Self::Merged {
            Branch::Done(resp)
        }
    }
    impl Status for MyReq {
        fn succeeded(&self) -> bool {
            true
        }
        fn error(&self) -> Option<String> {
            None
        }
    }

    fn enrich(r: MyReq) -> MyReq {
        MyReq(MyPayload(r.0 .0 + 1))
    }

    let out = MyReq(MyPayload(1)).then(enrich);
    assert_eq!(out.0, MyPayload(2));
}

#[test]
fn custom_response_implements_response_trait() {
    struct MyResp {
        ok: bool,
        value: String,
    }

    impl __Classify for MyResp {
        const KIND: &'static str = "Response";
    }

    impl Response for MyResp {
        type Payload = String;
        fn ok(payload: String) -> Self {
            MyResp {
                ok: true,
                value: payload,
            }
        }
        fn err(msg: impl Into<String>) -> Self {
            MyResp {
                ok: false,
                value: msg.into(),
            }
        }
        fn is_ok(&self) -> bool {
            self.ok
        }
        fn into_result(self) -> Result<String, String> {
            if self.ok {
                Ok(self.value)
            } else {
                Err(self.value)
            }
        }
        fn error_msg(&self) -> Option<String> {
            if self.ok {
                None
            } else {
                Some(self.value.clone())
            }
        }
    }
    impl Merge<MyResp> for MyResp {
        type Merged = MyResp;
        fn into_continue(self) -> Self::Merged {
            self
        }
        fn from_done(resp: MyResp) -> Self::Merged {
            resp
        }
    }
    impl Status for MyResp {
        fn succeeded(&self) -> bool {
            self.is_ok()
        }
        fn error(&self) -> Option<String> {
            self.error_msg()
        }
    }

    fn handle(r: ReqI32) -> MyResp {
        MyResp::ok(format!("got {}", r.0))
    }
    fn finalize(r: MyResp) -> MyResp {
        if r.is_ok() {
            MyResp::ok(format!("{} !", r.value))
        } else {
            r
        }
    }

    let out = ReqI32(42).then(handle).then(finalize);
    assert_eq!(out.into_result().unwrap(), "got 42 !");
}

#[test]
fn default_error_msg_returns_some_on_err() {
    struct MinResp(bool);
    impl __Classify for MinResp {
        const KIND: &'static str = "Response";
    }
    impl Response for MinResp {
        type Payload = ();
        fn ok(_: ()) -> Self {
            MinResp(true)
        }
        fn err(_: impl Into<String>) -> Self {
            MinResp(false)
        }
        fn is_ok(&self) -> bool {
            self.0
        }
        fn into_result(self) -> Result<(), String> {
            if self.0 {
                Ok(())
            } else {
                Err(String::new())
            }
        }
    }
    impl Merge<MinResp> for MinResp {
        type Merged = MinResp;
        fn into_continue(self) -> Self::Merged {
            self
        }
        fn from_done(resp: MinResp) -> Self::Merged {
            resp
        }
    }
    impl Status for MinResp {
        fn succeeded(&self) -> bool {
            self.is_ok()
        }
        fn error(&self) -> Option<String> {
            self.error_msg()
        }
    }

    let ok_resp = MinResp::ok(());
    assert!(
        ok_resp.error_msg().is_none(),
        "ok response should have no error_msg"
    );

    let err_resp = MinResp::err("boom");
    let msg = err_resp.error_msg().unwrap();
    assert_eq!(
        msg, "unknown error",
        "default error_msg should return 'unknown error'"
    );
}
