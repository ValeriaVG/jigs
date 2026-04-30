use crate::types::{Event, EventCtx, EventResult};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn validate_notification(req: Request<EventCtx>) -> Branch<EventCtx, EventResult> {
    match &req.0.event {
        Event::UserNotification {
            channel, message, ..
        } => {
            let allowed = ["email", "push", "sms"];
            if !allowed.contains(&channel.as_str()) || message.is_empty() {
                return Branch::Done(Response::err("invalid notification payload"));
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
        Event::UserNotification {
            user_id,
            channel,
            message,
        } => {
            format!("sent {channel} to user {user_id}: {message}")
        }
        _ => "unknown notification event".to_string(),
    };
    Response::ok(EventResult {
        event_id: format!("{:?}", ctx.event),
        tenant_id: ctx.tenant_id,
        outcome,
    })
}

#[jig]
pub fn handle(req: Request<EventCtx>) -> Response<EventResult> {
    req.then(validate_notification).then(build_result)
}
