use crate::bindings;
use crate::types::{AgentOutput, Ctx};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn require_authenticated(req: Request<Ctx>) -> Branch<Ctx, AgentOutput> {
    if req.0.tenant.is_some() {
        Branch::Continue(req)
    } else {
        Branch::Done(Response::err("401 unauthenticated"))
    }
}

#[jig]
fn input_filter(req: Request<Ctx>) -> Branch<Ctx, AgentOutput> {
    let q = req.0.input.query.to_lowercase();
    let banned = ["ignore previous", "system prompt", "<script"];
    if banned.iter().any(|p| q.contains(p)) {
        return Branch::Done(Response::err("400 input rejected by safety filter"));
    }
    if req.0.input.query.trim().is_empty() {
        return Branch::Done(Response::err("400 empty query"));
    }
    Branch::Continue(req)
}

#[jig]
pub fn admit(req: Request<Ctx>) -> Branch<Ctx, AgentOutput> {
    req.then(require_authenticated).then(input_filter)
}

// Async branch: looks up the Cache API and short-circuits on hit. Runs after
// `admit`, so cache hits skip retrieve + generate but still pass auth + filter.
#[jig]
pub async fn lookup_cache(req: Request<Ctx>) -> Branch<Ctx, AgentOutput> {
    let tenant = req.0.tenant.as_ref().expect("admit").id;
    let url = bindings::cache_url(tenant, &req.0.input.query);
    match bindings::cache_match(&url).await {
        Some(mut hit) => {
            hit.cached = true;
            Branch::Done(Response::ok(hit))
        }
        None => Branch::Continue(req),
    }
}
