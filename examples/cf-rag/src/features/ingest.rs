use crate::bindings;
use crate::types::{Ctx, CtxReq};
use jigs::jig;

#[jig]
pub async fn authenticate(req: CtxReq) -> CtxReq {
    let tenant = bindings::lookup_tenant(&req.0.input.api_token).await;
    CtxReq(Ctx { tenant, ..req.0 })
}
