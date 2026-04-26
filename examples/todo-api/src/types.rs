use crate::store::Store;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Raw {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub store: Arc<Store>,
}

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
