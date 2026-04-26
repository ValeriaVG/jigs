use std::collections::HashMap;

#[derive(Clone)]
pub struct CheckoutInput {
    pub token: String,
    pub items: Vec<(String, u32)>,
}

#[derive(Clone)]
pub struct User {
    pub id: u64,
}

pub struct Ctx {
    pub input: CheckoutInput,
    pub user: Option<User>,
    pub stock: HashMap<String, u32>,
    pub prices: HashMap<String, u64>,
    pub reservation: Option<String>,
    pub total_cents: u64,
}

impl Ctx {
    pub fn new(input: CheckoutInput) -> Self {
        Self {
            input,
            user: None,
            stock: HashMap::new(),
            prices: HashMap::new(),
            reservation: None,
            total_cents: 0,
        }
    }
}

pub struct OrderResult {
    pub order_id: u64,
    pub user_id: u64,
    pub reservation: String,
    pub total_cents: u64,
    pub line_count: usize,
}
