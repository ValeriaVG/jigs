mod http;

use http::{read_request, write_response, HttpRequest, HttpResponse};
use jigs::{fork, jig, jigs, Branch, Request, Response};
use std::net::TcpListener;

#[derive(Clone, Request)]
struct HttpReq(HttpRequest);

#[derive(Clone, Response)]
struct HttpResp(Result<HttpResponse, String>);

#[jig]
fn log_incoming(req: HttpReq) -> HttpReq {
    println!("--> {} {}", req.0.method, req.0.path);
    req
}

#[jig]
fn only_get(req: HttpReq) -> Branch<HttpReq, HttpResp> {
    if req.0.method == "GET" {
        Branch::Continue(req)
    } else {
        Branch::Done(HttpResp::ok(HttpResponse::method_not_allowed()))
    }
}

fn is_root(r: &HttpRequest) -> bool {
    r.path == "/"
}
#[jig]
fn root(_req: HttpReq) -> HttpResp {
    HttpResp::ok(HttpResponse::ok("hello, world\n"))
}

fn is_hello(r: &HttpRequest) -> bool {
    r.path.strip_prefix("/hello/").is_some()
}
#[jig]
fn hello(req: HttpReq) -> HttpResp {
    let name = req.0.path.strip_prefix("/hello/").unwrap_or("");
    if name.is_empty() || name.contains('/') {
        return HttpResp::ok(HttpResponse::bad_request("provide a single name segment\n"));
    }
    HttpResp::ok(HttpResponse::ok(format!("hello, {name}\n")))
}

#[jig]
fn not_found(_req: HttpReq) -> HttpResp {
    HttpResp::ok(HttpResponse::not_found("not found\n"))
}

#[jig]
fn route(req: HttpReq) -> HttpResp {
    fork!(req,
        is_root  => root,
        is_hello => hello,
        _ => not_found,
    )
}

#[jig]
fn log_outbound(res: HttpResp) -> HttpResp {
    if let Ok(r) = res.0.as_ref() {
        println!("<-- {} {}", r.status, r.reason);
    }
    res
}

#[jig]
fn handle(request: HttpReq) -> HttpResp {
    request
        .then(log_incoming)
        .then(only_get)
        .then(route)
        .then(log_outbound)
}

jigs!(handle);

fn main() -> std::io::Result<()> {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/http");
        std::fs::create_dir_all(&html_dir)?;
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                all_jigs(),
                "http example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{rel_path}#L{line}"),
            ),
        )?;
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(all_jigs(), "http example"),
        )?;
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return Ok(());
    }
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".into());
    let listener = TcpListener::bind(&addr)?;
    println!("listening on http://{addr}");
    println!("try: curl http://{addr}/hello/jigs");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };

        let request = match read_request(&mut stream) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("read error: {e}");
                continue;
            }
        };

        let response = handle(HttpReq(request));
        let body = match response.0 {
            Ok(r) => r,
            Err(msg) => HttpResponse {
                status: 500,
                reason: "Internal Server Error",
                body: msg,
            },
        };
        if let Err(e) = write_response(&mut stream, &body) {
            eprintln!("write error: {e}");
        }
        let entries = jigs::trace::take();
        if std::env::var("JIGS_LOG_JSON").is_ok() {
            print!("{}", jigs::log::render_ndjson(&entries));
        } else {
            print!("{}", jigs::log::render_tree(&entries));
        }
    }
    Ok(())
}
