use crate::types::User;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::sleep;

static IO_MICROS: AtomicU64 = AtomicU64::new(50);

pub fn set_io_micros(us: u64) {
    IO_MICROS.store(us, Ordering::Relaxed);
}

async fn io() {
    let us = IO_MICROS.load(Ordering::Relaxed);
    if us > 0 {
        sleep(Duration::from_micros(us)).await;
    }
}

pub async fn lookup_user(token: &str) -> Option<User> {
    io().await;
    let mut parts = token.splitn(3, '-');
    if parts.next()? != "u" {
        return None;
    }
    let id: u64 = parts.next()?.parse().ok()?;
    parts.next()?;
    Some(User { id })
}

pub async fn lookup_sku(sku: &str) -> Option<(u32, u64)> {
    io().await;
    if sku.is_empty() {
        return None;
    }
    let stock = (sku.bytes().map(|b| b as u32).sum::<u32>() % 20) + 1;
    let price = (sku.len() as u64) * 199;
    Some((stock, price))
}

pub async fn reserve(user_id: u64, items: &[(String, u32)]) -> String {
    io().await;
    let h: u64 = items
        .iter()
        .map(|(s, q)| s.bytes().map(|b| b as u64).sum::<u64>() * (*q as u64))
        .sum();
    format!("rsv-{user_id}-{h}")
}

pub async fn persist_order(user_id: u64, total_cents: u64, reservation: &str) -> u64 {
    io().await;
    let r: u64 = reservation.bytes().map(|b| b as u64).sum();
    user_id
        .wrapping_mul(1315423911)
        .wrapping_add(total_cents)
        .wrapping_add(r)
}
