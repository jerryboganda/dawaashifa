use crate::error::ChannelError;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

/// Rate limiter respecting channel capability ceilings with backoff retry tracking
#[derive(Debug)]
pub struct ChannelRateLimiter {
    max_rate_per_min: u32,
    timestamps: Mutex<Vec<DateTime<Utc>>>,
    sent_idempotency_keys: Mutex<HashMap<uuid::Uuid, DateTime<Utc>>>,
}

impl ChannelRateLimiter {
    pub fn new(max_rate_per_min: u32) -> Self {
        Self {
            max_rate_per_min,
            timestamps: Mutex::new(Vec::new()),
            sent_idempotency_keys: Mutex::new(HashMap::new()),
        }
    }

    /// Check rate limit and deduplicate idempotency key
    pub fn check_and_acquire(&self, idempotency_key: uuid::Uuid) -> Result<bool, ChannelError> {
        let now = Utc::now();
        let cutoff = now - Duration::minutes(1);

        // Check idempotency: if already sent within 24h, return false (already processed)
        let mut idemp = self.sent_idempotency_keys.lock().unwrap();
        if idemp.contains_key(&idempotency_key) {
            return Ok(false); // Duplicate send prevented
        }

        let mut times = self.timestamps.lock().unwrap();
        times.retain(|&t| t > cutoff);

        if times.len() >= self.max_rate_per_min as usize {
            return Err(ChannelError::RateLimitExceeded);
        }

        times.push(now);
        idemp.insert(idempotency_key, now);
        Ok(true) // Permitted to send
    }
}
