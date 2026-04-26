mod features;
mod pipeline;
mod store;
mod types;

use crate::pipeline::handle;
use crate::types::{CheckoutInput, Ctx};
use jigs::Request;
use std::time::Instant;

fn make_request(token: &str, items: Vec<(String, u32)>) -> Request<Ctx> {
    Request(Ctx::new(CheckoutInput {
        token: token.into(),
        items,
    }))
}

async fn run_sample() {
    let req = make_request(
        "u-7-alice@example.com",
        vec![
            ("SKU-RED".into(), 2),
            ("SKU-BLUE".into(), 1),
            ("SKU-GREEN".into(), 3),
        ],
    );
    let resp = handle(req).await;
    match &resp.inner {
        Ok(o) => println!(
            "sample OK: order={} user={} reservation={} total={}c lines={}",
            o.order_id, o.user_id, o.reservation, o.total_cents, o.line_count
        ),
        Err(e) => println!("sample ERR: {e}"),
    }
    let entries = jigs::trace::take();
    print!("{}", jigs::log::render_tree(&entries));
}

async fn run_unauth_sample() {
    let resp = handle(make_request("garbage", vec![("SKU-X".into(), 1)])).await;
    println!(
        "\nunauth sample: {}",
        match resp.inner {
            Ok(_) => "unexpectedly OK".to_string(),
            Err(e) => e,
        }
    );
    let _ = jigs::trace::take();
}

async fn run_bench(label: &str, n: usize, io_micros: u64) {
    store::set_io_micros(io_micros);
    let started = Instant::now();
    let mut ok = 0usize;
    for i in 0..n {
        let req = make_request(
            "u-7-alice@example.com",
            vec![
                (format!("SKU-{i:04}"), 1),
                ("SKU-COMMON".into(), 2),
                ("SKU-OTHER".into(), 1),
            ],
        );
        if handle(req).await.is_ok() {
            ok += 1;
        }
        let _ = jigs::trace::take();
    }
    let elapsed = started.elapsed();
    let rps = (n as f64) / elapsed.as_secs_f64();
    println!(
        "\nbench [{label}, io={io_micros}us]: {n} requests in {:?} ({:.0} req/s, {:.1} us/req avg, ok={ok})",
        elapsed,
        rps,
        elapsed.as_micros() as f64 / n as f64,
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/checkout");
        std::fs::create_dir_all(&html_dir).expect("create docs/checkout");
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                Some("handle"),
                "checkout example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{path}#L{line}"),
            ),
        )
        .expect("write index.html");
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(Some("handle"), "checkout example"),
        )
        .expect("write map.md");
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return;
    }

    run_sample().await;
    run_unauth_sample().await;

    let n: usize = std::env::var("JIGS_BENCH_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000);
    run_bench("realistic", n.min(2000), 50).await;
    run_bench("zero-io", n, 0).await;
}
