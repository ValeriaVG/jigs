use jigs::{jig, trace::Entry, Branch, Request, Response};

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
    Response::ok(resp.inner.unwrap().to_uppercase())
}

#[jig]
fn log_outbound(res: Response<String>) -> Response<String> {
    println!("result: {}", res.inner.as_ref().unwrap());
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
    let name = std::env::args()
        .nth(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "".to_string());
    let response = handle(Request(name));
    match response.inner {
        Ok(v) => println!("{v}"),
        Err(e) => println!("ERROR: {e}"),
    }
    print!("{}", render(&jigs::trace::take()));
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
        let mark = if e.ok { "✓" } else { "✗" };
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
