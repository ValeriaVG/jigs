use crate::features;
use crate::types::{OutputResp, RawReq};
use jigs::{jig, jigs, Branch, Request, Response};

#[jig]
fn log_incoming(req: RawReq) -> RawReq {
    println!("--> key={} val={}", req.0.api_key, req.0.value);
    req
}

#[jig]
fn log_outbound(res: OutputResp) -> OutputResp {
    match res.0.as_ref() {
        Ok(o) => println!("<-- uid={} result={}", o.user_id, o.result),
        Err(e) => println!("<-- error: {e}"),
    }
    res
}

#[jig]
pub fn handle(req: RawReq) -> OutputResp {
    let branch = req.then(log_incoming).then(features::auth::authenticate);
    match branch {
        Branch::Done(resp) => resp,
        Branch::Continue(req) => req.then(features::compute::compute),
    }
    .then(log_outbound)
}

jigs!(handle);
