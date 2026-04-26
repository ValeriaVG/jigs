use crate::features::{fulfillment, ingest, pricing, validation};
use crate::types::{Ctx, OrderResult};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn log_incoming(req: Request<Ctx>) -> Request<Ctx> {
    let _ = (&req.0.input.token, req.0.input.items.len());
    req
}

#[jig]
fn log_outbound(res: Response<OrderResult>) -> Response<OrderResult> {
    let _ = res.inner.as_ref().map(|o| o.order_id);
    res
}

// Phase 1: log + async I/O ingestion.
#[jig]
async fn prepare(req: Request<Ctx>) -> Request<Ctx> {
    req.then(log_incoming).then(ingest::ingest).await
}

// Phase 2: sync gates and pricing.
#[jig]
fn gate(req: Request<Ctx>) -> Branch<Ctx, OrderResult> {
    req.then(validation::validate).then(pricing::price)
}

#[jig]
pub async fn handle(req: Request<Ctx>) -> Response<OrderResult> {
    let req = req.then(prepare).await;
    let priced = req.then(gate);

    // Branch -> async -> Response has no combinator in the jigs API,
    // so the short-circuit boundary is handled explicitly here.
    let resp = match priced {
        Branch::Done(resp) => resp,
        Branch::Continue(req) => req.then(fulfillment::fulfill).await,
    };
    resp.then(log_outbound)
}
