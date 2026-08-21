use chrono::Utc;
use serde_json::json;
use shifa_admin::models::*;
use shifa_core::id::{TenantId, UserId};
use uuid::Uuid;

#[test]
fn test_audit_event_dto_serialization() {
    let tenant_id = TenantId::from(Uuid::now_v7());
    let user_id = UserId::from(Uuid::now_v7());
    let entity_id = Uuid::now_v7();

    let event = AuditEventDto {
        id: Uuid::now_v7(),
        tenant_id,
        actor_id: Some(user_id),
        actor_type: "PHARMACIST".into(),
        entity_type: "PRESCRIPTION".into(),
        entity_id,
        action: "APPROVE".into(),
        before: Some(json!({"status": "PENDING_REVIEW"})),
        after: Some(json!({"status": "CONFIRMED"})),
        reason: Some("Licensed pharmacist approval verified".into()),
        ip: Some("127.0.0.1".into()),
        occurred_at: Utc::now(),
    };

    let serialized = serde_json::to_string(&event).expect("Serialize AuditEventDto");
    assert!(serialized.contains("PHARMACIST"));
    assert!(serialized.contains("PRESCRIPTION"));
    assert!(serialized.contains("APPROVE"));

    let deserialized: AuditEventDto =
        serde_json::from_str(&serialized).expect("Deserialize AuditEventDto");
    assert_eq!(deserialized.id, event.id);
    assert_eq!(deserialized.action, "APPROVE");
    assert_eq!(deserialized.entity_type, "PRESCRIPTION");
}

#[test]
fn test_system_settings_dto_and_updates() {
    let tenant_id = TenantId::from(Uuid::now_v7());

    let settings = SystemSettingsDto {
        tenant_id,
        name: "Shifa Central Pharmacy".into(),
        legal_name: "Shifa Healthcare PVT LTD".into(),
        ntn: Some("1234567-8".into()),
        strn: Some("327787654321".into()),
        status: "ACTIVE".into(),
        settings: json!({
            "cod_limit_pkr": "5000.0000",
            "auto_assign_riders": true
        }),
        updated_at: Utc::now(),
    };

    let serialized = serde_json::to_string(&settings).expect("Serialize SystemSettingsDto");
    assert!(serialized.contains("1234567-8"));
    assert!(serialized.contains("5000.0000"));

    let req = UpdateSystemSettingsRequest {
        legal_name: Some("Shifa Global Pharmacy LTD".into()),
        ntn: Some("9876543-2".into()),
        strn: None,
        settings: Some(json!({"cod_limit_pkr": "10000.0000"})),
    };

    assert_eq!(req.legal_name.as_deref(), Some("Shifa Global Pharmacy LTD"));
}

#[test]
fn test_operational_report_dto_values() {
    let report = OperationalReportDto {
        today_orders_count: 142,
        rx_queue_depth: 3,
        pending_payments_count: 5,
        active_riders_count: 12,
        total_revenue_pkr: "345000.0000".into(),
        fbr_pending_invoices: 0,
        generated_at: Utc::now(),
    };

    assert_eq!(report.today_orders_count, 142);
    assert_eq!(report.rx_queue_depth, 3);
    assert_eq!(report.total_revenue_pkr, "345000.0000");
}
