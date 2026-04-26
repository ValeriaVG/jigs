use crate::types::{Ctx, OrderResult};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn require_authenticated(req: Request<Ctx>) -> Branch<Ctx, OrderResult> {
    if req.0.user.is_some() {
        Branch::Continue(req)
    } else {
        Branch::Done(Response::err("401 unauthenticated"))
    }
}

#[jig]
fn check_stock(req: Request<Ctx>) -> Branch<Ctx, OrderResult> {
    for (sku, qty) in &req.0.input.items {
        let have = req.0.stock.get(sku).copied().unwrap_or(0);
        if have < *qty {
            return Branch::Done(Response::err(format!(
                "409 insufficient stock for {sku} (have {have}, need {qty})"
            )));
        }
    }
    Branch::Continue(req)
}

#[jig]
pub fn validate(req: Request<Ctx>) -> Branch<Ctx, OrderResult> {
    req.then(require_authenticated).then(check_stock)
}
