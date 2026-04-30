use std::collections::HashMap;

pub struct RawEvent {
    pub event_type: String,
    pub payload: String,
    pub tenant_id: u64,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

pub struct EventCtx {
    pub event: Event,
    pub tenant_id: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    OrderCreated {
        order_id: u64,
        amount: u64,
        currency: String,
    },
    OrderUpdated {
        order_id: u64,
        status: String,
    },
    InventoryChanged {
        sku: String,
        delta: i64,
        warehouse_id: u64,
    },
    UserNotification {
        user_id: u64,
        channel: String,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct EventResult {
    pub event_id: String,
    pub tenant_id: u64,
    pub outcome: String,
}
