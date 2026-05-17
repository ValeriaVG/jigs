use crate::types::AgentResult;
use jigs::{jig, Response};

#[jig]
fn pii_redact(res: AgentResult) -> AgentResult {
    let inner = res.0.map(|mut o| {
        o.answer = redact_emails(&o.answer);
        o
    });
    AgentResult(inner)
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
pub fn finalize(res: AgentResult) -> AgentResult {
    res.then(pii_redact)
}
