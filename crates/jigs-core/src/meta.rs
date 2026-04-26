//! Compile-time metadata for every `#[jig]` function in the binary.
//!
//! The `#[jig]` macro emits one [`JigMeta`] per annotated function and
//! registers it via the `inventory` crate. Consumers iterate them at runtime
//! through [`all`].

/// Static description of one jig.
#[derive(Debug, Clone, Copy)]
pub struct JigMeta {
    /// Function name as written in source.
    pub name: &'static str,
    /// Source file path (as seen by the compiler at expansion time).
    pub file: &'static str,
    /// 1-based line of the function declaration.
    pub line: u32,
    /// Outer return-type identifier: `"Request"`, `"Response"`, `"Branch"`,
    /// `"Pending"`, or `"Other"`.
    pub kind: &'static str,
    /// Outer first-argument type identifier: `"Request"`, `"Response"`, or
    /// `"Other"`. Combined with [`Self::kind`] this places a jig in one of three
    /// semantic buckets: request-side (Request → Request), switching
    /// (Request → Response/Branch) or response-side (Response → Response).
    pub input: &'static str,
    /// `true` if the underlying function is `async fn`.
    pub is_async: bool,
    /// Names of jigs this function calls via `.then(IDENT)`, in chain order.
    pub chain: &'static [&'static str],
}

inventory::collect!(JigMeta);

/// Iterator over every jig registered in the current binary.
pub fn all() -> impl Iterator<Item = &'static JigMeta> {
    inventory::iter::<JigMeta>()
}

/// Look up a jig by name. `O(N)` over the registry.
pub fn find(name: &str) -> Option<&'static JigMeta> {
    all().find(|m| m.name == name)
}
