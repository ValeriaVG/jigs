use std::ffi::c_char;

use jigs::{jig, jigs, Request, Response};

#[jig]
fn decode(req: Request<Vec<u8>>) -> Request<String> {
    Response::ok(String::from_utf8_lossy(&req.0).into())
        .inner
        .map(Request)
        .unwrap_or(Request(String::new()))
}

#[jig]
fn uppercase(req: Request<String>) -> Request<String> {
    Request(req.0.to_uppercase())
}

#[jig]
pub fn handle(req: Request<Vec<u8>>) -> Response<String> {
    req.then(decode)
        .then(uppercase)
        .then(|r: Request<String>| Response::ok(r.0))
}

jigs!(handle);

#[no_mangle]
pub extern "C" fn jigs_num_collected() -> usize {
    all_jigs().count()
}

#[no_mangle]
pub extern "C" fn jigs_entry_name() -> *const u8 {
    let name = find_jig("handle").unwrap().name;
    name.as_ptr()
}

#[no_mangle]
pub extern "C" fn jigs_entry_name_len() -> usize {
    find_jig("handle").unwrap().name.len()
}

/// # Safety
/// `input` must point to at least `input_len` valid bytes.
#[no_mangle]
pub unsafe extern "C" fn jigs_run_pipeline(input: *const u8, input_len: usize) -> *mut c_char {
    let bytes = unsafe { std::slice::from_raw_parts(input, input_len) }.to_vec();
    let response = handle(Request(bytes));
    let s = response.inner.unwrap_or_else(|e| e);
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
        assert_eq!(m.kind, "Response");
        assert_eq!(m.input, "Request");
        assert!(m.chain.iter().any(|s| s.name == "decode"));
        assert!(m.chain.iter().any(|s| s.name == "uppercase"));
    }

    #[cfg(test)]
    #[test]
    fn pipeline_runs_end_to_end() {
        let response = handle(Request(vec![b'h', b'i']));
        assert_eq!(response.inner.unwrap(), "HI");
    }
}
