use crate::http::HttpResponse;
use crate::store::{Label, Store, Todo};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Raw {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub store: Arc<Store>,
}

#[derive(Clone)]
pub struct Authed {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub body: String,
    pub path: String,
}

pub fn json_err(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

pub fn path_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Clone)]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub store: Arc<Store>,
}

#[derive(Clone)]
pub struct Issued {
    pub user_id: u64,
    pub token: String,
}

#[derive(Clone)]
pub struct NewTodo {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub title: String,
}

#[derive(Clone)]
pub struct TodoLookup {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub id: u64,
}

#[derive(Clone)]
pub struct TodoUpdate {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub id: u64,
    pub title: Option<String>,
    pub done: Option<bool>,
}

#[derive(Clone)]
pub struct LabelOp {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub todo_id: u64,
    pub label_id: u64,
}

#[derive(Clone)]
pub struct OneTodo(pub Todo);

#[derive(Clone)]
pub struct ManyTodos(pub Vec<Todo>);

#[derive(Clone)]
pub struct Removed;

#[derive(Clone)]
pub struct NewLabel {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub name: String,
}

#[derive(Clone)]
pub struct LabelLookup {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub id: u64,
}

#[derive(Clone)]
pub struct LabelUpdate {
    pub user_id: u64,
    pub store: Arc<Store>,
    pub id: u64,
    pub name: String,
}

#[derive(Clone)]
pub struct OneLabel(pub Label);

#[derive(Clone)]
pub struct ManyLabels(pub Vec<Label>);

macro_rules! req_wrapper {
    ($name:ident, $payload:ty) => {
        #[derive(Clone, jigs::Request)]
        pub struct $name(pub $payload);
    };
}

req_wrapper!(RawReq, Raw);
req_wrapper!(AuthedReq, Authed);
req_wrapper!(CredentialsReq, Credentials);
req_wrapper!(IssuedReq, Issued);
req_wrapper!(NewTodoReq, NewTodo);
req_wrapper!(TodoLookupReq, TodoLookup);
req_wrapper!(TodoUpdateReq, TodoUpdate);
req_wrapper!(LabelOpReq, LabelOp);
req_wrapper!(OneTodoReq, OneTodo);
req_wrapper!(ManyTodosReq, ManyTodos);
req_wrapper!(RemovedReq, Removed);
req_wrapper!(NewLabelReq, NewLabel);
req_wrapper!(LabelLookupReq, LabelLookup);
req_wrapper!(LabelUpdateReq, LabelUpdate);
req_wrapper!(OneLabelReq, OneLabel);
req_wrapper!(ManyLabelsReq, ManyLabels);

#[derive(Clone, jigs::Response)]
pub struct HttpResp(pub Result<HttpResponse, String>);
