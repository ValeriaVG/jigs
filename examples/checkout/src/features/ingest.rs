use crate::store;
use crate::types::Ctx;
use jigs::{jig, Request};

#[jig]
async fn authenticate(req: Request<Ctx>) -> Request<Ctx> {
    let user = store::lookup_user(&req.0.input.token).await;
    Request(Ctx { user, ..req.0 })
}

#[jig]
async fn load_cart(req: Request<Ctx>) -> Request<Ctx> {
    let lookups = req.0.input.items.iter().map(|(sku, _)| {
        let sku = sku.clone();
        async move {
            let info = store::lookup_sku(&sku).await;
            (sku, info)
        }
    });
    let results = futures::future::join_all(lookups).await;
    let mut stock = std::collections::HashMap::new();
    let mut prices = std::collections::HashMap::new();
    for (sku, info) in results {
        if let Some((s, p)) = info {
            stock.insert(sku.clone(), s);
            prices.insert(sku, p);
        }
    }
    Request(Ctx {
        stock,
        prices,
        ..req.0
    })
}

#[jig]
pub async fn ingest(req: Request<Ctx>) -> Request<Ctx> {
    req.then(authenticate).then(load_cart).await
}
