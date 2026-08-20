use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shifa_core::money::Money;

#[derive(Debug, Clone)]
pub struct ExtractedPaymentDetails {
    pub tid: Option<String>,
    pub amount: Option<Money>,
    pub timestamp: Option<DateTime<Utc>>,
    pub sender: Option<String>,
    pub bank: Option<String>,
    pub confidence: f32,
    pub is_known_bank_layout: bool,
}

#[async_trait]
pub trait PaymentOcrProvider: Send + Sync {
    async fn extract(&self, image_object_key: &str) -> Result<ExtractedPaymentDetails, String>;
}

#[derive(Debug, Clone)]
pub struct MockPaymentOcrProvider;

#[async_trait]
impl PaymentOcrProvider for MockPaymentOcrProvider {
    async fn extract(&self, image_object_key: &str) -> Result<ExtractedPaymentDetails, String> {
        // Deterministic extraction based on test image object key heuristics
        if image_object_key.contains("invalid_layout") {
            return Ok(ExtractedPaymentDetails {
                tid: Some("TID_UNKNOWN_LAYOUT".into()),
                amount: Some(Money::from_major(1500)),
                timestamp: Some(Utc::now()),
                sender: Some("03001234567".into()),
                bank: None,
                confidence: 0.85,
                is_known_bank_layout: false,
            });
        }

        if image_object_key.contains("low_confidence") {
            return Ok(ExtractedPaymentDetails {
                tid: Some("TID_BLURRY_999".into()),
                amount: Some(Money::from_major(1500)),
                timestamp: Some(Utc::now()),
                sender: Some("03001234567".into()),
                bank: Some("EasyPaisa".into()),
                confidence: 0.55,
                is_known_bank_layout: true,
            });
        }

        if image_object_key.contains("amount_mismatch") {
            return Ok(ExtractedPaymentDetails {
                tid: Some("TID_MISMATCH_888".into()),
                amount: Some(Money::from_major(500)),
                timestamp: Some(Utc::now()),
                sender: Some("03001234567".into()),
                bank: Some("JazzCash".into()),
                confidence: 0.95,
                is_known_bank_layout: true,
            });
        }

        if image_object_key.contains("stale_timestamp") {
            return Ok(ExtractedPaymentDetails {
                tid: Some("TID_STALE_777".into()),
                amount: Some(Money::from_major(1500)),
                timestamp: Some(Utc::now() - chrono::Duration::days(3)),
                sender: Some("03001234567".into()),
                bank: Some("JazzCash".into()),
                confidence: 0.95,
                is_known_bank_layout: true,
            });
        }

        if image_object_key.contains("before_order") {
            return Ok(ExtractedPaymentDetails {
                tid: Some("TID_BEFORE_ORDER_666".into()),
                amount: Some(Money::from_major(1500)),
                timestamp: Some(Utc::now() - chrono::Duration::hours(5)),
                sender: Some("03001234567".into()),
                bank: Some("Nayapay".into()),
                confidence: 0.95,
                is_known_bank_layout: true,
            });
        }

        if image_object_key.contains("reused_sender") {
            return Ok(ExtractedPaymentDetails {
                tid: Some("TID_REUSED_SENDER_555".into()),
                amount: Some(Money::from_major(1500)),
                timestamp: Some(Utc::now()),
                sender: Some("03129999999".into()),
                bank: Some("Meezan Bank".into()),
                confidence: 0.95,
                is_known_bank_layout: true,
            });
        }

        if image_object_key.contains("duplicate_tid") {
            return Ok(ExtractedPaymentDetails {
                tid: Some("TID_DUPLICATE_ALREADY_USED".into()),
                amount: Some(Money::from_major(1500)),
                timestamp: Some(Utc::now()),
                sender: Some("03001234567".into()),
                bank: Some("JazzCash".into()),
                confidence: 0.95,
                is_known_bank_layout: true,
            });
        }

        // Default valid high-confidence payment screenshot
        Ok(ExtractedPaymentDetails {
            tid: Some("TID_VALID_123456789".into()),
            amount: Some(Money::from_major(1500)),
            timestamp: Some(Utc::now()),
            sender: Some("03001234567".into()),
            bank: Some("JazzCash".into()),
            confidence: 0.96,
            is_known_bank_layout: true,
        })
    }
}
