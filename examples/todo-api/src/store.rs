use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

#[derive(Clone)]
#[allow(dead_code)]
pub struct User {
    pub id: u64,
    pub email: String,
    pub password: String,
}

#[derive(Clone, Serialize)]
pub struct Todo {
    pub id: u64,
    pub user_id: u64,
    pub title: String,
    pub done: bool,
    pub labels: Vec<u64>,
}

#[derive(Clone, Serialize)]
pub struct Label {
    pub id: u64,
    pub user_id: u64,
    pub name: String,
}

#[derive(Default)]
pub struct Store {
    next_id: AtomicU64,
    users: RwLock<HashMap<u64, User>>,
    users_by_email: RwLock<HashMap<String, u64>>,
    sessions: RwLock<HashMap<String, u64>>,
    todos: RwLock<HashMap<u64, Todo>>,
    labels: RwLock<HashMap<u64, Label>>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    fn id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn mint_token(&self, user_id: u64) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let token = format!("tok_{user_id}_{nanos}");
        self.sessions
            .write()
            .unwrap()
            .insert(token.clone(), user_id);
        token
    }

    pub fn signup(&self, email: &str, password: &str) -> Result<(u64, String), &'static str> {
        let mut by_email = self.users_by_email.write().unwrap();
        if by_email.contains_key(email) {
            return Err("email already registered");
        }
        let id = self.id();
        self.users.write().unwrap().insert(
            id,
            User {
                id,
                email: email.into(),
                password: password.into(),
            },
        );
        by_email.insert(email.into(), id);
        Ok((id, self.mint_token(id)))
    }

    pub fn login(&self, email: &str, password: &str) -> Option<(u64, String)> {
        let id = *self.users_by_email.read().unwrap().get(email)?;
        let users = self.users.read().unwrap();
        let u = users.get(&id)?;
        if u.password != password {
            return None;
        }
        Some((id, self.mint_token(id)))
    }

    pub fn user_for_token(&self, token: &str) -> Option<u64> {
        self.sessions.read().unwrap().get(token).copied()
    }

    pub fn create_todo(&self, user_id: u64, title: String) -> Todo {
        let id = self.id();
        let todo = Todo {
            id,
            user_id,
            title,
            done: false,
            labels: vec![],
        };
        self.todos.write().unwrap().insert(id, todo.clone());
        todo
    }
    pub fn list_todos(&self, user_id: u64) -> Vec<Todo> {
        let mut v: Vec<Todo> = self
            .todos
            .read()
            .unwrap()
            .values()
            .filter(|t| t.user_id == user_id)
            .cloned()
            .collect();
        v.sort_by_key(|t| t.id);
        v
    }
    pub fn get_todo(&self, user_id: u64, id: u64) -> Option<Todo> {
        self.todos
            .read()
            .unwrap()
            .get(&id)
            .filter(|t| t.user_id == user_id)
            .cloned()
    }
    pub fn update_todo(
        &self,
        user_id: u64,
        id: u64,
        title: Option<String>,
        done: Option<bool>,
    ) -> Option<Todo> {
        let mut todos = self.todos.write().unwrap();
        let t = todos.get_mut(&id).filter(|t| t.user_id == user_id)?;
        if let Some(v) = title {
            t.title = v;
        }
        if let Some(v) = done {
            t.done = v;
        }
        Some(t.clone())
    }
    pub fn delete_todo(&self, user_id: u64, id: u64) -> bool {
        let mut todos = self.todos.write().unwrap();
        if todos.get(&id).is_some_and(|t| t.user_id == user_id) {
            todos.remove(&id);
            true
        } else {
            false
        }
    }

    pub fn create_label(&self, user_id: u64, name: String) -> Label {
        let id = self.id();
        let l = Label { id, user_id, name };
        self.labels.write().unwrap().insert(id, l.clone());
        l
    }
    pub fn list_labels(&self, user_id: u64) -> Vec<Label> {
        let mut v: Vec<Label> = self
            .labels
            .read()
            .unwrap()
            .values()
            .filter(|l| l.user_id == user_id)
            .cloned()
            .collect();
        v.sort_by_key(|l| l.id);
        v
    }
    pub fn update_label(&self, user_id: u64, id: u64, name: String) -> Option<Label> {
        let mut labels = self.labels.write().unwrap();
        let l = labels.get_mut(&id).filter(|l| l.user_id == user_id)?;
        l.name = name;
        Some(l.clone())
    }
    pub fn delete_label(&self, user_id: u64, id: u64) -> bool {
        let mut labels = self.labels.write().unwrap();
        if labels.get(&id).is_none_or(|l| l.user_id != user_id) {
            return false;
        }
        labels.remove(&id);
        for t in self.todos.write().unwrap().values_mut() {
            t.labels.retain(|x| *x != id);
        }
        true
    }
    pub fn attach_label(&self, user_id: u64, todo_id: u64, label_id: u64) -> Option<Todo> {
        if self
            .labels
            .read()
            .unwrap()
            .get(&label_id)
            .is_none_or(|l| l.user_id != user_id)
        {
            return None;
        }
        let mut todos = self.todos.write().unwrap();
        let t = todos.get_mut(&todo_id).filter(|t| t.user_id == user_id)?;
        if !t.labels.contains(&label_id) {
            t.labels.push(label_id);
        }
        Some(t.clone())
    }
    pub fn detach_label(&self, user_id: u64, todo_id: u64, label_id: u64) -> Option<Todo> {
        let mut todos = self.todos.write().unwrap();
        let t = todos.get_mut(&todo_id).filter(|t| t.user_id == user_id)?;
        t.labels.retain(|x| *x != label_id);
        Some(t.clone())
    }
}
