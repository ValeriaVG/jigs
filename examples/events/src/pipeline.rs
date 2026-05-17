use crate::features::{inventory, notifications, orders};
use crate::types::{Event, EventCtx, EventReq, EventResp, RawEvent, RawReq};
use jigs::{fork, jig, jigs, Branch, Request, Response};

#[jig]
fn log_incoming(req: RawReq) -> RawReq {
    println!(
        "--> [{}] incoming {} at t={}",
        req.0.tenant_id, req.0.event_type, req.0.timestamp
    );
    req
}

fn parse_event(raw: &RawEvent) -> Result<Event, String> {
    match raw.event_type.as_str() {
        "order_created" => {
            let data: serde_json::Value =
                serde_json::from_str(&raw.payload).map_err(|e| e.to_string())?;
            Ok(Event::OrderCreated {
                order_id: data["order_id"].as_u64().ok_or("missing order_id")?,
                amount: data["amount"].as_u64().ok_or("missing amount")?,
                currency: data["currency"].as_str().unwrap_or("").to_string(),
            })
        }
        "order_updated" => {
            let data: serde_json::Value =
                serde_json::from_str(&raw.payload).map_err(|e| e.to_string())?;
            Ok(Event::OrderUpdated {
                order_id: data["order_id"].as_u64().ok_or("missing order_id")?,
                status: data["status"].as_str().unwrap_or("").to_string(),
            })
        }
        "inventory_changed" => {
            let data: serde_json::Value =
                serde_json::from_str(&raw.payload).map_err(|e| e.to_string())?;
            Ok(Event::InventoryChanged {
                sku: data["sku"].as_str().unwrap_or("").to_string(),
                delta: data["delta"].as_i64().ok_or("missing delta")?,
                warehouse_id: data["warehouse_id"]
                    .as_u64()
                    .ok_or("missing warehouse_id")?,
            })
        }
        "user_notification" => {
            let data: serde_json::Value =
                serde_json::from_str(&raw.payload).map_err(|e| e.to_string())?;
            Ok(Event::UserNotification {
                user_id: data["user_id"].as_u64().ok_or("missing user_id")?,
                channel: data["channel"].as_str().unwrap_or("").to_string(),
                message: data["message"].as_str().unwrap_or("").to_string(),
            })
        }
        _ => Err(format!("unknown event_type: {}", raw.event_type)),
    }
}

#[jig]
fn parse(req: RawReq) -> Branch<EventReq, EventResp> {
    let raw = req.0;
    let event = match parse_event(&raw) {
        Ok(e) => e,
        Err(msg) => return Branch::Done(EventResp::err(msg)),
    };
    Branch::Continue(EventReq(EventCtx {
        event,
        tenant_id: raw.tenant_id,
        metadata: raw.metadata,
    }))
}

#[jig]
fn enrich(req: EventReq) -> EventReq {
    let mut ctx = req.0;
    ctx.metadata
        .entry("processor".to_string())
        .or_insert_with(|| "events".to_string());
    EventReq(ctx)
}

#[jig]
fn route(req: EventReq) -> EventResp {
    fork!(req,
        |r: &EventCtx| matches!(r.event, Event::OrderCreated { .. } | Event::OrderUpdated { .. }) => orders::handle,
        |r: &EventCtx| matches!(r.event, Event::InventoryChanged { .. }) => inventory::handle,
        |r: &EventCtx| matches!(r.event, Event::UserNotification { .. }) => notifications::handle,
        _ => |_req: EventReq| EventResp::err("no route for event type"),
    )
}

#[jig]
fn log_outbound(res: EventResp) -> EventResp {
    if let Ok(r) = res.0.as_ref() {
        println!("<-- [{}] {}: {}", r.tenant_id, r.event_id, r.outcome);
    }
    res
}

#[jig]
pub fn handle(req: RawReq) -> EventResp {
    let parsed = req.then(log_incoming).then(parse);
    let routed = match parsed {
        Branch::Continue(req) => req.then(enrich).then(route),
        Branch::Done(resp) => resp,
    };
    routed.then(log_outbound)
}

jigs!(handle);
