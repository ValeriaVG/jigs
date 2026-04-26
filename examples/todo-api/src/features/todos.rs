use crate::features::auth::authenticate;
use crate::http::HttpResponse;
use crate::store::{Store, Todo};
use crate::types::{json_err, path_segments, Authed, Raw};
use jigs::{fork, jig, Branch, Request, Response};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct NewTodoDto {
    title: String,
}

#[derive(Deserialize)]
struct TodoUpdateDto {
    title: Option<String>,
    done: Option<bool>,
}

struct NewTodo {
    user_id: u64,
    store: Arc<Store>,
    title: String,
}

struct TodoLookup {
    user_id: u64,
    store: Arc<Store>,
    id: u64,
}

struct TodoUpdate {
    user_id: u64,
    store: Arc<Store>,
    id: u64,
    title: Option<String>,
    done: Option<bool>,
}

struct LabelOp {
    user_id: u64,
    store: Arc<Store>,
    todo_id: u64,
    label_id: u64,
}

struct OneTodo(Todo);
struct ManyTodos(Vec<Todo>);
struct Removed;

fn bad(msg: String) -> Response<HttpResponse> {
    Response::ok(HttpResponse::bad_request(msg))
}

fn missing(what: &str) -> Response<HttpResponse> {
    Response::ok(HttpResponse::not_found(json_err(what)))
}

#[jig]
fn parse_new_todo(req: Request<Authed>) -> Branch<NewTodo, HttpResponse> {
    let dto: NewTodoDto = match serde_json::from_str(&req.0.body) {
        Ok(d) => d,
        Err(e) => return Branch::Done(bad(json_err(&e.to_string()))),
    };
    Branch::Continue(Request(NewTodo {
        user_id: req.0.user_id,
        store: req.0.store,
        title: dto.title,
    }))
}

#[jig]
fn parse_todo_id(req: Request<Authed>) -> Branch<TodoLookup, HttpResponse> {
    let segs = path_segments(&req.0.path);
    match segs.get(1).and_then(|s| s.parse::<u64>().ok()) {
        Some(id) => Branch::Continue(Request(TodoLookup {
            user_id: req.0.user_id,
            store: req.0.store,
            id,
        })),
        None => Branch::Done(bad(json_err("invalid todo id"))),
    }
}

#[jig]
fn parse_todo_update(req: Request<Authed>) -> Branch<TodoUpdate, HttpResponse> {
    let dto: TodoUpdateDto = match serde_json::from_str(&req.0.body) {
        Ok(d) => d,
        Err(e) => return Branch::Done(bad(json_err(&e.to_string()))),
    };
    let segs = path_segments(&req.0.path);
    let Some(id) = segs.get(1).and_then(|s| s.parse::<u64>().ok()) else {
        return Branch::Done(bad(json_err("invalid todo id")));
    };
    Branch::Continue(Request(TodoUpdate {
        user_id: req.0.user_id,
        store: req.0.store,
        id,
        title: dto.title,
        done: dto.done,
    }))
}

#[jig]
fn parse_label_op(req: Request<Authed>) -> Branch<LabelOp, HttpResponse> {
    let segs = path_segments(&req.0.path);
    let parsed = segs
        .get(1)
        .and_then(|s| s.parse::<u64>().ok())
        .zip(segs.get(3).and_then(|s| s.parse::<u64>().ok()));
    match parsed {
        Some((todo_id, label_id)) => Branch::Continue(Request(LabelOp {
            user_id: req.0.user_id,
            store: req.0.store,
            todo_id,
            label_id,
        })),
        None => Branch::Done(bad(json_err("invalid path"))),
    }
}

#[jig]
fn load_todos(req: Request<Authed>) -> Request<ManyTodos> {
    Request(ManyTodos(req.0.store.list_todos(req.0.user_id)))
}

#[jig]
fn load_todo(req: Request<TodoLookup>) -> Branch<OneTodo, HttpResponse> {
    match req.0.store.get_todo(req.0.user_id, req.0.id) {
        Some(t) => Branch::Continue(Request(OneTodo(t))),
        None => Branch::Done(missing("todo not found")),
    }
}

#[jig]
fn insert_todo(req: Request<NewTodo>) -> Request<OneTodo> {
    let NewTodo {
        user_id,
        store,
        title,
    } = req.0;
    Request(OneTodo(store.create_todo(user_id, title)))
}

#[jig]
fn apply_update(req: Request<TodoUpdate>) -> Branch<OneTodo, HttpResponse> {
    let TodoUpdate {
        user_id,
        store,
        id,
        title,
        done,
    } = req.0;
    match store.update_todo(user_id, id, title, done) {
        Some(t) => Branch::Continue(Request(OneTodo(t))),
        None => Branch::Done(missing("todo not found")),
    }
}

#[jig]
fn remove_todo(req: Request<TodoLookup>) -> Branch<Removed, HttpResponse> {
    if req.0.store.delete_todo(req.0.user_id, req.0.id) {
        Branch::Continue(Request(Removed))
    } else {
        Branch::Done(missing("todo not found"))
    }
}

#[jig]
fn attach(req: Request<LabelOp>) -> Branch<OneTodo, HttpResponse> {
    let LabelOp {
        user_id,
        store,
        todo_id,
        label_id,
    } = req.0;
    match store.attach_label(user_id, todo_id, label_id) {
        Some(t) => Branch::Continue(Request(OneTodo(t))),
        None => Branch::Done(missing("todo or label not found")),
    }
}

#[jig]
fn detach(req: Request<LabelOp>) -> Branch<OneTodo, HttpResponse> {
    let LabelOp {
        user_id,
        store,
        todo_id,
        label_id,
    } = req.0;
    match store.detach_label(user_id, todo_id, label_id) {
        Some(t) => Branch::Continue(Request(OneTodo(t))),
        None => Branch::Done(missing("todo not found")),
    }
}

#[jig]
fn render_one(req: Request<OneTodo>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::ok(serde_json::to_string(&req.0 .0).unwrap()))
}

#[jig]
fn render_one_created(req: Request<OneTodo>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::created(
        serde_json::to_string(&req.0 .0).unwrap(),
    ))
}

#[jig]
fn render_many(req: Request<ManyTodos>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::ok(serde_json::to_string(&req.0 .0).unwrap()))
}

#[jig]
fn render_removed(_req: Request<Removed>) -> Response<HttpResponse> {
    Response::ok(HttpResponse::no_content())
}

fn is_list(r: &Raw) -> bool {
    r.method == "GET" && r.path == "/todos"
}
#[jig]
fn list(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate).then(load_todos).then(render_many)
}

fn is_create(r: &Raw) -> bool {
    r.method == "POST" && r.path == "/todos"
}
#[jig]
fn create(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_new_todo)
        .then(insert_todo)
        .then(render_one_created)
}

fn is_get(r: &Raw) -> bool {
    r.method == "GET" && path_segments(&r.path).len() == 2
}
#[jig]
fn get(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_todo_id)
        .then(load_todo)
        .then(render_one)
}

fn is_update(r: &Raw) -> bool {
    r.method == "PUT" && path_segments(&r.path).len() == 2
}
#[jig]
fn update(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_todo_update)
        .then(apply_update)
        .then(render_one)
}

fn is_delete(r: &Raw) -> bool {
    r.method == "DELETE" && path_segments(&r.path).len() == 2
}
#[jig]
fn delete(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_todo_id)
        .then(remove_todo)
        .then(render_removed)
}

fn is_attach(r: &Raw) -> bool {
    let s = path_segments(&r.path);
    r.method == "POST" && s.len() == 4 && s[2] == "labels"
}
#[jig]
fn attach_label(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_label_op)
        .then(attach)
        .then(render_one)
}

fn is_detach(r: &Raw) -> bool {
    let s = path_segments(&r.path);
    r.method == "DELETE" && s.len() == 4 && s[2] == "labels"
}
#[jig]
fn detach_label(req: Request<Raw>) -> Response<HttpResponse> {
    req.then(authenticate)
        .then(parse_label_op)
        .then(detach)
        .then(render_one)
}

#[jig]
pub fn todos(req: Request<Raw>) -> Response<HttpResponse> {
    fork!(req,
        is_list   => list,
        is_create => create,
        is_get    => get,
        is_update => update,
        is_delete => delete,
        is_attach => attach_label,
        is_detach => detach_label,
        _ => crate::pipeline::not_found,
    )
}
