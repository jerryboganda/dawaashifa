use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{TenantId, UserId};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditEventDto {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub actor_id: Option<UserId>,
    pub actor_type: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub action: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub ip: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AuditQueryRequest {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub action: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SystemSettingsDto {
    pub tenant_id: TenantId,
    pub name: String,
    pub legal_name: String,
    pub ntn: Option<String>,
    pub strn: Option<String>,
    pub status: String,
    pub settings: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateSystemSettingsRequest {
    pub legal_name: Option<String>,
    pub ntn: Option<String>,
    pub strn: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OperationalReportDto {
    pub today_orders_count: i64,
    pub rx_queue_depth: i64,
    pub pending_payments_count: i64,
    pub active_riders_count: i64,
    pub total_revenue_pkr: String,
    pub fbr_pending_invoices: i64,
    pub generated_at: DateTime<Utc>,
}
