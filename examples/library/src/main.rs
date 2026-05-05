use jigs::Request;
use jigs_example_library::{all_jigs, handle};

fn main() {
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/library");
        std::fs::create_dir_all(&html_dir).expect("create docs/library");
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                all_jigs(),
                "library example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{rel_path}#L{line}"),
            ),
        )
        .expect("write index.html");
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(all_jigs(), "library example"),
        )
        .expect("write map.md");
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return;
    }
    let name = std::env::args().nth(1).unwrap_or_default();
    let bytes = name.into_bytes();
    let response = handle(Request(bytes));
    match response.inner {
        Ok(v) => println!("{v}"),
        Err(e) => println!("ERROR: {e}"),
    }
}
