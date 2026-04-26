use jigs::{jig, trace::Entry, Branch, Request, Response};
use std::time::Duration;

struct Ctx {
    id: String,
    user: Option<String>,
    account: Option<u64>,
}

async fn fetch_user(id: &str) -> String {
    tokio::time::sleep(Duration::from_millis(50)).await;
    format!("user::{id}")
}

async fn fetch_account(id: &str) -> u64 {
    tokio::time::sleep(Duration::from_millis(30)).await;
    id.bytes().map(|b| b as u64).sum()
}

#[jig]
fn log_incoming(req: Request<Ctx>) -> Request<Ctx> {
    println!("--> incoming id={}", req.0.id);
    req
}

#[jig]
async fn enrich(req: Request<Ctx>) -> Request<Ctx> {
    let (user, account) = tokio::join!(fetch_user(&req.0.id), fetch_account(&req.0.id));
    Request(Ctx {
        id: req.0.id,
        user: Some(user),
        account: Some(account),
    })
}

#[jig]
fn require_account(req: Request<Ctx>) -> Branch<Ctx, String> {
    if req.0.account.is_some() {
        Branch::Continue(req)
    } else {
        Branch::Done(Response::err("no account"))
    }
}

#[jig]
fn render(req: Request<Ctx>) -> Response<String> {
    match (req.0.user, req.0.account) {
        (Some(u), Some(a)) => Response::ok(format!("{u} balance={a}")),
        _ => Response::err("missing fields"),
    }
}

#[jig]
fn log_outbound(res: Response<String>) -> Response<String> {
    match res.inner.as_ref() {
        Ok(body) => println!("<-- ok: {body}"),
        Err(msg) => println!("<-- err: {msg}"),
    }
    res
}

#[jig]
async fn handle(req: Request<Ctx>) -> Response<String> {
    // Async section: await collapses Pending back to a Request.
    let req = req.then(log_incoming).then(enrich).await;
    // Sync section: Branch + sync handlers chain normally on top.
    req.then(require_account)
        .then(render)
        .then(log_outbound)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let req = Request(Ctx {
        id: "u-42".to_string(),
        user: None,
        account: None,
    });
    let _ = handle(req).await;
    print!("{}", render_trace(&jigs::trace::take()));
}

fn render_trace(entries: &[Entry]) -> String {
    let labels: Vec<String> = entries
        .iter()
        .map(|e| {
            let indent = if e.depth == 0 {
                String::new()
            } else {
                format!("{}└─ ", "  ".repeat(e.depth - 1))
            };
            format!("{}{}", indent, e.name)
        })
        .collect();
    let width = labels.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let mut out = String::new();
    for (label, e) in labels.iter().zip(entries) {
        let pad = width - label.chars().count();
        let mark = if e.ok { "ok" } else { "err" };
        let detail = match &e.error {
            Some(msg) => format!("ERROR: {msg}"),
            None => format!("{:?}", e.duration),
        };
        out.push_str(&format!("{}{}  {}  {}\n", label, " ".repeat(pad), mark, detail));
    }
    out
}
