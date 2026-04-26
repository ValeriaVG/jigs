use crate::types::AgentOutput;
use jigs::{jig, Response};

// Strip anything that looks like an email from the answer before it leaves.
#[jig]
fn pii_redact(res: Response<AgentOutput>) -> Response<AgentOutput> {
    let Response { inner } = res;
    let inner = inner.map(|mut o| {
        o.answer = redact_emails(&o.answer);
        o
    });
    Response { inner }
}

fn redact_emails(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for word in s.split_inclusive(char::is_whitespace) {
        if word.contains('@') && word.contains('.') {
            out.push_str("[redacted]");
            if let Some(c) = word.chars().last().filter(|c| c.is_whitespace()) {
                out.push(c);
            }
        } else {
            out.push_str(word);
        }
    }
    out
}

#[jig]
pub fn finalize(res: Response<AgentOutput>) -> Response<AgentOutput> {
    res.then(pii_redact)
}
