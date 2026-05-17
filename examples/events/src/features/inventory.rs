use crate::types::EventReq;
use crate::types::EventResp;
use crate::types::{Event, EventResult};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn validate_inventory(req: EventReq) -> Branch<EventReq, EventResp> {
    match &req.0.event {
        Event::InventoryChanged {
            sku, warehouse_id, ..
        } => {
            if sku.is_empty() || *warehouse_id == 0 {
                return Branch::Done(EventResp::err("invalid inventory payload"));
            }
        }
        _ => return Branch::Continue(req),
    }
    Branch::Continue(req)
}

#[jig]
fn build_result(req: EventReq) -> EventResp {
    let ctx = req.0;
    let outcome = match &ctx.event {
        Event::InventoryChanged {
            sku,
            delta,
            warehouse_id,
        } => {
            format!("adjusted inventory for {sku} by {delta} in warehouse {warehouse_id}")
        }
        _ => "unknown inventory event".to_string(),
    };
    EventResp::ok(EventResult {
        event_id: format!("{:?}", ctx.event),
        tenant_id: ctx.tenant_id,
        outcome,
    })
}

#[jig]
pub fn handle(req: EventReq) -> EventResp {
    req.then(validate_inventory).then(build_result)
}
