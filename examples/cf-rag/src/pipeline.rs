use crate::features::{agent, guard, ingest, output, retrieve};
use crate::types::{AgentResult, CtxReq};
use jigs::{jig, jigs, Branch, Request, Response};

#[jig]
fn log_incoming(req: CtxReq) -> CtxReq {
    let _ = (&req.0.input.api_token, req.0.input.query.len());
    req
}

#[jig]
fn log_outbound(res: AgentResult) -> AgentResult {
    let _ = res.0.as_ref().map(|o| o.tenant_id);
    res
}

#[jig]
async fn prepare(req: CtxReq) -> CtxReq {
    req.then(log_incoming).then(ingest::authenticate).await
}

#[jig]
pub async fn handle(req: CtxReq) -> AgentResult {
    let req = req.then(prepare).await;

    let resp = match req.then(guard::admit) {
        Branch::Done(r) => r,
        Branch::Continue(req) => match req.then(guard::lookup_cache).await {
            Branch::Done(r) => r,
            Branch::Continue(req) => {
                let req = req.then(retrieve::retrieve).await;
                req.then(agent::run).await
            }
        },
    };
    resp.then(output::finalize).then(log_outbound)
}

jigs!(handle);
