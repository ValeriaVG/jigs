// Mocked Cloudflare bindings: Workers AI (embeddings + chat), Vectorize, KV.
// In a real Worker these would be `env.AI.run(...)`, `env.VECTORIZE.query(...)`,
// `env.KV.get/put(...)`. The shape of the calls matches what jigs would compose.

use crate::types::{AgentOutput, Doc, Tenant};
use std::hash::Hasher;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

static IO_MICROS: AtomicU64 = AtomicU64::new(0);

pub fn set_io_micros(v: u64) {
    IO_MICROS.store(v, Ordering::Relaxed);
}

async fn io() {
    let us = IO_MICROS.load(Ordering::Relaxed);
    if us > 0 {
        tokio::time::sleep(Duration::from_micros(us)).await;
    }
}

pub async fn lookup_tenant(token: &str) -> Option<Tenant> {
    io().await;
    token
        .strip_prefix("cf-")
        .and_then(|rest| rest.split_once('-'))
        .and_then(|(id, _name)| {
            Some(Tenant {
                id: id.parse().ok()?,
            })
        })
}

// Pretend to call Workers AI `@cf/baai/bge-base-en-v1.5`.
pub async fn embed(text: &str) -> Vec<f32> {
    io().await;
    let mut v = [0f32; 8];
    for (i, b) in text.bytes().enumerate() {
        v[i % v.len()] += (b as f32) / 255.0;
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    v.iter().map(|x| x / norm).collect()
}

// Pretend to call Vectorize `index.query(vector, topK=k)`.
pub async fn vector_query(v: &[f32], k: usize, tenant: u64) -> Vec<Doc> {
    io().await;
    // Mock: weak query signal -> sparse retrieval (lets the agent fall back
    // to a tool call). A real Vectorize query would return scored matches.
    let active = v.iter().filter(|x| x.abs() > 0.01).count();
    if active < 3 {
        return Vec::new();
    }
    let corpus = [
        (
            "doc-cf-1",
            "Cloudflare Workers run on the edge in V8 isolates.",
        ),
        (
            "doc-cf-2",
            "Vectorize is a globally distributed vector database.",
        ),
        (
            "doc-cf-3",
            "Workers AI exposes models like Llama and BGE embeddings.",
        ),
        (
            "doc-cf-4",
            "KV is an eventually-consistent key-value store.",
        ),
        (
            "doc-cf-5",
            "Durable Objects provide single-instance coordination.",
        ),
        ("doc-spam", "Buy followers fast cheap link in bio."),
    ];
    corpus
        .iter()
        .enumerate()
        .map(|(i, (id, text))| Doc {
            id: format!("t{tenant}/{id}"),
            text: (*text).into(),
            score: 0.95 - (i as f32) * 0.07,
        })
        .take(k)
        .collect()
}

// Pretend to call Workers AI `@cf/meta/llama-3.1-8b-instruct`.
pub async fn chat(prompt: &str, ctx: &[Doc]) -> String {
    io().await;
    let snippet = ctx
        .iter()
        .map(|d| format!("- {} ({})", d.text, d.id))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "[from {n} sources]\n{snippet}\n=> answer for: {q}",
        n = ctx.len(),
        q = prompt.chars().take(64).collect::<String>()
    )
}

// Pretend to call `fetch()` for an agentic tool step.
pub async fn fetch_tool(url: &str) -> String {
    io().await;
    format!("[fetched {url}: 200 OK, 1.2kb]")
}

// Mocked Cloudflare Cache API: `caches.default.match(req)` / `.put(req, res)`.
// Real Workers use a synthetic Request URL as the key. Here we hash
// (tenant_id, query) to a u64 and use it as the cache URL path so neither
// the tenant id nor the raw query appear in the key material.
static CACHE: Mutex<Vec<(String, AgentOutput)>> = Mutex::new(Vec::new());

pub async fn cache_match(url: &str) -> Option<AgentOutput> {
    io().await;
    CACHE
        .lock()
        .ok()?
        .iter()
        .find(|(k, _)| k == url)
        .map(|(_, v)| v.clone())
}

pub async fn cache_put(url: &str, value: AgentOutput) {
    io().await;
    if let Ok(mut g) = CACHE.lock() {
        g.retain(|(k, _)| k != url);
        g.push((url.into(), value));
    }
}

pub fn cache_url(tenant: u64, query: &str) -> String {
    // DefaultHasher is deterministic within a Rust version, fine for a demo.
    // Production should use a fixed-key SipHash or a real digest like blake3
    // so the key stays stable across toolchain upgrades.
    let mut h = std::collections::hash_map::DefaultHasher::new();
    h.write_u64(tenant);
    h.write_u8(0);
    h.write(query.as_bytes());
    format!("https://cache.internal/rag/{:016x}", h.finish())
}
