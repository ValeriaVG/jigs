use crate::http::HttpResponse;
use crate::types::{
    json_err, Authed, AuthedReq, Credentials, CredentialsReq, HttpResp, Issued, IssuedReq, Raw,
    RawReq,
};
use jigs::{fork, jig, Branch, Request, Response};
use serde::Deserialize;

#[derive(Deserialize)]
struct CredentialsDto {
    email: String,
    password: String,
}

#[jig]
fn parse_credentials(req: RawReq) -> Branch<CredentialsReq, HttpResp> {
    let dto: CredentialsDto = match serde_json::from_str(&req.0.body) {
        Ok(c) => c,
        Err(e) => {
            return Branch::Done(HttpResp::ok(HttpResponse::bad_request(json_err(
                &e.to_string(),
            ))));
        }
    };
    Branch::Continue(CredentialsReq(Credentials {
        email: dto.email,
        password: dto.password,
        store: req.0.store,
    }))
}

#[jig]
fn create_user(req: CredentialsReq) -> Branch<IssuedReq, HttpResp> {
    let Credentials {
        email,
        password,
        store,
    } = req.0;
    match store.signup(&email, &password) {
        Ok((user_id, token)) => Branch::Continue(IssuedReq(Issued { user_id, token })),
        Err(e) => Branch::Done(HttpResp::ok(HttpResponse::conflict(json_err(e)))),
    }
}

#[jig]
fn verify_credentials(req: CredentialsReq) -> Branch<IssuedReq, HttpResp> {
    let Credentials {
        email,
        password,
        store,
    } = req.0;
    match store.login(&email, &password) {
        Some((user_id, token)) => Branch::Continue(IssuedReq(Issued { user_id, token })),
        None => Branch::Done(HttpResp::ok(HttpResponse::unauthorized(json_err(
            "invalid credentials",
        )))),
    }
}

fn token_body(i: &Issued) -> String {
    serde_json::json!({ "user_id": i.user_id, "token": i.token }).to_string()
}

#[jig]
fn render_created_token(req: IssuedReq) -> HttpResp {
    HttpResp::ok(HttpResponse::created(token_body(&req.0)))
}

#[jig]
fn render_existing_token(req: IssuedReq) -> HttpResp {
    HttpResp::ok(HttpResponse::ok(token_body(&req.0)))
}

#[jig]
pub fn authenticate(req: RawReq) -> Branch<AuthedReq, HttpResp> {
    let token = req
        .0
        .headers
        .get("authorization")
        .and_then(|v| v.strip_prefix("Bearer ").map(str::to_string));
    match token.and_then(|t| req.0.store.user_for_token(&t)) {
        Some(user_id) => Branch::Continue(AuthedReq(Authed {
            user_id,
            store: req.0.store,
            body: req.0.body,
            path: req.0.path,
        })),
        None => Branch::Done(HttpResp::ok(HttpResponse::unauthorized(json_err(
            "missing or invalid token",
        )))),
    }
}

#[jig]
fn signup(req: RawReq) -> HttpResp {
    req.then(parse_credentials)
        .then(create_user)
        .then(render_created_token)
}

#[jig]
fn login(req: RawReq) -> HttpResp {
    req.then(parse_credentials)
        .then(verify_credentials)
        .then(render_existing_token)
}

#[jig]
pub fn auth(req: RawReq) -> HttpResp {
    fork!(req,
        |r: &Raw| r.method == "POST" && r.path == "/auth/signup" => signup,
        |r: &Raw| r.method == "POST" && r.path == "/auth/login"  => login,
        _ => crate::pipeline::not_found,
    )
}
