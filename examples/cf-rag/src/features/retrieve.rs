use crate::bindings;
use crate::types::{Ctx, CtxReq};
use jigs::{jig, Request};

const TOP_K: usize = 5;
const MIN_SCORE: f32 = 0.55;
const SPAM_TAG: &str = "spam";

#[jig]
async fn embed_query(req: CtxReq) -> CtxReq {
    let embedding = bindings::embed(&req.0.input.query).await;
    CtxReq(Ctx { embedding, ..req.0 })
}

#[jig]
async fn vector_search(req: CtxReq) -> CtxReq {
    let tenant_id = req.0.tenant.as_ref().expect("guard").id;
    let candidates = bindings::vector_query(&req.0.embedding, TOP_K, tenant_id).await;
    CtxReq(Ctx {
        candidates,
        ..req.0
    })
}

#[jig]
fn filter_and_rerank(req: CtxReq) -> CtxReq {
    let mut context: Vec<_> = req
        .0
        .candidates
        .iter()
        .filter(|d| d.score >= MIN_SCORE && !d.id.contains(SPAM_TAG))
        .cloned()
        .collect();
    context.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    context.truncate(3);
    CtxReq(Ctx { context, ..req.0 })
}

#[jig]
pub async fn retrieve(req: CtxReq) -> CtxReq {
    req.then(embed_query)
        .then(vector_search)
        .await
        .then(filter_and_rerank)
}
