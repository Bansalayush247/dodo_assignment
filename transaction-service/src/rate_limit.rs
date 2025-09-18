use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};

static RATE_STORE: Lazy<DashMap<String, (Instant, u64)>> = Lazy::new(|| DashMap::new());

/// Returns true if allowed, false if rate limit exceeded.
pub fn allow(api_key_fingerprint: &str, limit_per_min: u64) -> bool {
    let now = Instant::now();
    let window = Duration::from_secs(60);

    let mut entry = RATE_STORE.entry(api_key_fingerprint.to_string()).or_insert((now, 0));
    let start = entry.value().0;
    let count = entry.value().1;

    if now.duration_since(start) > window {
        // reset window
        *entry = (now, 1);
        true
    } else {
        if count < limit_per_min {
            *entry = (start, count + 1);
            true
        } else {
            false
        }
    }
}
