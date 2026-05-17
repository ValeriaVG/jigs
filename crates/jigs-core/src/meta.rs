//! Compile-time metadata for every `#[jig]` function in the binary.
//!
//! The `#[jig]` macro emits one zero-sized marker struct per annotated
//! function and implements [`JigDef`] on it. The `jigs!` macro on the
//! entry point recursively collects all reachable jig metadata through
//! the trait system, with no link-time registration.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainStep {
    /// Name of the jig referenced at this step.
    pub name: &'static str,
    /// Compositional relationship to the surrounding jig.
    pub kind: ChainKind,
}

/// Static description of one jig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Exact payload type coming in (e.g. `"Raw"`, `"u32"`, `"Response<HttpResponse>"`).
    pub input_type: &'static str,
    /// Exact payload type going out (e.g. `"HttpResponse"`, `"String"`, `"Branch<Ctx,OrderResult>"`).
    pub output_type: &'static str,
    /// `true` if the underlying function is `async fn`.
    pub is_async: bool,
    /// Rust module path of the function (e.g. `crate::features::orders`).
    pub module: &'static str,
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

/// Trait implemented by the zero-sized marker struct that the `#[jig]`
/// macro emits alongside each jig function. The marker struct is named
/// `__Jig_<fn_name>` to avoid namespace collisions with the function
/// itself. The `jigs!` macro calls `<Entry as JigDef>::collect` to
/// recursively gather metadata for every reachable jig, with no
/// link-time registration.
pub trait JigDef {
    /// Static metadata for this jig.
    const META: JigMeta;

    /// Append this jig's metadata to `out` and recursively collect every
    /// jig reachable through [`Self::META`]'s chain, deduplicating by name.
    fn collect(out: &mut Vec<&'static JigMeta>);
}
