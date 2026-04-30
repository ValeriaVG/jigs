use crate::types::{Event, EventCtx, EventResult};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn validate_order(req: Request<EventCtx>) -> Branch<EventCtx, EventResult> {
    match &req.0.event {
        Event::OrderCreated {
            amount, currency, ..
        } => {
            if *amount == 0 || currency.is_empty() {
                return Branch::Done(Response::err("invalid order payload"));
            }
        }
        Event::OrderUpdated { status, .. } => {
            if status.is_empty() {
                return Branch::Done(Response::err("missing order status"));
            }
        }
        _ => return Branch::Continue(req),
    }
    Branch::Continue(req)
}

#[jig]
fn build_result(req: Request<EventCtx>) -> Response<EventResult> {
    let ctx = req.0;
    let outcome = match &ctx.event {
        Event::OrderCreated {
            order_id,
            amount,
            currency,
        } => {
            format!("created order {order_id} for {amount} {currency}")
        }
        Event::OrderUpdated { order_id, status } => {
            format!("updated order {order_id} to {status}")
        }
        _ => "unknown order event".to_string(),
    };
    Response::ok(EventResult {
        event_id: format!("{:?}", ctx.event),
        tenant_id: ctx.tenant_id,
        outcome,
    })
}

#[jig]
pub fn handle(req: Request<EventCtx>) -> Response<EventResult> {
    req.then(validate_order).then(build_result)
}
