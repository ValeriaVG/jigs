//! Core traits and runtime for the `jigs` framework.
//!
//! A `Jig` is anything that turns an input into an output. Jigs compose with
//! `.then(...)`, so a pipeline is itself a jig.

pub trait Jig<In> {
    type Out;

    fn run(&self, input: In) -> Self::Out;

    fn then<J>(self, next: J) -> Then<Self, J>
    where
        Self: Sized,
        J: Jig<Self::Out>,
    {
        Then { first: self, next }
    }
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

pub struct Then<A, B> {
    first: A,
    next: B,
}

impl<In, A, B> Jig<In> for Then<A, B>
where
    A: Jig<In>,
    B: Jig<A::Out>,
{
    type Out = B::Out;

    fn run(&self, input: In) -> Self::Out {
        self.next.run(self.first.run(input))
    }
}
