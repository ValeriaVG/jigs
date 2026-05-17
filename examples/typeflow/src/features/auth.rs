use crate::types::{AuthReq, AuthenticatedData, OutputResp, RawReq};
use jigs::{jig, Branch, Response};

#[jig]
pub fn authenticate(req: RawReq) -> Branch<AuthReq, OutputResp> {
    let raw = req.0;
    if raw.api_key.is_empty() {
        return Branch::Done(OutputResp::err("missing api key"));
    }
    let uid = raw.api_key.chars().map(|c| c as u64).sum::<u64>() % 1000;
    Branch::Continue(AuthReq(AuthenticatedData {
        user_id: uid,
        value: raw.value,
    }))
}
