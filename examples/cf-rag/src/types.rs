#[derive(Clone)]
pub struct AgentInput {
    pub api_token: String,
    pub query: String,
}

#[derive(Clone)]
pub struct Tenant {
    pub id: u64,
}

#[derive(Clone, Debug)]
pub struct Doc {
    pub id: String,
    pub text: String,
    pub score: f32,
}

pub struct Ctx {
    pub input: AgentInput,
    pub tenant: Option<Tenant>,
    pub embedding: Vec<f32>,
    pub candidates: Vec<Doc>,
    pub context: Vec<Doc>,
    pub tool_output: Option<String>,
}

impl Ctx {
    pub fn new(input: AgentInput) -> Self {
        Self {
            input,
            tenant: None,
            embedding: Vec::new(),
            candidates: Vec::new(),
            context: Vec::new(),
            tool_output: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AgentOutput {
    pub tenant_id: u64,
    pub answer: String,
    pub sources: Vec<String>,
    pub cached: bool,
}
