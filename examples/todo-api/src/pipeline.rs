use crate::features::{auth::auth, labels::labels, todos::todos};
use crate::http::HttpResponse;
use crate::types::{json_err, Raw};
use jigs::{fork, jig, Request, Response};

#[jig]
fn log_incoming(req: Request<Raw>) -> Request<Raw> {
    println!(
        "[t{:?}] --> {} {}",
        std::thread::current().id(),
        req.0.method,
        req.0.path
    );
    req
}

#[jig]
pub fn not_found(_req: Request<Raw>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::not_found(json_err("not found")))
}

#[jig]
fn log_outbound(res: Response<HttpResponse>) -> Response<HttpResponse> {
    if let Ok(r) = res.inner.as_ref() {
        println!(
            "[t{:?}] <-- {} {}",
            std::thread::current().id(),
            r.status,
            r.reason
        );
    }
    res
}

#[jig]
fn dispatch(req: Request<Raw>) -> Response<HttpResponse> {
    fork!(req,
        |r: &Raw| r.path.starts_with("/auth/")  => auth,
        |r: &Raw| r.path.starts_with("/todos")  => todos,
        |r: &Raw| r.path.starts_with("/labels") => labels,
        _ => not_found,
    )
}

#[jig]
pub fn handle(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(log_incoming).then(dispatch).then(log_outbound)
}
