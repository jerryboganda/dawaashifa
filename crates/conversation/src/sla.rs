use chrono::{DateTime, Timelike, Utc};

/// Check if a given UTC timestamp falls within branch opening hours (default 09:00 - 21:00 Pakistan time UTC+5).
pub fn is_within_opening_hours(timestamp: DateTime<Utc>, open_hour: u32, close_hour: u32) -> bool {
    // Pakistan is UTC+5
    let pkt_hour = (timestamp.time().hour() + 5) % 24;
    pkt_hour >= open_hour && pkt_hour < close_hour
}

/// Calculate SLA breach stage based on elapsed active minutes:
/// - < 15 min: None
/// - 15 - 45 min: Stage 1 (BRANCH_MANAGER)
/// - > 45 min: Stage 2 (OPERATIONS_HEAD)
pub fn evaluate_sla_escalation(elapsed_minutes: i64) -> Option<&'static str> {
    if elapsed_minutes > 45 {
        Some("OPERATIONS_HEAD")
    } else if elapsed_minutes > 15 {
        Some("BRANCH_MANAGER")
    } else {
        None
    }
}
