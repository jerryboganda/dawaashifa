use shifa_api::build_app;
use shifa_identity::IdentityService;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_shifa_jwt_key_at_least_32_bytes_long".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    let identity_service = IdentityService::new(pool.clone(), jwt_secret);
    let app = build_app(pool, identity_service);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting Shifa API server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
