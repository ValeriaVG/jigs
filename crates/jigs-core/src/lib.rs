#![warn(missing_docs)]
//! Core types for the `jigs` framework.
//!
//! A jig is one step in a request-to-response pipeline. Four kinds exist:
//! - `Request` to `Request`            — enrich, validate, transform
//! - `Request` to `Response`           — handler that produces a response
//! - `Response` to `Response`          — post-process the outgoing message
//! - `Request` to `Branch<Req, Resp>`  — guard that may short-circuit
//!
//! Pipelines are built by chaining jigs with `.then(...)`. The type system
//! enforces ordering: once you hold a `Response`, you cannot chain a jig that
//! expects a `Request`. `Branch::Done` and errored request-handling jigs
//! short-circuit the request side of the pipeline, but once a `Response`
//! exists every `Response -> Response` jig runs — including on errored
//! responses — so finalizers (logging, headers, error envelopes) always
//! see the outcome. Jigs that should only act on success must check
//! `Response::is_ok` themselves.

pub mod meta;
pub use meta::{all as all_jigs, find as find_jig, JigMeta};

#[doc(hidden)]
pub use inventory;

/// Inbound message flowing through a pipeline.
pub struct Request<T>(pub T);

/// Outbound message produced by a pipeline. Wraps a `Result` so that
/// downstream jigs can short-circuit on error.
pub struct Response<T> {
    /// The wrapped value, or an error message.
    pub inner: Result<T, String>,
}

impl<T> Response<T> {
    /// Construct a successful response.
    pub fn ok(value: T) -> Self {
        Self { inner: Ok(value) }
    }
    /// Construct an errored response from a message.
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            inner: Err(msg.into()),
        }
    }
    /// Returns `true` if this response carries a value.
    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
    /// Returns `true` if this response carries an error.
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
    }
}

/// Outcome of a guard jig: either continue with a (possibly transformed)
/// request, or short-circuit the pipeline with a response.
pub enum Branch<Req, Resp> {
    /// Continue the pipeline with this request.
    Continue(Request<Req>),
    /// Stop the pipeline and return this response.
    Done(Response<Resp>),
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

impl<T> Step for Request<T> {
    type Out = Request<T>;
    type Fut = core::future::Ready<Request<T>>;
    fn into_step(self) -> Self::Fut {
        core::future::ready(self)
    }
}

impl<T> Step for Response<T> {
    type Out = Response<T>;
    type Fut = core::future::Ready<Response<T>>;
    fn into_step(self) -> Self::Fut {
        core::future::ready(self)
    }
}

impl<R, P> Step for Branch<R, P> {
    type Out = Branch<R, P>;
    type Fut = core::future::Ready<Branch<R, P>>;
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
    fn ok(&self) -> bool;
    /// Error message, if any. Defaults to `None`.
    fn error(&self) -> Option<String> {
        None
    }
}

impl<T> Status for Request<T> {
    fn ok(&self) -> bool {
        true
    }
}

impl<T> Status for Response<T> {
    fn ok(&self) -> bool {
        self.is_ok()
    }
    fn error(&self) -> Option<String> {
        self.inner.as_ref().err().cloned()
    }
}

impl<Req, Resp> Status for Branch<Req, Resp> {
    fn ok(&self) -> bool {
        match self {
            Branch::Continue(_) => true,
            Branch::Done(r) => r.is_ok(),
        }
    }
    fn error(&self) -> Option<String> {
        match self {
            Branch::Continue(_) => None,
            Branch::Done(r) => r.inner.as_ref().err().cloned(),
        }
    }
}

/// Glue trait that lets a `Branch::then(jig)` accept a jig whose output is a
/// `Request`, a `Response`, or another `Branch`, and merge the two outcomes
/// into a single value.
pub trait Merge<Resp> {
    /// Result of merging this value with the prior `Branch`.
    type Merged;
    /// Called when the previous `Branch` was `Continue`.
    fn into_continue(self) -> Self::Merged;
    /// Called when the previous `Branch` was `Done`, propagating its response.
    fn from_done(resp: Response<Resp>) -> Self::Merged;
}

impl<NewReq, Resp> Merge<Resp> for Request<NewReq> {
    type Merged = Branch<NewReq, Resp>;
    fn into_continue(self) -> Self::Merged {
        Branch::Continue(self)
    }
    fn from_done(resp: Response<Resp>) -> Self::Merged {
        Branch::Done(resp)
    }
}

impl<Resp> Merge<Resp> for Response<Resp> {
    type Merged = Response<Resp>;
    fn into_continue(self) -> Self::Merged {
        self
    }
    fn from_done(resp: Response<Resp>) -> Self::Merged {
        resp
    }
}

impl<NewReq, Resp> Merge<Resp> for Branch<NewReq, Resp> {
    type Merged = Branch<NewReq, Resp>;
    fn into_continue(self) -> Self::Merged {
        self
    }
    fn from_done(resp: Response<Resp>) -> Self::Merged {
        Branch::Done(resp)
    }
}

impl<T> Request<T> {
    /// Append the next jig to the pipeline.
    pub fn then<J, U>(self, jig: J) -> U
    where
        J: Jig<Request<T>, Out = U>,
    {
        jig.run(self)
    }
}

impl<T> Response<T> {
    /// Append a `Response -> Response` jig. The jig always runs, including
    /// on errored responses, so finalizers see every outcome. Jigs that
    /// should only transform successful responses must check `is_ok` first.
    pub fn then<J, U>(self, jig: J) -> Response<U>
    where
        J: Jig<Response<T>, Out = Response<U>>,
    {
        jig.run(self)
    }
}

impl<Req, Resp> Branch<Req, Resp> {
    /// Append the next jig to a guarded pipeline. If the previous step was
    /// `Done`, its response is propagated and `jig` is not run.
    pub fn then<J>(self, jig: J) -> <J::Out as Merge<Resp>>::Merged
    where
        J: Jig<Request<Req>>,
        J::Out: Merge<Resp>,
    {
        match self {
            Branch::Continue(r) => <J::Out as Merge<Resp>>::into_continue(jig.run(r)),
            Branch::Done(resp) => <J::Out as Merge<Resp>>::from_done(resp),
        }
    }
}

#[cfg(test)]
mod tests;
