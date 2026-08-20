use std::time::Duration;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::error::ChannelError;
use crate::types::ChannelPoolStatus;

pub struct HumanPacer {
    last_sent_at: Mutex<Option<Instant>>,
}

impl Default for HumanPacer {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanPacer {
    pub fn new() -> Self {
        Self {
            last_sent_at: Mutex::new(None),
        }
    }

    /// Calculates composing presence duration based on body length (1-7s) (Doc 03 §7)
    pub fn calculate_composing_duration(body_length: usize) -> Duration {
        let secs = (body_length as f64 / 12.0).clamp(1.0, 7.0);
        Duration::from_secs_f64(secs)
    }

    /// Checks daily limits based on pool status (300/day for ACTIVE, 40/day for WARMING) (Doc 03 §7)
    pub fn check_daily_limits(
        status: ChannelPoolStatus,
        sent_today: u32,
    ) -> Result<(), ChannelError> {
        let limit = match status {
            ChannelPoolStatus::Active => 300,
            ChannelPoolStatus::Warming => 40,
            ChannelPoolStatus::Degraded => 150,
            _ => 0,
        };

        if sent_today >= limit {
            return Err(ChannelError::DailyLimitExceeded {
                current: sent_today,
                limit,
            });
        }

        Ok(())
    }

    /// Enforces minimum 2-second spacing between messages to avoid machine-speed burst bans
    pub async fn enforce_minimum_gap(&self, min_gap_secs: u64) {
        let mut last = self.last_sent_at.lock().await;
        if let Some(prev) = *last {
            let elapsed = prev.elapsed();
            let min_duration = Duration::from_secs(min_gap_secs);
            if elapsed < min_duration {
                let wait = min_duration - elapsed;
                tokio::time::sleep(wait).await;
            }
        }
        *last = Some(Instant::now());
    }
}
