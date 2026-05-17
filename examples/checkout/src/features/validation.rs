use crate::types::{CheckoutReq, CheckoutResp};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn require_authenticated(req: CheckoutReq) -> Branch<CheckoutReq, CheckoutResp> {
    if req.0.user.is_some() {
        Branch::Continue(req)
    } else {
        Branch::Done(CheckoutResp::err("401 unauthenticated"))
    }
}

#[jig]
fn check_stock(req: CheckoutReq) -> Branch<CheckoutReq, CheckoutResp> {
    for (sku, qty) in &req.0.input.items {
        let have = req.0.stock.get(sku).copied().unwrap_or(0);
        if have < *qty {
            return Branch::Done(CheckoutResp::err(format!(
                "409 insufficient stock for {sku} (have {have}, need {qty})"
            )));
        }
    }
    Branch::Continue(req)
}

#[jig]
pub fn validate(req: CheckoutReq) -> Branch<CheckoutReq, CheckoutResp> {
    req.then(require_authenticated).then(check_stock)
}
