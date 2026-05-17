use crate::types::AuthReq;
use crate::types::{Computation, ComputeReq, Output, OutputResp};
use jigs::{jig, Branch, Request, Response};

#[jig]
fn prepare(req: AuthReq) -> Branch<ComputeReq, OutputResp> {
    let data = req.0;
    if data.value < 0 {
        return Branch::Done(OutputResp::err("negative value not allowed"));
    }
    Branch::Continue(ComputeReq(Computation {
        user_id: data.user_id,
        value: data.value,
        label: format!("uid-{}", data.user_id),
    }))
}

#[jig]
fn calculate(req: ComputeReq) -> OutputResp {
    let c = req.0;
    let doubled = c.value * 2;
    OutputResp::ok(Output {
        user_id: c.user_id,
        result: format!("{}: {} * 2 = {}", c.label, c.value, doubled),
    })
}

#[jig]
pub fn compute(req: AuthReq) -> OutputResp {
    req.then(prepare).then(calculate)
}
