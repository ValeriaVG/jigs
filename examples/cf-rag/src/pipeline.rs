use crate::features::{agent, guard, ingest, output, retrieve};
use crate::types::{AgentOutput, Ctx};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn log_incoming(req: Request<Ctx>) -> Request<Ctx> {
    let _ = (&req.0.input.api_token, req.0.input.query.len());
    req
}

#[jig]
fn log_outbound(res: Response<AgentOutput>) -> Response<AgentOutput> {
    let _ = res.inner.as_ref().map(|o| o.tenant_id);
    res
}

#[jig]
async fn prepare(req: Request<Ctx>) -> Request<Ctx> {
    req.then(log_incoming).then(ingest::authenticate).await
}

#[jig]
pub async fn handle(req: Request<Ctx>) -> Response<AgentOutput> {
    let req = req.then(prepare).await;

    // Two Branch boundaries: cheap sync admit, then async cache lookup.
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
