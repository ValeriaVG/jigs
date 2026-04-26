use crate::types::Ctx;
use jigs::{jig, Request};

#[jig]
fn compute_totals(req: Request<Ctx>) -> Request<Ctx> {
    let total: u64 = req
        .0
        .input
        .items
        .iter()
        .map(|(sku, qty)| req.0.prices.get(sku).copied().unwrap_or(0) * (*qty as u64))
        .sum();
    Request(Ctx {
        total_cents: total,
        ..req.0
    })
}

// Bulk-discount: 5% off carts with 3+ line items.
#[jig]
fn apply_discount(req: Request<Ctx>) -> Request<Ctx> {
    if req.0.input.items.len() >= 3 {
        let discounted = req.0.total_cents * 95 / 100;
        Request(Ctx {
            total_cents: discounted,
            ..req.0
        })
    } else {
        req
    }
}

#[jig]
pub fn price(req: Request<Ctx>) -> Request<Ctx> {
    req.then(compute_totals).then(apply_discount)
}
