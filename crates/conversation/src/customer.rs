use crate::error::ConversationError;
use shifa_core::id::{CustomerId, TenantId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct CustomerRecord {
    pub id: CustomerId,
    pub msisdn: String,
    pub is_blocked: bool,
    pub default_branch_id: Option<Uuid>,
}

/// Resolve customer by MSISDN, creating on first inbound per Doc 07 §5.
/// Invariant: UNIQUE (tenant_id, msisdn) race-safe via ON CONFLICT DO NOTHING.
pub async fn resolve_or_create_customer(
    pool: &PgPool,
    tenant_id: TenantId,
    msisdn: &str,
    display_name: Option<&str>,
) -> Result<CustomerRecord, ConversationError> {
    let customer_id = CustomerId::new();

    sqlx::query(
        "INSERT INTO customers (id, tenant_id, phone, full_name, preferred_locale, is_blocked, is_verified)
         VALUES ($1, $2, $3, $4, 'UNKNOWN', false, false)
         ON CONFLICT (tenant_id, phone) DO NOTHING"
    )
    .bind(customer_id.0)
    .bind(tenant_id.0)
    .bind(msisdn)
    .bind(display_name.unwrap_or("WhatsApp User"))
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id, phone, is_blocked, default_branch_id
         FROM customers
         WHERE tenant_id = $1 AND phone = $2",
    )
    .bind(tenant_id.0)
    .bind(msisdn)
    .fetch_one(pool)
    .await?;

    Ok(CustomerRecord {
        id: CustomerId::from(row.get::<Uuid, _>("id")),
        msisdn: row.get("phone"),
        is_blocked: row.get("is_blocked"),
        default_branch_id: row.get("default_branch_id"),
    })
}
