use std::ffi::c_char;

use jigs::{jig, jigs, Request, Response};

#[derive(Clone, Request)]
pub struct BytesReq(pub Vec<u8>);

#[derive(Clone, Request)]
pub struct TextReq(pub String);

#[derive(Clone, Response)]
pub struct TextResp(pub Result<String, String>);

#[jig]
fn decode(req: BytesReq) -> TextReq {
    TextReq(String::from_utf8_lossy(&req.0).into_owned())
}

#[jig]
fn uppercase(req: TextReq) -> TextReq {
    TextReq(req.0.to_uppercase())
}

#[jig]
pub fn handle(req: BytesReq) -> TextResp {
    req.then(decode)
        .then(uppercase)
        .then(|r: TextReq| TextResp::ok(r.0))
}

jigs!(handle);

#[no_mangle]
pub extern "C" fn jigs_num_collected() -> usize {
    all_jigs().count()
}

/// # Panics
/// Panics if `find_jig("handle")` returns `None`.
#[no_mangle]
pub extern "C" fn jigs_entry_name() -> *const u8 {
    let name = find_jig("handle").unwrap().name;
    name.as_ptr()
}

/// # Panics
/// Panics if `find_jig("handle")` returns `None`.
#[no_mangle]
pub extern "C" fn jigs_entry_name_len() -> usize {
    find_jig("handle").unwrap().name.len()
}

/// # Safety
/// `input` must point to at least `input_len` valid bytes.
///
/// # Panics
/// Panics if `find_jig("handle")` returns `None` or if the result string
/// contains an interior NUL byte.
#[no_mangle]
pub unsafe extern "C" fn jigs_run_pipeline(input: *const u8, input_len: usize) -> *mut c_char {
    let bytes = std::slice::from_raw_parts(input, input_len).to_vec();
    let response = handle(BytesReq(bytes));
    let s = response.0.unwrap_or_else(|e| e);
    let c_str = std::ffi::CString::new(s).unwrap();
    c_str.into_raw()
}

mod c_api {
    #[cfg(test)]
    use super::*;

    #[cfg(test)]
    #[test]
    fn collect_finds_all_jigs_in_lib() {
        let names: Vec<&str> = all_jigs().map(|m| m.name).collect();
        assert!(names.contains(&"handle"), "missing handle, got {:?}", names);
        assert!(names.contains(&"decode"), "missing decode, got {:?}", names);
        assert!(
            names.contains(&"uppercase"),
            "missing uppercase, got {:?}",
            names
        );
    }

    #[cfg(test)]
    #[test]
    fn entry_meta_is_correct() {
        let m = find_jig("handle").unwrap();
        assert_eq!(m.kind, "Other");
        assert_eq!(m.input, "Other");
        assert!(m.chain.iter().any(|s| s.name == "decode"));
        assert!(m.chain.iter().any(|s| s.name == "uppercase"));
    }

    #[cfg(test)]
    #[test]
    fn pipeline_runs_end_to_end() {
        let response = handle(BytesReq(vec![b'h', b'i']));
        assert_eq!(response.0.unwrap(), "HI");
    }
}
