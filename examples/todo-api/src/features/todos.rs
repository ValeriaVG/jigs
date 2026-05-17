use crate::features::auth;
use crate::http::HttpResponse;
use crate::types::{
    json_err, path_segments, AuthedReq, HttpResp, LabelOp, LabelOpReq, ManyTodos, ManyTodosReq,
    NewTodo, NewTodoReq, OneTodo, OneTodoReq, Raw, RawReq, Removed, RemovedReq, TodoLookup,
    TodoLookupReq, TodoUpdate, TodoUpdateReq,
};
use jigs::{fork, jig, Branch, Request, Response};
use serde::Deserialize;

#[derive(Deserialize)]
struct NewTodoDto {
    title: String,
}

#[derive(Deserialize)]
struct TodoUpdateDto {
    title: Option<String>,
    done: Option<bool>,
}

fn bad(msg: String) -> HttpResp {
    HttpResp::ok(HttpResponse::bad_request(msg))
}

fn missing(what: &str) -> HttpResp {
    HttpResp::ok(HttpResponse::not_found(json_err(what)))
}

#[jig]
fn parse_new_todo(req: AuthedReq) -> Branch<NewTodoReq, HttpResp> {
    let dto: NewTodoDto = match serde_json::from_str(&req.0.body) {
        Ok(d) => d,
        Err(e) => return Branch::Done(bad(json_err(&e.to_string()))),
    };
    Branch::Continue(NewTodoReq(NewTodo {
        user_id: req.0.user_id,
        store: req.0.store,
        title: dto.title,
    }))
}

#[jig]
fn parse_todo_id(req: AuthedReq) -> Branch<TodoLookupReq, HttpResp> {
    let segs = path_segments(&req.0.path);
    match segs.get(1).and_then(|s| s.parse::<u64>().ok()) {
        Some(id) => Branch::Continue(TodoLookupReq(TodoLookup {
            user_id: req.0.user_id,
            store: req.0.store,
            id,
        })),
        None => Branch::Done(bad(json_err("invalid todo id"))),
    }
}

#[jig]
fn parse_todo_update(req: AuthedReq) -> Branch<TodoUpdateReq, HttpResp> {
    let dto: TodoUpdateDto = match serde_json::from_str(&req.0.body) {
        Ok(d) => d,
        Err(e) => return Branch::Done(bad(json_err(&e.to_string()))),
    };
    let segs = path_segments(&req.0.path);
    let Some(id) = segs.get(1).and_then(|s| s.parse::<u64>().ok()) else {
        return Branch::Done(bad(json_err("invalid todo id")));
    };
    Branch::Continue(TodoUpdateReq(TodoUpdate {
        user_id: req.0.user_id,
        store: req.0.store,
        id,
        title: dto.title,
        done: dto.done,
    }))
}

#[jig]
fn parse_label_op(req: AuthedReq) -> Branch<LabelOpReq, HttpResp> {
    let segs = path_segments(&req.0.path);
    let parsed = segs
        .get(1)
        .and_then(|s| s.parse::<u64>().ok())
        .zip(segs.get(3).and_then(|s| s.parse::<u64>().ok()));
    match parsed {
        Some((todo_id, label_id)) => Branch::Continue(LabelOpReq(LabelOp {
            user_id: req.0.user_id,
            store: req.0.store,
            todo_id,
            label_id,
        })),
        None => Branch::Done(bad(json_err("invalid path"))),
    }
}

#[jig]
fn load_todos(req: AuthedReq) -> ManyTodosReq {
    ManyTodosReq(ManyTodos(req.0.store.list_todos(req.0.user_id)))
}

#[jig]
fn load_todo(req: TodoLookupReq) -> Branch<OneTodoReq, HttpResp> {
    match req.0.store.get_todo(req.0.user_id, req.0.id) {
        Some(t) => Branch::Continue(OneTodoReq(OneTodo(t))),
        None => Branch::Done(missing("todo not found")),
    }
}

#[jig]
fn insert_todo(req: NewTodoReq) -> OneTodoReq {
    let NewTodo {
        user_id,
        store,
        title,
    } = req.0;
    OneTodoReq(OneTodo(store.create_todo(user_id, title)))
}

#[jig]
fn apply_update(req: TodoUpdateReq) -> Branch<OneTodoReq, HttpResp> {
    let TodoUpdate {
        user_id,
        store,
        id,
        title,
        done,
    } = req.0;
    match store.update_todo(user_id, id, title, done) {
        Some(t) => Branch::Continue(OneTodoReq(OneTodo(t))),
        None => Branch::Done(missing("todo not found")),
    }
}

#[jig]
fn remove_todo(req: TodoLookupReq) -> Branch<RemovedReq, HttpResp> {
    if req.0.store.delete_todo(req.0.user_id, req.0.id) {
        Branch::Continue(RemovedReq(Removed))
    } else {
        Branch::Done(missing("todo not found"))
    }
}

#[jig]
fn attach(req: LabelOpReq) -> Branch<OneTodoReq, HttpResp> {
    let LabelOp {
        user_id,
        store,
        todo_id,
        label_id,
    } = req.0;
    match store.attach_label(user_id, todo_id, label_id) {
        Some(t) => Branch::Continue(OneTodoReq(OneTodo(t))),
        None => Branch::Done(missing("todo or label not found")),
    }
}

#[jig]
fn detach(req: LabelOpReq) -> Branch<OneTodoReq, HttpResp> {
    let LabelOp {
        user_id,
        store,
        todo_id,
        label_id,
    } = req.0;
    match store.detach_label(user_id, todo_id, label_id) {
        Some(t) => Branch::Continue(OneTodoReq(OneTodo(t))),
        None => Branch::Done(missing("todo not found")),
    }
}

#[jig]
fn render_one(req: OneTodoReq) -> HttpResp {
    HttpResp::ok(HttpResponse::ok(serde_json::to_string(&req.0 .0).unwrap()))
}

#[jig]
fn render_one_created(req: OneTodoReq) -> HttpResp {
    HttpResp::ok(HttpResponse::created(
        serde_json::to_string(&req.0 .0).unwrap(),
    ))
}

#[jig]
fn render_many(req: ManyTodosReq) -> HttpResp {
    HttpResp::ok(HttpResponse::ok(serde_json::to_string(&req.0 .0).unwrap()))
}

#[jig]
fn render_removed(_req: RemovedReq) -> HttpResp {
    HttpResp::ok(HttpResponse::no_content())
}

fn is_list(r: &Raw) -> bool {
    r.method == "GET" && r.path == "/todos"
}
#[jig]
fn list(req: RawReq) -> HttpResp {
    req.then(auth::authenticate)
        .then(load_todos)
        .then(render_many)
}

fn is_create(r: &Raw) -> bool {
    r.method == "POST" && r.path == "/todos"
}
#[jig]
fn create(req: RawReq) -> HttpResp {
    req.then(auth::authenticate)
        .then(parse_new_todo)
        .then(insert_todo)
        .then(render_one_created)
}

fn is_get(r: &Raw) -> bool {
    r.method == "GET" && path_segments(&r.path).len() == 2
}
#[jig]
fn get(req: RawReq) -> HttpResp {
    req.then(auth::authenticate)
        .then(parse_todo_id)
        .then(load_todo)
        .then(render_one)
}

fn is_update(r: &Raw) -> bool {
    r.method == "PUT" && path_segments(&r.path).len() == 2
}
#[jig]
fn update(req: RawReq) -> HttpResp {
    req.then(auth::authenticate)
        .then(parse_todo_update)
        .then(apply_update)
        .then(render_one)
}

fn is_delete(r: &Raw) -> bool {
    r.method == "DELETE" && path_segments(&r.path).len() == 2
}
#[jig]
fn delete(req: RawReq) -> HttpResp {
    req.then(auth::authenticate)
        .then(parse_todo_id)
        .then(remove_todo)
        .then(render_removed)
}

fn is_attach(r: &Raw) -> bool {
    let s = path_segments(&r.path);
    r.method == "POST" && s.len() == 4 && s[2] == "labels"
}
#[jig]
fn attach_label(req: RawReq) -> HttpResp {
    req.then(auth::authenticate)
        .then(parse_label_op)
        .then(attach)
        .then(render_one)
}

fn is_detach(r: &Raw) -> bool {
    let s = path_segments(&r.path);
    r.method == "DELETE" && s.len() == 4 && s[2] == "labels"
}
#[jig]
fn detach_label(req: RawReq) -> HttpResp {
    req.then(auth::authenticate)
        .then(parse_label_op)
        .then(detach)
        .then(render_one)
}

#[jig]
pub fn todos(req: RawReq) -> HttpResp {
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
