use crate::bindings;
use crate::types::{AgentResult, CtxReq};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn require_authenticated(req: CtxReq) -> Branch<CtxReq, AgentResult> {
    if req.0.tenant.is_some() {
        Branch::Continue(req)
    } else {
        Branch::Done(AgentResult::err("401 unauthenticated"))
    }
}

#[jig]
fn input_filter(req: CtxReq) -> Branch<CtxReq, AgentResult> {
    let q = req.0.input.query.to_lowercase();
    let banned = ["ignore previous", "system prompt", "<script"];
    if banned.iter().any(|p| q.contains(p)) {
        return Branch::Done(AgentResult::err("400 input rejected by safety filter"));
    }
    if req.0.input.query.trim().is_empty() {
        return Branch::Done(AgentResult::err("400 empty query"));
    }
    Branch::Continue(req)
}

#[jig]
pub fn admit(req: CtxReq) -> Branch<CtxReq, AgentResult> {
    req.then(require_authenticated).then(input_filter)
}

#[jig]
pub async fn lookup_cache(req: CtxReq) -> Branch<CtxReq, AgentResult> {
    let tenant = req.0.tenant.as_ref().expect("admit").id;
    let url = bindings::cache_url(tenant, &req.0.input.query);
    match bindings::cache_match(&url).await {
        Some(mut hit) => {
            hit.cached = true;
            Branch::Done(AgentResult::ok(hit))
        }
        None => Branch::Continue(req),
    }
}
