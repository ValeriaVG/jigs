use crate::features::{fulfillment, ingest, pricing, validation};
use crate::types::{CheckoutReq, CheckoutResp};
use jigs::{jig, jigs, Branch, Request, Response};

#[jig]
fn log_incoming(req: CheckoutReq) -> CheckoutReq {
    let _ = (&req.0.input.token, req.0.input.items.len());
    req
}

#[jig]
fn log_outbound(res: CheckoutResp) -> CheckoutResp {
    let _ = res.0.as_ref().map(|o| o.order_id);
    res
}

// Phase 1: log + async I/O ingestion.
#[jig]
async fn prepare(req: CheckoutReq) -> CheckoutReq {
    req.then(log_incoming).then(ingest::ingest).await
}

// Phase 2: sync gates and pricing.
#[jig]
fn gate(req: CheckoutReq) -> Branch<CheckoutReq, CheckoutResp> {
    req.then(validation::validate).then(pricing::price)
}

#[jig]
pub async fn handle(req: CheckoutReq) -> CheckoutResp {
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

jigs!(handle);
