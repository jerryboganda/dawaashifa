use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use shifa_channel::adapter::ChannelAdapter;
use shifa_channel::cloud_api::{CloudApiAdapter, CloudApiConfig};
use shifa_channel::error::ChannelError;
use shifa_channel::rate_limit::ChannelRateLimiter;
use shifa_channel::templates::TemplateRegistry;
use shifa_channel::types::*;
use shifa_channel::webhook::{parse_inbound_webhook, verify_hub_signature};
use shifa_core::id::{ChannelId, ConversationId, TenantId};
use std::sync::Arc;
use uuid::Uuid;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

type HmacSha256 = Hmac<Sha256>;

fn generate_signature(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let hex_digest = hex::encode(mac.finalize().into_bytes());
    format!("sha256={}", hex_digest)
}

#[test]
fn test_webhook_signature_verification() {
    let secret = "test_meta_app_secret";
    let body = b"{\"object\":\"whatsapp_business_account\"}";

    let valid_sig = generate_signature(secret, body);
    assert!(verify_hub_signature(body, &valid_sig, secret).is_ok());

    // 1. Acceptance test: webhook_rejects_bad_signature (403, invalid signature)
    let bad_sig = "sha256=0000000000000000000000000000000000000000000000000000000000000000";
    assert!(verify_hub_signature(body, bad_sig, secret).is_err());
}

#[test]
fn test_choice_rendering_three_tiers() {
    // 2. Acceptance test: choice_three_options_renders_buttons (<= 3 options)
    let three_options = vec![
        ChoiceOption { id: "1".into(), title: "Panadol".into(), description: None },
        ChoiceOption { id: "2".into(), title: "Disprin".into(), description: None },
        ChoiceOption { id: "3".into(), title: "Brufen".into(), description: None },
    ];
    let res3 = CloudApiAdapter::render_choice("Select medicine:", &three_options);
    assert_eq!(res3["interactive"]["type"], "button");
    assert_eq!(res3["interactive"]["action"]["buttons"].as_array().unwrap().len(), 3);

    // 3. Acceptance test: choice_eight_options_renders_list (4..=10 options)
    let eight_options: Vec<_> = (1..=8)
        .map(|i| ChoiceOption { id: i.to_string(), title: format!("Option {}", i), description: Some(format!("Desc {}", i)) })
        .collect();
    let res8 = CloudApiAdapter::render_choice("Select option:", &eight_options);
    assert_eq!(res8["interactive"]["type"], "list");
    assert_eq!(res8["interactive"]["action"]["sections"][0]["rows"].as_array().unwrap().len(), 8);

    // 4. Acceptance test: choice_fifteen_options_renders_numbered_text (> 10 options)
    let fifteen_options: Vec<_> = (1..=15)
        .map(|i| ChoiceOption { id: i.to_string(), title: format!("Item {}", i), description: None })
        .collect();
    let res15 = CloudApiAdapter::render_choice("Browse items:", &fifteen_options);
    assert_eq!(res15["type"], "text");
    let text = res15["text"]["body"].as_str().unwrap();
    assert!(text.contains("1. Item 1"));
    assert!(text.contains("15. Item 15"));
    assert!(text.contains("Reply with the option number (1-15)"));
}

#[test]
fn test_unknown_message_type_is_stored_as_unsupported() {
    // 5. Acceptance test: unknown_message_type_is_stored_not_dropped
    let payload = json!({
        "object": "whatsapp_business_account",
        "entry": [{
            "id": "WHATSAPP_BUSINESS_ACCOUNT_ID",
            "changes": [{
                "value": {
                    "messaging_product": "whatsapp",
                    "metadata": { "display_phone_number": "12345", "phone_number_id": "123" },
                    "messages": [{
                        "from": "923001234567",
                        "id": "wamid.unknown123",
                        "timestamp": "1700000000",
                        "type": "ephemeral_sticker_packet",
                        "ephemeral_sticker_packet": {}
                    }]
                },
                "field": "messages"
            }]
        }]
    });

    let tenant_id = TenantId::new();
    let channel_id = ChannelId::new();
    let messages = parse_inbound_webhook(&payload, tenant_id, channel_id);

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].from, "+923001234567");
    assert_eq!(messages[0].transport_message_id, "wamid.unknown123");
    match &messages[0].content {
        InboundContent::Unsupported { raw_type } => {
            assert_eq!(raw_type, "ephemeral_sticker_packet");
        }
        _ => panic!("Expected InboundContent::Unsupported"),
    }
}

#[tokio::test]
async fn test_freeform_outside_window_fails_loudly() {
    // 6. Acceptance test: freeform_outside_window_returns_error
    let server = MockServer::start().await;
    let config = CloudApiConfig {
        base_url: server.uri(),
        api_version: "v21.0".into(),
        phone_number_id: "phone123".into(),
        access_token: "token123".into(),
    };
    let registry = Arc::new(TemplateRegistry::new());
    let adapter = CloudApiAdapter::new(ChannelId::new(), config, registry);

    let msg = OutboundMessage {
        tenant_id: TenantId::new(),
        conversation_id: ConversationId::new(),
        to: "+923001234567".into(),
        body: OutboundBody::Text { body: "Hello customer".into() },
        idempotency_key: Uuid::now_v7(),
        locale: "en".into(),
    };

    // Freeform when window is closed (false) must return Err(WindowClosed)
    let result = adapter.send(msg, false).await;
    assert!(matches!(result, Err(ChannelError::WindowClosed)));
}

#[tokio::test]
async fn test_unapproved_template_fails_before_network_call() {
    // 7. Acceptance test: unapproved_template_fails_before_network_call
    let server = MockServer::start().await;
    let config = CloudApiConfig {
        base_url: server.uri(),
        api_version: "v21.0".into(),
        phone_number_id: "phone123".into(),
        access_token: "token123".into(),
    };
    let registry = Arc::new(TemplateRegistry::new());
    registry.set_status("pending_promo_template".into(), "PENDING_APPROVAL".into());
    let adapter = CloudApiAdapter::new(ChannelId::new(), config, registry);

    let msg = OutboundMessage {
        tenant_id: TenantId::new(),
        conversation_id: ConversationId::new(),
        to: "+923001234567".into(),
        body: OutboundBody::Template {
            name: "pending_promo_template".into(),
            language: "en".into(),
            params: vec![],
        },
        idempotency_key: Uuid::now_v7(),
        locale: "en".into(),
    };

    let result = adapter.send(msg, false).await;
    assert!(matches!(result, Err(ChannelError::TemplateNotApproved(_, _))));
}

#[tokio::test]
async fn test_cloud_api_send_success_and_error_handling() {
    let server = MockServer::start().await;

    // 8. Acceptance test: Send template outside window succeeds with 200 OK from Meta
    Mock::given(method("POST"))
        .and(path("/v21.0/phone123/messages"))
        .and(header("Authorization", "Bearer token123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "messaging_product": "whatsapp",
            "contacts": [{ "input": "923001234567", "wa_id": "923001234567" }],
            "messages": [{ "id": "wamid.HBgNNjI4OTk=" }]
        })))
        .mount(&server)
        .await;

    let config = CloudApiConfig {
        base_url: server.uri(),
        api_version: "v21.0".into(),
        phone_number_id: "phone123".into(),
        access_token: "token123".into(),
    };
    let registry = Arc::new(TemplateRegistry::new());
    let adapter = CloudApiAdapter::new(ChannelId::new(), config, registry);

    let msg = OutboundMessage {
        tenant_id: TenantId::new(),
        conversation_id: ConversationId::new(),
        to: "+923001234567".into(),
        body: OutboundBody::Template {
            name: "order_confirmed".into(),
            language: "en".into(),
            params: vec![
                TemplateParam { name: "order_no".into(), value: "ORD-1234".into() },
                TemplateParam { name: "total".into(), value: "Rs 1,500.00".into() },
            ],
        },
        idempotency_key: Uuid::now_v7(),
        locale: "en".into(),
    };

    let receipt = adapter.send(msg, false).await.expect("send template");
    assert_eq!(receipt.transport_message_id, "wamid.HBgNNjI4OTk=");
    assert_eq!(receipt.status, "ACCEPTED");
}

#[test]
fn test_rate_limiter_and_idempotency_prevention() {
    // 9. Acceptance test: rate_limiter_respects_capability_ceiling and idempotency_key_prevents_duplicate_send
    let limiter = ChannelRateLimiter::new(3);
    let key1 = Uuid::now_v7();
    let key2 = Uuid::now_v7();
    let key3 = Uuid::now_v7();
    let key4 = Uuid::now_v7();

    assert!(limiter.check_and_acquire(key1).unwrap());
    // Duplicate send with key1 returns false (prevents duplicate transmission)
    assert!(!limiter.check_and_acquire(key1).unwrap());

    assert!(limiter.check_and_acquire(key2).unwrap());
    assert!(limiter.check_and_acquire(key3).unwrap());

    // 4th unique key exceeds rate limit of 3 per minute
    assert!(limiter.check_and_acquire(key4).is_err());
}
