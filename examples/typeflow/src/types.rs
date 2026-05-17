use jigs::{Request, Response};

#[derive(Clone)]
pub struct RawInput {
    pub api_key: String,
    pub value: i64,
}

#[derive(Clone)]
pub struct AuthenticatedData {
    pub user_id: u64,
    pub value: i64,
}

#[derive(Debug, Clone)]
pub struct Computation {
    pub user_id: u64,
    pub value: i64,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct Output {
    pub user_id: u64,
    pub result: String,
}

#[derive(Clone, Request)]
pub struct RawReq(pub RawInput);

#[derive(Clone, Request)]
pub struct AuthReq(pub AuthenticatedData);

#[derive(Clone, Request)]
pub struct ComputeReq(pub Computation);

#[derive(Clone, Response)]
pub struct OutputResp(pub Result<Output, String>);
