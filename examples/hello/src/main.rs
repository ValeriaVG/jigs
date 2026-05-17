use jigs::{jig, jigs, Branch, Request, Response};

#[derive(Clone, Request)]
struct NameReq(String);

#[derive(Clone, Response)]
struct GreetingResp(Result<String, String>);

#[jig]
fn log_incoming(req: NameReq) -> NameReq {
    println!("incoming: {}", req.0);
    req
}

#[jig]
fn validate_incoming(req: NameReq) -> Branch<NameReq, GreetingResp> {
    if req.0.is_empty() {
        return Branch::Done(GreetingResp::err("empty name provided"));
    }
    Branch::Continue(req)
}

#[jig]
fn greet(req: NameReq) -> GreetingResp {
    GreetingResp::ok(format!("hello, {}", req.0))
}

#[jig]
fn shout(resp: GreetingResp) -> GreetingResp {
    match resp.0 {
        Ok(v) => GreetingResp::ok(v.to_uppercase()),
        Err(_) => resp,
    }
}

#[jig]
fn log_outbound(res: GreetingResp) -> GreetingResp {
    match &res.0 {
        Ok(v) => println!("result: {v}"),
        Err(e) => println!("error: {e}"),
    }
    res
}

#[jig]
fn handle(request: NameReq) -> GreetingResp {
    request
        .then(log_incoming)
        .then(validate_incoming)
        .then(greet)
        .then(shout)
        .then(log_outbound)
}

jigs!(handle);

fn main() {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/hello");
        std::fs::create_dir_all(&html_dir).expect("create docs/hello");
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                all_jigs(),
                "hello example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{rel_path}#L{line}"),
            ),
        )
        .expect("write index.html");
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(all_jigs(), "hello example"),
        )
        .expect("write map.md");
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return;
    }
    let name = std::env::args()
        .nth(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    let _response = handle(NameReq(name));
    let entries = jigs::trace::take();
    if std::env::var("JIGS_LOG_JSON").is_ok() {
        print!("{}", jigs::log::render_ndjson(&entries));
    } else {
        print!("{}", jigs::log::render_tree(&entries));
    }
}
