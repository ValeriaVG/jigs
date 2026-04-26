use crate::bindings;
use crate::types::{AgentOutput, Ctx};
use jigs::{jig, Request, Response};

// Simple planner: if context is thin, fall back to a fetch tool call before
// generation. Real agents would loop; here a single optional step is enough
// to show how an agentic step composes inside the pipeline.
#[jig]
async fn maybe_call_tool(req: Request<Ctx>) -> Request<Ctx> {
    if req.0.context.len() >= 2 {
        return req;
    }
    let url = format!("https://search.example/?q={}", req.0.input.query);
    let tool_output = Some(bindings::fetch_tool(&url).await);
    Request(Ctx {
        tool_output,
        ..req.0
    })
}

#[jig]
async fn generate(req: Request<Ctx>) -> Response<AgentOutput> {
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
    Response::ok(out)
}

#[jig]
pub async fn run(req: Request<Ctx>) -> Response<AgentOutput> {
    req.then(maybe_call_tool).then(generate).await
}
