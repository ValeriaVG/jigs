mod features;
mod pipeline;
mod types;

use crate::pipeline::handle;
use crate::types::RawInput;
use crate::types::RawReq;

fn main() {
    use crate::pipeline::all_jigs;
    if std::env::var("JIGS_MAP").is_ok() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let html_dir = format!("{dir}/../../docs/typeflow");
        std::fs::create_dir_all(&html_dir).expect("create docs/typeflow");
        std::fs::write(
            format!("{html_dir}/index.html"),
            jigs::map::to_html(
                all_jigs(),
                "typeflow example",
                Some("https://github.com/ValeriaVG/jigs/blob/main/{rel_path}#L{line}"),
            ),
        )
        .expect("write index.html");
        std::fs::write(
            format!("{dir}/map.md"),
            jigs::map::to_markdown(all_jigs(), "typeflow example"),
        )
        .expect("write map.md");
        eprintln!("wrote {html_dir}/index.html and {dir}/map.md");
        return;
    }

    let cases = vec![
        RawInput {
            api_key: "abc".into(),
            value: 21,
        },
        RawInput {
            api_key: String::new(),
            value: 5,
        },
        RawInput {
            api_key: "abc".into(),
            value: -1,
        },
    ];

    for input in cases {
        let _ = handle(RawReq(input));
    }

    let entries = jigs::trace::take();
    print!("{}", jigs::log::render_tree(&entries));
}

#[cfg(test)]
mod tests {
    use crate::pipeline::handle;
    use crate::types::{Output, RawInput, RawReq};
    use jigs::Response;

    #[test]
    fn valid_request_passes_through_all_types() {
        let resp = handle(RawReq(RawInput {
            api_key: "abc".into(),
            value: 21,
        }));
        assert!(resp.is_ok());
        let out: Output = resp.into_result().unwrap();
        assert!(out.result.contains("42"));
        assert!(out.result.contains("uid-"));
    }

    #[test]
    fn missing_api_key_stops_at_auth() {
        let resp = handle(RawReq(RawInput {
            api_key: String::new(),
            value: 5,
        }));
        assert!(resp.is_err());
        assert_eq!(resp.into_result().unwrap_err(), "missing api key");
    }

    #[test]
    fn negative_value_stops_at_prepare() {
        let resp = handle(RawReq(RawInput {
            api_key: "abc".into(),
            value: -1,
        }));
        assert!(resp.is_err());
        assert_eq!(
            resp.into_result().unwrap_err(),
            "negative value not allowed"
        );
    }
}
