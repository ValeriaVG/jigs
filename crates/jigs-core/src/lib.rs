#![warn(missing_docs)]
//! Core types for the `jigs` framework.
//!
//! A jig is one step in a request-to-response pipeline. Four kinds exist:
//! - Request -> Request            — enrich, validate, transform
//! - Request -> Response           — handler that produces a response
//! - Response -> Response          — post-process the outgoing message
//! - Request -> Branch<Request, Response>  — guard that may short-circuit
//!
//! Pipelines are built by chaining jigs with `.then(...)`. The type system
//! enforces ordering: once you hold a [`Response`] you cannot chain a jig that
//! expects a [`Request`]. `Branch::Done` and errored request-handling jigs
//! short-circuit the request side of the pipeline, but once a [`Response`]
//! exists every `Response -> Response` jig runs — including on errored
//! responses — so finalizers (logging, headers, error envelopes) always
//! see the outcome. Jigs that should only act on success must check
//! [`Response::is_ok`] themselves.

pub mod meta;
pub use meta::{ChainKind, ChainStep, JigDef, JigMeta};

pub mod json;

#[doc(hidden)]
pub trait __Classify {
    const KIND: &'static str;
}

/// An inbound message flowing through a pipeline.
///
/// Types implementing this trait can be chained with `.then(jig)` on the
/// request side.
pub trait Request: Sized + __Classify {
    /// Payload extracted from this request.
    type Payload;
    /// Borrow the payload.
    fn payload(&self) -> &Self::Payload;
    /// Consume the request and return the payload.
    fn into_payload(self) -> Self::Payload;
    /// Wrap a payload into a request.
    fn from_payload(payload: Self::Payload) -> Self;

    /// Append the next jig to the pipeline.
    fn then<J, U>(self, jig: J) -> U
    where
        J: Jig<Self, Out = U>,
    {
        jig.run(self)
    }
}

/// An outbound message produced by a pipeline.
///
/// Types implementing this trait wrap a `Result` so that downstream jigs can
/// short-circuit on error.
pub trait Response: Sized + __Classify {
    /// The payload carried by a successful response.
    type Payload;
    /// Construct a successful response.
    fn ok(payload: Self::Payload) -> Self;
    /// Construct an errored response from a message.
    fn err(msg: impl Into<String>) -> Self;
    /// Returns `true` if this response carries a value.
    fn is_ok(&self) -> bool;
    /// Returns `true` if this response carries an error.
    fn is_err(&self) -> bool {
        !self.is_ok()
    }
    /// Convert into an owned `Result`.
    ///
    /// # Errors
    /// Returns `Err` with the error message when the response carries an error.
    fn into_result(self) -> Result<Self::Payload, String>;
    /// Wrap a `Result` into a response.
    fn from_result(result: Result<Self::Payload, String>) -> Self {
        match result {
            Ok(v) => Self::ok(v),
            Err(e) => Self::err(e),
        }
    }
    /// Non-consuming access to the error message, if this is an error.
    fn error_msg(&self) -> Option<String> {
        if self.is_err() {
            Some("unknown error".into())
        } else {
            None
        }
    }

    /// Append a `Response -> Response` jig. The jig always runs, including
    /// on errored responses, so finalizers see every outcome.
    fn then<J>(self, jig: J) -> J::Out
    where
        J: Jig<Self>,
        J::Out: Response,
    {
        jig.run(self)
    }
}

/// Outcome of a guard jig: either continue with a (possibly transformed)
/// request, or short-circuit the pipeline with a response.
#[derive(Debug)]
pub enum Branch<Req, Resp> {
    /// Continue the pipeline with this request.
    Continue(Req),
    /// Stop the pipeline and return this response.
    Done(Resp),
}

impl<Req: Request, Resp: Response> __Classify for Branch<Req, Resp> {
    const KIND: &'static str = "Branch";
}

impl<Req, Resp> Branch<Req, Resp> {
    /// Returns `true` if this is `Branch::Continue`.
    #[must_use]
    pub fn is_continue(&self) -> bool {
        matches!(self, Branch::Continue(_))
    }

    /// Returns `true` if this is `Branch::Done`.
    #[must_use]
    pub fn is_done(&self) -> bool {
        matches!(self, Branch::Done(_))
    }
}

/// One step in a jigs pipeline. Any `Fn(In) -> Out` automatically implements
/// this trait, so plain functions, closures, and `#[jig]`-annotated functions
/// can all be chained with `.then(...)`.
pub trait Jig<In> {
    /// The value produced by running this jig.
    type Out;
    /// Execute the jig on the given input.
    fn run(&self, input: In) -> Self::Out;
}

impl<In, Out, F> Jig<In> for F
where
    F: Fn(In) -> Out,
{
    type Out = Out;
    fn run(&self, input: In) -> Out {
        (self)(input)
    }
}

/// Wraps a future returned by an async jig so the chain remains spelled with `.then`.
///
/// The `#[jig]` macro converts `async fn` jigs into ordinary functions returning
/// `Pending<impl Future<Output = T>>`. `Pending` itself impls `IntoFuture`, so the
/// final `.await` resolves the whole chain.
pub struct Pending<F>(pub F);

impl<F> __Classify for Pending<F> {
    const KIND: &'static str = "Pending";
}

/// Lifts the output of a jig into a future, so async and sync jigs can be chained
/// uniformly inside a `Pending` chain. Sync values become a `Ready` future, a
/// nested `Pending` is unwrapped to its inner future.
pub trait Step {
    /// Resolved output of this step.
    type Out;
    /// Future yielding the output.
    type Fut: core::future::Future<Output = Self::Out>;
    /// Convert this value into the future the chain awaits.
    fn into_step(self) -> Self::Fut;
}

impl<REQ, RESP> Step for Branch<REQ, RESP>
where
    REQ: Request,
    RESP: Response,
{
    type Out = Branch<REQ, RESP>;
    type Fut = core::future::Ready<Branch<REQ, RESP>>;
    fn into_step(self) -> Self::Fut {
        core::future::ready(self)
    }
}

impl<F> Step for Pending<F>
where
    F: core::future::Future,
{
    type Out = F::Output;
    type Fut = F;
    fn into_step(self) -> Self::Fut {
        self.0
    }
}

impl<F> core::future::IntoFuture for Pending<F>
where
    F: core::future::Future,
{
    type Output = F::Output;
    type IntoFuture = F;
    fn into_future(self) -> F {
        self.0
    }
}

impl<F> Pending<F>
where
    F: core::future::Future + 'static,
{
    /// Append a jig to an in-flight async chain. The next jig may be sync
    /// or async; sync values are lifted via [`Step`].
    pub fn then<J, R>(self, jig: J) -> Pending<impl core::future::Future<Output = R::Out>>
    where
        J: Jig<F::Output, Out = R> + 'static,
        R: Step + 'static,
    {
        Pending(async move {
            let val = self.0.await;
            jig.run(val).into_step().await
        })
    }
}

/// Common interface used by tracing to inspect a jig's outcome without
/// knowing whether the value is a `Request`, `Response`, or `Branch`.
pub trait Status {
    /// Returns `true` if the value represents a successful outcome.
    fn succeeded(&self) -> bool;
    /// Error message, if any. Defaults to `None`.
    fn error(&self) -> Option<String> {
        None
    }
}

impl<REQ, RESP> Status for Branch<REQ, RESP>
where
    REQ: Request,
    RESP: Response,
{
    fn succeeded(&self) -> bool {
        match self {
            Branch::Continue(_) => true,
            Branch::Done(r) => r.is_ok(),
        }
    }
    fn error(&self) -> Option<String> {
        match self {
            Branch::Continue(_) => None,
            Branch::Done(r) => r.error_msg(),
        }
    }
}

/// Glue trait that lets a `Branch::then(jig)` accept a jig whose output is a
/// request, a response, or another `Branch`, and merge the two outcomes
/// into a single value.
///
/// Implement this for custom request/response types when you need to use them
/// after a `Branch`. See the [`impl_request!`] and [`impl_response!`]
/// convenience macros, or derive [`Request`] / [`Response`] which generate this
/// automatically.
pub trait Merge<R> {
    /// Result of merging this value with the prior `Branch`.
    type Merged;
    /// Called when the previous `Branch` was `Continue`.
    fn into_continue(self) -> Self::Merged;
    /// Called when the previous `Branch` was `Done`, propagating its response.
    fn from_done(resp: R) -> Self::Merged;
}

impl<REQ, RESP> Merge<RESP> for Branch<REQ, RESP>
where
    REQ: Request,
    RESP: Response,
{
    type Merged = Branch<REQ, RESP>;
    fn into_continue(self) -> Self::Merged {
        self
    }
    fn from_done(resp: RESP) -> Self::Merged {
        Branch::Done(resp)
    }
}

impl<REQ, RESP> Branch<REQ, RESP>
where
    REQ: Request,
    RESP: Response,
{
    /// Append the next jig to a guarded pipeline. If the previous step was
    /// `Done`, its response is propagated and `jig` is not run.
    #[allow(clippy::needless_pass_by_value)]
    pub fn then<J, Out>(self, jig: J) -> <Out as Merge<RESP>>::Merged
    where
        J: Jig<REQ, Out = Out>,
        Out: Merge<RESP>,
    {
        match self {
            Branch::Continue(r) => Out::into_continue(jig.run(r)),
            Branch::Done(resp) => Out::from_done(resp),
        }
    }
}

/// Wire a custom request type into the framework.
///
/// Generates `Merge<R>`, `Status`, and `Step` so the type works in standard
/// pipelines and async chains.
///
/// ```ignore
/// impl_request!(MyReq);
/// ```
#[macro_export]
macro_rules! impl_request {
    ($t:ty) => {
        impl $crate::__Classify for $t {
            const KIND: &'static str = "Request";
        }
        impl $crate::Step for $t {
            type Out = $t;
            type Fut = ::core::future::Ready<$t>;
            fn into_step(self) -> Self::Fut {
                ::core::future::ready(self)
            }
        }
        impl<R: $crate::Response> $crate::Merge<R> for $t {
            type Merged = $crate::Branch<$t, R>;
            fn into_continue(self) -> Self::Merged {
                $crate::Branch::Continue(self)
            }
            fn from_done(resp: R) -> Self::Merged {
                $crate::Branch::Done(resp)
            }
        }
        impl $crate::Status for $t {
            fn succeeded(&self) -> bool {
                true
            }
            fn error(&self) -> Option<String> {
                None
            }
        }
    };
}

/// Wire a custom response type into the framework.
///
/// Generates `Merge<Self>`, `Status`, and `Step` so the type works in standard
/// pipelines and async chains.
///
/// ```ignore
/// impl_response!(MyResp);
/// ```
#[macro_export]
macro_rules! impl_response {
    ($t:ty) => {
        impl $crate::__Classify for $t {
            const KIND: &'static str = "Response";
        }
        impl $crate::Step for $t {
            type Out = $t;
            type Fut = ::core::future::Ready<$t>;
            fn into_step(self) -> Self::Fut {
                ::core::future::ready(self)
            }
        }
        impl $crate::Merge<$t> for $t {
            type Merged = $t;
            fn into_continue(self) -> Self::Merged {
                self
            }
            fn from_done(resp: $t) -> Self::Merged {
                resp
            }
        }
        impl $crate::Status for $t {
            fn succeeded(&self) -> bool {
                $crate::Response::is_ok(self)
            }
            fn error(&self) -> Option<String> {
                $crate::Response::error_msg(self)
            }
        }
    };
}

/// Multi-arm fork. Predicates are checked in order; the first match
/// consumes the request and its jig is run. If none match, the
/// `_ => default` arm runs. Every arm must produce the same `Out` type;
/// each arm's internal pipeline can have its own intermediate types.
///
/// ```ignore
/// fork!(req,
///     |r| r.path.starts_with("/auth/")  => auth,
///     |r| r.path.starts_with("/todos")  => todos,
///     |r| r.path.starts_with("/labels") => labels,
///     _ => not_found,
/// )
/// ```
#[macro_export]
macro_rules! fork {
    ($req:expr, $($pred:expr => $jig:expr,)+ _ => $default:expr $(,)?) => {{
        let __req = $req;
        $crate::__fork_chain!(__req, $($pred => $jig,)+ ; $default)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __fork_chain {
    ($req:ident, $pred:expr => $jig:expr, $($rest_p:expr => $rest_j:expr,)* ; $default:expr) => {
        if ($pred)($crate::Request::payload(&$req)) {
            ($jig)($req)
        } else {
            $crate::__fork_chain!($req, $($rest_p => $rest_j,)* ; $default)
        }
    };
    ($req:ident, ; $default:expr) => {
        ($default)($req)
    };
}

#[cfg(test)]
mod tests;
