use crate::bindings;
use crate::types::Ctx;
use jigs::{jig, Request};

#[jig]
pub async fn authenticate(req: Request<Ctx>) -> Request<Ctx> {
    let tenant = bindings::lookup_tenant(&req.0.input.api_token).await;
    Request(Ctx { tenant, ..req.0 })
}
