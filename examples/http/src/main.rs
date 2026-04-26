mod http;

use http::{read_request, write_response, HttpRequest, HttpResponse};
use jigs::{jig, trace::Entry, Branch, Request, Response};
use std::net::TcpListener;

#[jig]
fn log_incoming(req: Request<HttpRequest>) -> Request<HttpRequest> {
    println!("--> {} {}", req.0.method, req.0.path);
    req
}

#[jig]
fn only_get(req: Request<HttpRequest>) -> Branch<HttpRequest, HttpResponse> {
    if req.0.method == "GET" {
        Branch::Continue(req)
    } else {
        Branch::Done(Response::ok(HttpResponse::method_not_allowed()))
    }
}

#[jig]
fn root_route(req: Request<HttpRequest>) -> Branch<HttpRequest, HttpResponse> {
    if req.0.path == "/" {
        return Branch::Done(Response::ok(HttpResponse::ok("hello, world\n")));
    }
    Branch::Continue(req)
}

#[jig]
fn hello_route(req: Request<HttpRequest>) -> Branch<HttpRequest, HttpResponse> {
    let Some(name) = req.0.path.strip_prefix("/hello/") else {
        return Branch::Continue(req);
    };
    if name.is_empty() || name.contains('/') {
        return Branch::Done(Response::ok(HttpResponse::bad_request(
            "provide a single name segment\n",
        )));
    }
    Branch::Done(Response::ok(HttpResponse::ok(format!("hello, {name}\n"))))
}

#[jig]
fn not_found(_req: Request<HttpRequest>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::not_found("not found\n"))
}

#[jig]
fn route(req: Request<HttpRequest>) -> Response<HttpResponse> {
    req.then(root_route).then(hello_route).then(not_found)
}

#[jig]
fn log_outbound(res: Response<HttpResponse>) -> Response<HttpResponse> {
    if let Ok(r) = res.inner.as_ref() {
        println!("<-- {} {}", r.status, r.reason);
    }
    res
}

#[jig]
fn handle(request: Request<HttpRequest>) -> Response<HttpResponse> {
    request
        .then(log_incoming)
        .then(only_get)
        .then(route)
        .then(log_outbound)
}

fn main() -> std::io::Result<()> {
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

        let response = handle(Request(request));
        let body = match response.inner {
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
        print!("{}", render(&jigs::trace::take()));
    }
    Ok(())
}

fn render(entries: &[Entry]) -> String {
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
        out.push_str(&format!(
            "{}{}  {}  {}\n",
            label,
            " ".repeat(pad),
            mark,
            detail
        ));
    }
    out
}
