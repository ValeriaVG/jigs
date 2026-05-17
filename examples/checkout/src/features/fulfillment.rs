use crate::store;
use crate::types::{CheckoutReq, CheckoutResp, Ctx, OrderResult};
use jigs::{jig, Request, Response};

#[jig]
async fn reserve_inventory(req: CheckoutReq) -> CheckoutReq {
    let user = req
        .0
        .user
        .as_ref()
        .expect("guarded by validate::require_authenticated");
    let id = store::reserve(user.id, &req.0.input.items).await;
    CheckoutReq(Ctx {
        reservation: Some(id),
        ..req.0
    })
}

#[jig]
async fn create_order(req: CheckoutReq) -> CheckoutResp {
    let user = req
        .0
        .user
        .expect("guarded by validate::require_authenticated");
    let reservation = req.0.reservation.expect("set by reserve_inventory");
    let order_id = store::persist_order(user.id, req.0.total_cents, &reservation).await;
    CheckoutResp::ok(OrderResult {
        order_id,
        user_id: user.id,
        reservation,
        total_cents: req.0.total_cents,
        line_count: req.0.input.items.len(),
    })
}

#[jig]
pub async fn fulfill(req: CheckoutReq) -> CheckoutResp {
    req.then(reserve_inventory).then(create_order).await
}
