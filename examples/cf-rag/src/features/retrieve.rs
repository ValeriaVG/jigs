use crate::bindings;
use crate::types::Ctx;
use jigs::{jig, Request};

const TOP_K: usize = 5;
const MIN_SCORE: f32 = 0.55;
const SPAM_TAG: &str = "spam";

#[jig]
async fn embed_query(req: Request<Ctx>) -> Request<Ctx> {
    let embedding = bindings::embed(&req.0.input.query).await;
    Request(Ctx { embedding, ..req.0 })
}

#[jig]
async fn vector_search(req: Request<Ctx>) -> Request<Ctx> {
    let tenant_id = req.0.tenant.as_ref().expect("guard").id;
    let candidates = bindings::vector_query(&req.0.embedding, TOP_K, tenant_id).await;
    Request(Ctx {
        candidates,
        ..req.0
    })
}

// Drop low-confidence and known-spam docs, keep an ordered shortlist.
#[jig]
fn filter_and_rerank(req: Request<Ctx>) -> Request<Ctx> {
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
    Request(Ctx { context, ..req.0 })
}

#[jig]
pub async fn retrieve(req: Request<Ctx>) -> Request<Ctx> {
    req.then(embed_query)
        .then(vector_search)
        .await
        .then(filter_and_rerank)
}
