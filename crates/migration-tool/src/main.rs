use clap::{Parser, Subcommand};
use shifa_core::context::TenantContext;
use shifa_core::id::{TenantId, UserId};
use shifa_migration_tool::adapters::{CsvSourceAdapter, JsonSourceAdapter, SourceAdapter};
use shifa_migration_tool::engine::MigrationEngine;
use shifa_migration_tool::mapping::MappingConfig;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::fs;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "migration-tool")]
#[command(about = "Data migration CLI toolkit for the Shifa platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Probe an unknown source and discover columns/samples (Doc 15 §4, §12)
    Probe {
        #[arg(long)]
        source: String,
        #[arg(long)]
        file_path: Option<String>,
        #[arg(long)]
        table: Option<String>,
    },
    /// Validate a YAML mapping configuration
    Validate {
        #[arg(long)]
        mapping: String,
    },
    /// Run migration in dry-run mode (default) or with explicit --commit (Doc 15 §7)
    Run {
        #[arg(long)]
        mapping: String,
        #[arg(long, default_value_t = false)]
        commit: bool,
    },
    /// Check the status of an import batch
    Status {
        #[arg(long)]
        batch_id: Uuid,
    },
    /// Roll back an import batch by ID (Doc 15 §9)
    Rollback {
        #[arg(long)]
        batch_id: Uuid,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let default_tenant_id = TenantId::new();
    let default_user_id = UserId::new();
    let mut perms = HashSet::new();
    perms.insert("tenant.settings".to_string());
    let ctx = TenantContext::from_authenticated_session(
        default_tenant_id,
        default_user_id,
        vec![],
        perms,
        vec!["SUPER_ADMIN".to_string()],
    );

    match cli.command {
        Commands::Probe {
            source, file_path, ..
        } => {
            println!("🔍 Probing source '{}'...", source);
            if let Some(path) = file_path {
                let content = fs::read_to_string(&path)?;
                let adapter: Box<dyn SourceAdapter> = if source == "json" {
                    Box::new(JsonSourceAdapter::new(content))
                } else {
                    Box::new(CsvSourceAdapter::new(content))
                };
                let schema = adapter.probe().await?;
                println!("Columns discovered ({}):", schema.columns.len());
                for col in &schema.columns {
                    println!("  - {}", col);
                }
                println!("Estimated rows: {}", schema.estimated_count);
            }
        }
        Commands::Validate { mapping } => {
            println!("📋 Validating mapping '{}'...", mapping);
            let content = fs::read_to_string(&mapping)?;
            let map_cfg = MappingConfig::from_yaml_str(&content)?;
            MigrationEngine::validate_mapping(&map_cfg)?;
            println!("✅ Mapping configuration is valid!");
        }
        Commands::Run { mapping, commit } => {
            let dry_run = !commit;
            println!(
                "🚀 Running migration with mapping '{}' (dry_run: {})...",
                mapping, dry_run
            );
            let content = fs::read_to_string(&mapping)?;
            let map_cfg = MappingConfig::from_yaml_str(&content)?;

            let adapter: Box<dyn SourceAdapter> = if let Some(ref path) = map_cfg.source.file_path {
                let file_content = fs::read_to_string(path)?;
                if map_cfg.source.kind == "json" {
                    Box::new(JsonSourceAdapter::new(file_content))
                } else {
                    Box::new(CsvSourceAdapter::new(file_content))
                }
            } else {
                Box::new(CsvSourceAdapter::new(String::new()))
            };

            let report =
                MigrationEngine::run(&ctx, &map_cfg, adapter.as_ref(), dry_run, &pool).await?;
            println!("📊 Migration Report:");
            println!("   Batch ID: {}", report.batch_id);
            println!("   Total Records: {}", report.total_records);
            println!("   Would Insert: {}", report.would_insert);
            println!("   Would Update: {}", report.would_update);
            println!("   Would Skip: {}", report.would_skip);
            println!("   Rejected: {}", report.rejected);
        }
        Commands::Status { batch_id } => {
            println!("ℹ️ Checking batch status for {}", batch_id);
        }
        Commands::Rollback { batch_id } => {
            println!("⏪ Rolling back batch {}", batch_id);
            let count = MigrationEngine::rollback(&ctx, batch_id, &pool).await?;
            println!("✅ Rolled back {} records successfully!", count);
        }
    }

    Ok(())
}
