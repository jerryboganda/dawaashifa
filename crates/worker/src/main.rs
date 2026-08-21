use shifa_worker::schedulers::*;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shifa_worker=info,info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("🚀 Shifa Platform Background Worker Daemon initializing...");

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://shifa:shifa_dev_secret@localhost:5432/shifa".into());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy(&database_url)?;

    info!("📡 Connected to database. Spawning asynchronous schedulers and watchdogs...");

    // Spawn concurrent background workers
    let pool_fbr = pool.clone();
    tokio::spawn(async move {
        run_fbr_retry_scheduler(pool_fbr).await;
    });

    let pool_rx = pool.clone();
    tokio::spawn(async move {
        run_rx_sla_watchdog(pool_rx).await;
    });

    let pool_cold = pool.clone();
    tokio::spawn(async move {
        run_cold_chain_and_expiry_monitor(pool_cold).await;
    });

    let pool_pool = pool.clone();
    tokio::spawn(async move {
        run_number_pool_maintenance(pool_pool).await;
    });

    let pool_part = pool.clone();
    tokio::spawn(async move {
        run_partition_maintenance(pool_part).await;
    });

    info!("✅ All background workers active and running.");

    // Keep main daemon alive until interrupt
    tokio::signal::ctrl_c().await?;
    info!("🛑 Received shutdown signal. Gracefully exiting Shifa Worker Daemon.");

    Ok(())
}
