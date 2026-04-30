mod features;
mod pipeline;
mod types;

use crate::pipeline::handle;
use crate::types::RawEvent;
use jigs::Request;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/events");
        std::fs::create_dir_all(&html_dir).expect("create docs/events");
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                Some("handle"),
                "events bus example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{rel_path}#L{line}"),
            ),
        )
        .expect("write index.html");
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(Some("handle"), "events bus example"),
        )
        .expect("write map.md");
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs();

    let events = vec![
        RawEvent {
            event_type: "order_created".to_string(),
            payload: r#"{ "order_id": 1, "amount": 500, "currency": "USD"}"#.to_string(),
            tenant_id: 42,
            timestamp: now,
            metadata: HashMap::new(),
        },
        RawEvent {
            event_type: "inventory_changed".to_string(),
            payload: r#"{ "sku": "SKU-99", "delta": -1, "warehouse_id": 7 }"#.to_string(),
            tenant_id: 42,
            timestamp: now,
            metadata: HashMap::new(),
        },
        RawEvent {
            event_type: "user_notification".to_string(),
            payload: r#"{ "user_id": 7, "channel": "email", "message": "order shipped" }"#
                .to_string(),
            tenant_id: 42,
            timestamp: now,
            metadata: HashMap::new(),
        },
        RawEvent {
            event_type: "order_updated".to_string(),
            payload: r#"{ "order_id": 1, "status": "paid" }"#.to_string(),
            tenant_id: 42,
            timestamp: now,
            metadata: HashMap::new(),
        },
        RawEvent {
            event_type: "unknown_event".to_string(),
            payload: "{}".to_string(),
            tenant_id: 42,
            timestamp: now,
            metadata: HashMap::new(),
        },
    ];

    for ev in events {
        let _ = handle(Request(ev));
    }

    let entries = jigs::trace::take();
    print!("{}", jigs::log::render_tree(&entries));
}
