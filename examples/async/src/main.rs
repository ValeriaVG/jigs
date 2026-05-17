use jigs::{jig, jigs, Branch, Request, Response};
use std::time::Duration;

#[derive(Clone)]
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
    id.bytes().map(u64::from).sum()
}

#[derive(Clone, Request)]
struct AppReq(Ctx);

#[derive(Clone, Response)]
struct AppResp(Result<String, String>);

#[jig]
fn log_incoming(req: AppReq) -> AppReq {
    println!("--> incoming id={}", req.0.id);
    req
}

#[jig]
async fn enrich(req: AppReq) -> AppReq {
    let (user, account) = tokio::join!(fetch_user(&req.0.id), fetch_account(&req.0.id));
    AppReq(Ctx {
        id: req.0.id,
        user: Some(user),
        account: Some(account),
    })
}

#[jig]
fn require_account(req: AppReq) -> Branch<AppReq, AppResp> {
    if req.0.account.is_some() {
        Branch::Continue(req)
    } else {
        Branch::Done(AppResp::err("no account"))
    }
}

#[jig]
fn render(req: AppReq) -> AppResp {
    match (req.0.user, req.0.account) {
        (Some(u), Some(a)) => AppResp::ok(format!("{u} balance={a}")),
        _ => AppResp::err("missing fields"),
    }
}

#[jig]
fn log_outbound(res: AppResp) -> AppResp {
    match &res.0 {
        Ok(body) => println!("<-- ok: {body}"),
        Err(msg) => println!("<-- err: {msg}"),
    }
    res
}

#[jig]
async fn handle(req: AppReq) -> AppResp {
    let req = req.then(log_incoming).then(enrich).await;
    req.then(require_account).then(render).then(log_outbound)
}

jigs!(handle);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/async");
        std::fs::create_dir_all(&html_dir).expect("create docs/async");
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                all_jigs(),
                "async example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{rel_path}#L{line}"),
            ),
        )
        .expect("write index.html");
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(all_jigs(), "async example"),
        )
        .expect("write map.md");
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return;
    }
    let req = AppReq(Ctx {
        id: "u-42".to_string(),
        user: None,
        account: None,
    });
    let _ = handle(req).await;
    let entries = jigs::trace::take();
    if std::env::var("JIGS_LOG_JSON").is_ok() {
        print!("{}", jigs::log::render_ndjson(&entries));
    } else {
        print!("{}", jigs::log::render_tree(&entries));
    }
}
