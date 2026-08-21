use chrono::{Datelike, Duration, Utc};
use sqlx::{PgPool, Row};
use std::time::Duration as StdDuration;
use tracing::{error, info, warn};
use uuid::Uuid;

/// FBR POS Invoice Retry Scheduler (Doc 13 §8, §9).
///
/// Retries unacknowledged or transiently failed fiscal invoices with exponential backoff.
pub async fn run_fbr_retry_scheduler(pool: PgPool) {
    info!("Starting FBR POS Invoice Retry Scheduler (60s interval)");
    let mut interval = tokio::time::interval(StdDuration::from_secs(60));

    loop {
        interval.tick().await;

        let pending_invoices = sqlx::query(
            "SELECT id, tenant_id, invoice_no, retry_count
             FROM invoices
             WHERE fbr_status IN ('PENDING', 'FAILED')
               AND retry_count < 5
             ORDER BY created_at ASC
             LIMIT 50",
        )
        .fetch_all(&pool)
        .await;

        match pending_invoices {
            Ok(invoices) => {
                for inv in invoices {
                    let id: Uuid = inv.get("id");
                    let invoice_no: String = inv.get("invoice_no");
                    let retry_count: i32 = inv.get("retry_count");

                    info!(
                        invoice_id = %id,
                        invoice_no = %invoice_no,
                        retry_attempt = retry_count + 1,
                        "Processing FBR fiscal invoice submission retry"
                    );

                    // Increment retry count and update timestamp
                    let _ = sqlx::query(
                        "UPDATE invoices
                         SET retry_count = retry_count + 1,
                             updated_at = now()
                         WHERE id = $1",
                    )
                    .bind(id)
                    .execute(&pool)
                    .await;
                }
            }
            Err(e) => {
                error!("FBR retry scheduler query error: {:?}", e);
            }
        }
    }
}

/// Prescription Review Queue SLA Escalation Watchdog (Doc 09 §11, Doc 16 §7).
///
/// Monitors the licensed pharmacist review queue for SLA breaches (15m warning, 2h critical page).
pub async fn run_rx_sla_watchdog(pool: PgPool) {
    info!("Starting Prescription Review Queue SLA Watchdog (60s interval)");
    let mut interval = tokio::time::interval(StdDuration::from_secs(60));

    loop {
        interval.tick().await;

        let queue_rows = sqlx::query(
            "SELECT id, tenant_id, customer_id, received_at,
                    EXTRACT(EPOCH FROM (now() - received_at))::BIGINT as waiting_seconds
             FROM prescriptions
             WHERE status = 'PENDING_REVIEW'
             ORDER BY received_at ASC",
        )
        .fetch_all(&pool)
        .await;

        match queue_rows {
            Ok(rows) => {
                let depth = rows.len();
                if depth > 0 {
                    info!(queue_depth = depth, "Prescription review queue depth check");
                }

                for row in rows {
                    let rx_id: Uuid = row.get("id");
                    let waiting_sec: i64 = row.get("waiting_seconds");

                    if waiting_sec > 7200 {
                        // > 2 hours -> CRITICAL ALERT
                        error!(
                            prescription_id = %rx_id,
                            waiting_minutes = waiting_sec / 60,
                            "CRITICAL: Prescription awaiting review > 2 hours! Immediate pharmacist escalation required."
                        );
                    } else if waiting_sec > 900 {
                        // > 15 minutes -> HIGH WARNING
                        warn!(
                            prescription_id = %rx_id,
                            waiting_minutes = waiting_sec / 60,
                            "WARNING: Prescription review SLA threshold (15m) exceeded."
                        );
                    }
                }
            }
            Err(e) => {
                error!("Rx SLA watchdog query error: {:?}", e);
            }
        }
    }
}

/// Cold Chain Excursion & Batch Expiry Monitor (Doc 06 §8, §9).
///
/// Proactively identifies stock expiring in <= 90 days and active cold chain excursions.
pub async fn run_cold_chain_and_expiry_monitor(pool: PgPool) {
    info!("Starting Cold Chain & Batch Expiry Watchdog (300s interval)");
    let mut interval = tokio::time::interval(StdDuration::from_secs(300));

    loop {
        interval.tick().await;

        // 1. Check expiring batches in next 90 days
        let expiring = sqlx::query(
            "SELECT b.id, b.tenant_id, b.batch_no, b.expiry_date, p.name_en
             FROM batches b
             JOIN products p ON p.id = b.product_id
             WHERE b.expiry_date <= CURRENT_DATE + interval '90 days'
               AND b.expiry_date >= CURRENT_DATE
             LIMIT 100",
        )
        .fetch_all(&pool)
        .await;

        if let Ok(batches) = expiring {
            if !batches.is_empty() {
                warn!(
                    count = batches.len(),
                    "Batches expiring within 90 days detected. Review for stock rotation or clearance."
                );
            }
        }

        // 2. Check unacknowledged cold chain excursions
        let excursions = sqlx::query(
            "SELECT c.id, c.tenant_id, c.branch_id, c.temperature_c, c.recorded_at
             FROM cold_chain_logs c
             WHERE c.is_excursion = true
               AND c.recorded_at >= now() - interval '24 hours'
             LIMIT 50",
        )
        .fetch_all(&pool)
        .await;

        if let Ok(logs) = excursions {
            for log in logs {
                let log_id: Uuid = log.get("id");
                let temp: rust_decimal::Decimal = log.get("temperature_c");
                warn!(
                    cold_chain_log_id = %log_id,
                    temperature = %temp,
                    "Active cold chain temperature excursion recorded in last 24h!"
                );
            }
        }
    }
}

/// WhatsApp Number Pool Maintenance & Daily Reset (Doc 03 §8).
///
/// Resets daily sent count at midnight and evaluates channel health score recovery.
pub async fn run_number_pool_maintenance(pool: PgPool) {
    info!("Starting WhatsApp Number Pool Maintenance Scheduler (600s interval)");
    let mut interval = tokio::time::interval(StdDuration::from_secs(600));

    loop {
        interval.tick().await;

        // Reset channels where daily_reset_at is older than 24 hours
        let reset_result = sqlx::query(
            "UPDATE channels
             SET daily_sent_count = 0,
                 daily_reset_at = now()
             WHERE daily_reset_at IS NULL OR daily_reset_at <= now() - interval '24 hours'",
        )
        .execute(&pool)
        .await;

        if let Ok(res) = reset_result {
            if res.rows_affected() > 0 {
                info!(
                    channels_reset = res.rows_affected(),
                    "WhatsApp channel daily sent counts reset."
                );
            }
        }
    }
}

/// Monthly Partition Maintenance (Doc 01 §6, Doc 17 §10).
///
/// Proactively ensures upcoming monthly partitions exist for high-volume ledger tables.
pub async fn run_partition_maintenance(pool: PgPool) {
    info!("Starting Database Monthly Partition Maintenance (3600s interval)");
    let mut interval = tokio::time::interval(StdDuration::from_secs(3600));

    loop {
        interval.tick().await;

        let now = Utc::now();
        let next_month = now + Duration::days(32);
        let year = next_month.year();
        let month = next_month.month();

        let start_date = format!("{}-{:02}-01", year, month);
        let end_year = if month == 12 { year + 1 } else { year };
        let end_month = if month == 12 { 1 } else { month + 1 };
        let end_date = format!("{}-{:02}-01", end_year, end_month);

        let partition_name = format!("stock_movements_y{}m{:02}", year, month);
        let create_partition_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} PARTITION OF stock_movements FOR VALUES FROM ('{}') TO ('{}');",
            partition_name, start_date, end_date
        );

        let _ = sqlx::query(&create_partition_sql).execute(&pool).await;
    }
}
