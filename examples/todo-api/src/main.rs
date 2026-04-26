mod features;
mod http;
mod pipeline;
mod store;
mod types;

use crate::http::{read_request, write_response, HttpRequest, HttpResponse};
use crate::pipeline::handle;
use crate::store::Store;
use crate::types::Raw;
use jigs::Request;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

fn into_raw(req: HttpRequest, store: Arc<Store>) -> Raw {
    Raw {
        method: req.method,
        path: req.path,
        headers: req.headers,
        body: req.body,
        store,
    }
}

fn main() -> std::io::Result<()> {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/todo-api");
        std::fs::create_dir_all(&html_dir)?;
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                Some("handle"),
                "todo-api example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{path}#L{line}"),
            ),
        )?;
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(Some("handle"), "todo-api example"),
        )?;
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return Ok(());
    }
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".into());
    let listener = TcpListener::bind(&addr)?;
    let store = Arc::new(Store::new());
    println!("listening on http://{addr}");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };
        let store = Arc::clone(&store);
        thread::spawn(move || {
            let request = match read_request(&mut stream) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("read error: {e}");
                    return;
                }
            };
            let response = handle(Request(into_raw(request, store)));
            let body = match response.inner {
                Ok(r) => r,
                Err(msg) => HttpResponse {
                    status: 500,
                    reason: "Internal Server Error",
                    body: msg,
                },
            };
            if let Err(e) = write_response(&mut stream, &body) {
                eprintln!("write error: {e}");
            }
            let entries = jigs::trace::take();
            print!("{}", jigs::log::render_tree(&entries));
        });
    }
    Ok(())
}
