use crate::bindings;
use crate::types::{AgentOutput, AgentResult, Ctx, CtxReq};
use jigs::{jig, Request, Response};

#[jig]
async fn maybe_call_tool(req: CtxReq) -> CtxReq {
    if req.0.context.len() >= 2 {
        return req;
    }
    let url = format!("https://search.example/?q={}", req.0.input.query);
    let tool_output = Some(bindings::fetch_tool(&url).await);
    CtxReq(Ctx {
        tool_output,
        ..req.0
    })
}

#[jig]
async fn generate(req: CtxReq) -> AgentResult {
    let tenant = req.0.tenant.expect("guard");
    let mut prompt = req.0.input.query.clone();
    if let Some(t) = req.0.tool_output.as_ref() {
        prompt.push_str("\n\nTool result:\n");
        prompt.push_str(t);
    }
    let answer = bindings::chat(&prompt, &req.0.context).await;
    let sources = req.0.context.iter().map(|d| d.id.clone()).collect();
    let out = AgentOutput {
        tenant_id: tenant.id,
        answer,
        sources,
        cached: false,
    };
    bindings::cache_put(
        &bindings::cache_url(tenant.id, &req.0.input.query),
        out.clone(),
    )
    .await;
    AgentResult::ok(out)
}

#[jig]
pub async fn run(req: CtxReq) -> AgentResult {
    req.then(maybe_call_tool).then(generate).await
}
