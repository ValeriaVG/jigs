use jigs::{jig, Branch, Request, Response};

#[jig]
fn log_incoming(req: Request<String>) -> Request<String> {
    println!("incoming: {}", req.0);
    req
}

#[jig]
fn validate_incoming(req: Request<String>) -> Branch<String, String> {
    if req.0.is_empty() {
        return Branch::Done(Response::err("empty name provided"));
    }
    Branch::Continue(req)
}

#[jig]
fn greet(req: Request<String>) -> Response<String> {
    Response::ok(format!("hello, {}", req.0))
}

#[jig]
fn shout(resp: Response<String>) -> Response<String> {
    match resp.inner {
        Ok(v) => Response::ok(v.to_uppercase()),
        Err(_) => resp,
    }
}

#[jig]
fn log_outbound(res: Response<String>) -> Response<String> {
    match res.inner.as_ref() {
        Ok(v) => println!("result: {v}"),
        Err(e) => println!("error: {e}"),
    }
    res
}

#[jig]
fn handle(request: Request<String>) -> Response<String> {
    request
        .then(log_incoming)
        .then(validate_incoming)
        .then(greet)
        .then(shout)
        .then(log_outbound)
}

fn main() {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/hello");
        std::fs::create_dir_all(&html_dir).expect("create docs/hello");
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                Some("handle"),
                "hello example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{rel_path}#L{line}"),
            ),
        )
        .expect("write index.html");
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(Some("handle"), "hello example"),
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
    let response = handle(Request(name));
    match response.inner {
        Ok(v) => println!("{v}"),
        Err(e) => println!("ERROR: {e}"),
    }
    let entries = jigs::trace::take();
    if std::env::var("JIGS_LOG_JSON").is_ok() {
        print!("{}", jigs::log::render_ndjson(&entries));
    } else {
        print!("{}", jigs::log::render_tree(&entries));
    }
}
