use crate::features::auth::authenticate;
use crate::http::HttpResponse;
use crate::store::{Label, Store};
use crate::types::{json_err, path_segments, Authed, Raw};
use jigs::{fork, jig, Branch, Request, Response};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct LabelDto {
    name: String,
}

struct NewLabel {
    user_id: u64,
    store: Arc<Store>,
    name: String,
}

struct LabelLookup {
    user_id: u64,
    store: Arc<Store>,
    id: u64,
}

struct LabelUpdate {
    user_id: u64,
    store: Arc<Store>,
    id: u64,
    name: String,
}

struct OneLabel(Label);
struct ManyLabels(Vec<Label>);
struct Removed;

fn bad(msg: String) -> Response<HttpResponse> {
    Response::ok(HttpResponse::bad_request(msg))
}

fn missing() -> Response<HttpResponse> {
    Response::ok(HttpResponse::not_found(json_err("label not found")))
}

#[jig]
fn parse_new_label(req: Request<Authed>) -> Branch<NewLabel, HttpResponse> {
    let dto: LabelDto = match serde_json::from_str(&req.0.body) {
        Ok(d) => d,
        Err(e) => return Branch::Done(bad(json_err(&e.to_string()))),
    };
    Branch::Continue(Request(NewLabel {
        user_id: req.0.user_id,
        store: req.0.store,
        name: dto.name,
    }))
}

#[jig]
fn parse_label_id(req: Request<Authed>) -> Branch<LabelLookup, HttpResponse> {
    let segs = path_segments(&req.0.path);
    match segs.get(1).and_then(|s| s.parse::<u64>().ok()) {
        Some(id) => Branch::Continue(Request(LabelLookup {
            user_id: req.0.user_id,
            store: req.0.store,
            id,
        })),
        None => Branch::Done(bad(json_err("invalid label id"))),
    }
}

#[jig]
fn parse_label_update(req: Request<Authed>) -> Branch<LabelUpdate, HttpResponse> {
    let dto: LabelDto = match serde_json::from_str(&req.0.body) {
        Ok(d) => d,
        Err(e) => return Branch::Done(bad(json_err(&e.to_string()))),
    };
    let segs = path_segments(&req.0.path);
    let Some(id) = segs.get(1).and_then(|s| s.parse::<u64>().ok()) else {
        return Branch::Done(bad(json_err("invalid label id")));
    };
    Branch::Continue(Request(LabelUpdate {
        user_id: req.0.user_id,
        store: req.0.store,
        id,
        name: dto.name,
    }))
}

#[jig]
fn load_labels(req: Request<Authed>) -> Request<ManyLabels> {
    Request(ManyLabels(req.0.store.list_labels(req.0.user_id)))
}

#[jig]
fn insert_label(req: Request<NewLabel>) -> Request<OneLabel> {
    let NewLabel {
        user_id,
        store,
        name,
    } = req.0;
    Request(OneLabel(store.create_label(user_id, name)))
}

#[jig]
fn apply_label_update(req: Request<LabelUpdate>) -> Branch<OneLabel, HttpResponse> {
    let LabelUpdate {
        user_id,
        store,
        id,
        name,
    } = req.0;
    match store.update_label(user_id, id, name) {
        Some(l) => Branch::Continue(Request(OneLabel(l))),
        None => Branch::Done(missing()),
    }
}

#[jig]
fn remove_label(req: Request<LabelLookup>) -> Branch<Removed, HttpResponse> {
    if req.0.store.delete_label(req.0.user_id, req.0.id) {
        Branch::Continue(Request(Removed))
    } else {
        Branch::Done(missing())
    }
}

#[jig]
fn render_one_label(req: Request<OneLabel>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::ok(serde_json::to_string(&req.0 .0).unwrap()))
}

#[jig]
fn render_one_label_created(req: Request<OneLabel>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::created(
        serde_json::to_string(&req.0 .0).unwrap(),
    ))
}

#[jig]
fn render_many_labels(req: Request<ManyLabels>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::ok(serde_json::to_string(&req.0 .0).unwrap()))
}

#[jig]
fn render_label_removed(_req: Request<Removed>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::no_content())
}

fn is_list_labels(r: &Raw) -> bool {
    r.method == "GET" && r.path == "/labels"
}
#[jig]
fn list_labels(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(load_labels)
        .then(render_many_labels)
}

fn is_create_label(r: &Raw) -> bool {
    r.method == "POST" && r.path == "/labels"
}
#[jig]
fn create_label(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_new_label)
        .then(insert_label)
        .then(render_one_label_created)
}

fn is_update_label(r: &Raw) -> bool {
    let s = path_segments(&r.path);
    r.method == "PUT" && s.len() == 2 && s[0] == "labels"
}
#[jig]
fn update_label(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_label_update)
        .then(apply_label_update)
        .then(render_one_label)
}

fn is_delete_label(r: &Raw) -> bool {
    let s = path_segments(&r.path);
    r.method == "DELETE" && s.len() == 2 && s[0] == "labels"
}
#[jig]
fn delete_label(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_label_id)
        .then(remove_label)
        .then(render_label_removed)
}

#[jig]
pub fn labels(req: Request<Raw>) -> Response<HttpResponse> {
    fork!(req,
        is_list_labels   => list_labels,
        is_create_label  => create_label,
        is_update_label  => update_label,
        is_delete_label  => delete_label,
        _ => crate::pipeline::not_found,
    )
}
