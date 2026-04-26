use crate::http::HttpResponse;
use crate::store::Store;
use crate::types::{json_err, Authed, Raw};
use jigs::{fork, jig, Branch, Request, Response};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct CredentialsDto {
    email: String,
    password: String,
}

struct Credentials {
    email: String,
    password: String,
    store: Arc<Store>,
}

struct Issued {
    user_id: u64,
    token: String,
}

#[jig]
fn parse_credentials(req: Request<Raw>) -> Branch<Credentials, HttpResponse> {
    let dto: CredentialsDto = match serde_json::from_str(&req.0.body) {
        Ok(c) => c,
        Err(e) => {
            return Branch::Done(Response::ok(HttpResponse::bad_request(json_err(
                &e.to_string(),
            ))));
        }
    };
    Branch::Continue(Request(Credentials {
        email: dto.email,
        password: dto.password,
        store: req.0.store,
    }))
}

#[jig]
fn create_user(req: Request<Credentials>) -> Branch<Issued, HttpResponse> {
    let Credentials {
        email,
        password,
        store,
    } = req.0;
    match store.signup(&email, &password) {
        Ok((user_id, token)) => Branch::Continue(Request(Issued { user_id, token })),
        Err(e) => Branch::Done(Response::ok(HttpResponse::conflict(json_err(e)))),
    }
}

#[jig]
fn verify_credentials(req: Request<Credentials>) -> Branch<Issued, HttpResponse> {
    let Credentials {
        email,
        password,
        store,
    } = req.0;
    match store.login(&email, &password) {
        Some((user_id, token)) => Branch::Continue(Request(Issued { user_id, token })),
        None => Branch::Done(Response::ok(HttpResponse::unauthorized(json_err(
            "invalid credentials",
        )))),
    }
}

fn token_body(i: &Issued) -> String {
    serde_json::json!({ "user_id": i.user_id, "token": i.token }).to_string()
}

#[jig]
fn render_created_token(req: Request<Issued>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::created(token_body(&req.0)))
}

#[jig]
fn render_existing_token(req: Request<Issued>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::ok(token_body(&req.0)))
}

#[jig]
pub fn authenticate(req: Request<Raw>) -> Branch<Authed, HttpResponse> {
    let token = req
        .0
        .headers
        .get("authorization")
        .and_then(|v| v.strip_prefix("Bearer ").map(str::to_string));
    match token.and_then(|t| req.0.store.user_for_token(&t)) {
        Some(user_id) => Branch::Continue(Request(Authed {
            user_id,
            store: req.0.store,
            body: req.0.body,
            path: req.0.path,
        })),
        None => Branch::Done(Response::ok(HttpResponse::unauthorized(json_err(
            "missing or invalid token",
        )))),
    }
}

#[jig]
fn signup(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(parse_credentials)
        .then(create_user)
        .then(render_created_token)
}

#[jig]
fn login(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(parse_credentials)
        .then(verify_credentials)
        .then(render_existing_token)
}

#[jig]
pub fn auth(req: Request<Raw>) -> Response<HttpResponse> {
    fork!(req,
        |r: &Raw| r.method == "POST" && r.path == "/auth/signup" => signup,
        |r: &Raw| r.method == "POST" && r.path == "/auth/login"  => login,
        _ => crate::pipeline::not_found,
    )
}
