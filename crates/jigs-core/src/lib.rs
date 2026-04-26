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
//! expects a `Request`. A `Response` carrying an error short-circuits the
//! remainder of the pipeline; so does a `Branch::Done`.

pub mod trace;

pub struct Request<T>(pub T);

pub struct Response<T> {
    pub inner: Result<T, String>,
}

impl<T> Response<T> {
    pub fn ok(value: T) -> Self {
        Self { inner: Ok(value) }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            inner: Err(msg.into()),
        }
    }
    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
    }
}

pub enum Branch<Req, Resp> {
    Continue(Request<Req>),
    Done(Response<Resp>),
}

pub trait Jig<In> {
    type Out;
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
    type Out;
    type Fut: core::future::Future<Output = Self::Out>;
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

pub trait Status {
    fn ok(&self) -> bool;
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

pub trait Merge<Resp> {
    type Merged;
    fn from_continue(self) -> Self::Merged;
    fn from_done(resp: Response<Resp>) -> Self::Merged;
}

impl<NewReq, Resp> Merge<Resp> for Request<NewReq> {
    type Merged = Branch<NewReq, Resp>;
    fn from_continue(self) -> Self::Merged {
        Branch::Continue(self)
    }
    fn from_done(resp: Response<Resp>) -> Self::Merged {
        Branch::Done(resp)
    }
}

impl<Resp> Merge<Resp> for Response<Resp> {
    type Merged = Response<Resp>;
    fn from_continue(self) -> Self::Merged {
        self
    }
    fn from_done(resp: Response<Resp>) -> Self::Merged {
        resp
    }
}

impl<NewReq, Resp> Merge<Resp> for Branch<NewReq, Resp> {
    type Merged = Branch<NewReq, Resp>;
    fn from_continue(self) -> Self::Merged {
        self
    }
    fn from_done(resp: Response<Resp>) -> Self::Merged {
        Branch::Done(resp)
    }
}

impl<T> Request<T> {
    pub fn then<J, U>(self, jig: J) -> U
    where
        J: Jig<Request<T>, Out = U>,
    {
        jig.run(self)
    }
}

impl<T> Response<T> {
    pub fn then<J, U>(self, jig: J) -> Response<U>
    where
        J: Jig<Response<T>, Out = Response<U>>,
    {
        match self.inner {
            Ok(_) => jig.run(self),
            Err(e) => Response { inner: Err(e) },
        }
    }
}

impl<Req, Resp> Branch<Req, Resp> {
    pub fn then<J>(self, jig: J) -> <J::Out as Merge<Resp>>::Merged
    where
        J: Jig<Request<Req>>,
        J::Out: Merge<Resp>,
    {
        match self {
            Branch::Continue(r) => <J::Out as Merge<Resp>>::from_continue(jig.run(r)),
            Branch::Done(resp) => <J::Out as Merge<Resp>>::from_done(resp),
        }
    }
}

#[cfg(test)]
mod tests;
