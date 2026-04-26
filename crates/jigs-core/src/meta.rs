//! Compile-time metadata for every `#[jig]` function in the binary.
//!
//! The `#[jig]` macro emits one [`JigMeta`] per annotated function and
//! registers it via the `inventory` crate. Consumers iterate them at runtime
//! through [`all`].

/// How a chain entry was reached from the surrounding jig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainKind {
    /// Reached via `.then(...)` — sequential composition. Output of the
    /// previous step flows into this one.
    Then,
    /// Reached via `fork!(...)` — alternative arm. Sibling arms do not
    /// flow into each other; exactly one arm runs per request.
    Fork,
}

/// One entry in a jig's chain: the called jig's name plus how it was
/// composed with the surrounding pipeline.
#[derive(Debug, Clone, Copy)]
pub struct ChainStep {
    /// Name of the jig referenced at this step.
    pub name: &'static str,
    /// Compositional relationship to the surrounding jig.
    pub kind: ChainKind,
}

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
    /// Jigs this function references, in source order, tagged with how
    /// they were composed (`.then(...)` vs `fork!(...)` arm).
    pub chain: &'static [ChainStep],
}

impl JigMeta {
    /// Iterator over chain step names, ignoring kind. Convenience for
    /// callers that only care about the referenced jig names.
    pub fn chain_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.chain.iter().map(|s| s.name)
    }
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
