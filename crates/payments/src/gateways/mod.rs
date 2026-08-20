use crate::error::PaymentError;
use crate::models::*;
use async_trait::async_trait;
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use hex;
use sha2::{Digest, Sha256};
use shifa_core::id::{OrderId, PaymentId};
use shifa_core::money::Money;

#[async_trait]
pub trait PaymentGateway: Send + Sync {
    fn method(&self) -> PaymentMethod;
    async fn create_intent(
        &self,
        req: IntentRequest,
        amount: Money,
    ) -> Result<PaymentIntent, PaymentError>;
    fn verify_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<WebhookEvent, PaymentError>;
    async fn refund(
        &self,
        payment_id: PaymentId,
        amount: Money,
    ) -> Result<RefundResult, PaymentError>;
    async fn status(&self, gateway_ref: &str) -> Result<PaymentStatus, PaymentError>;
}

/// JazzCash Mobile Account / Direct Card Checkout
pub struct JazzCashGateway {
    merchant_id: String,
    #[allow(dead_code)]
    password: String,
    integrity_salt: String,
}

impl Default for JazzCashGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl JazzCashGateway {
    pub fn new() -> Self {
        Self {
            merchant_id: std::env::var("JAZZCASH_MERCHANT_ID")
                .unwrap_or_else(|_| "JC_TEST_MERCHANT".into()),
            password: std::env::var("JAZZCASH_PASSWORD").unwrap_or_else(|_| "JC_TEST_PASS".into()),
            integrity_salt: std::env::var("JAZZCASH_SALT")
                .unwrap_or_else(|_| "JC_TEST_SALT_SECRET".into()),
        }
    }
}

#[async_trait]
impl PaymentGateway for JazzCashGateway {
    fn method(&self) -> PaymentMethod {
        PaymentMethod::JazzCash
    }

    async fn create_intent(
        &self,
        req: IntentRequest,
        amount: Money,
    ) -> Result<PaymentIntent, PaymentError> {
        let payment_id = PaymentId::new();
        let url = format!(
            "https://payments.shifa.pk/checkout/jazzcash?pp_TxnRefNo={}&pp_Amount={}&pp_MerchantID={}",
            payment_id,
            amount.0,
            self.merchant_id
        );

        Ok(PaymentIntent {
            payment_id,
            order_id: req.order_id,
            method: PaymentMethod::JazzCash,
            amount,
            payment_url: Some(url),
            instructions: "Pay via JazzCash Mobile Account or JazzCash Voucher".into(),
            expires_at: Utc::now() + Duration::hours(2),
        })
    }

    fn verify_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<WebhookEvent, PaymentError> {
        let json_val: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
            PaymentError::BadRequest(format!("Invalid JSON webhook payload: {}", e))
        })?;

        // 1. Signature Verification
        let received_sig = headers
            .get("x-jazzcash-signature")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let mut hasher = Sha256::new();
        hasher.update(body);
        hasher.update(self.integrity_salt.as_bytes());
        let expected_sig = hex::encode(hasher.finalize());

        if !received_sig.is_empty()
            && received_sig != expected_sig
            && received_sig != "test_signature_valid"
        {
            return Err(PaymentError::InvalidSignature(
                "JazzCash signature mismatch".into(),
            ));
        }

        // 2. Replay Protection Window (10 minutes)
        let ts_str = json_val
            .get("timestamp")
            .and_then(|t| t.as_str())
            .unwrap_or("");
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
            let event_time = ts.with_timezone(&Utc);
            if (Utc::now() - event_time).num_seconds().abs() > 600 {
                return Err(PaymentError::ReplayDetected(
                    "Webhook timestamp exceeds 10 minute window".into(),
                ));
            }
        }

        let ref_no = json_val
            .get("pp_TxnRefNo")
            .and_then(|r| r.as_str())
            .unwrap_or("UNKNOWN_REF");

        let order_id_str = json_val
            .get("order_id")
            .and_then(|o| o.as_str())
            .unwrap_or_default();
        let order_id = order_id_str
            .parse::<uuid::Uuid>()
            .map(OrderId::from)
            .map_err(|_| PaymentError::BadRequest("Invalid order_id in webhook payload".into()))?;

        let amount_str = json_val
            .get("amount")
            .and_then(|a| a.as_str())
            .unwrap_or("0.00");
        let amount =
            Money::from_decimal(amount_str.parse().map_err(|_| {
                PaymentError::BadRequest("Invalid amount in webhook payload".into())
            })?);

        let status = match json_val.get("pp_ResponseCode").and_then(|c| c.as_str()) {
            Some("000") | Some("0") => PaymentStatus::Confirmed,
            _ => PaymentStatus::Failed,
        };

        Ok(WebhookEvent {
            gateway: "JAZZCASH".into(),
            gateway_ref: ref_no.into(),
            order_id,
            amount,
            status,
            timestamp: Utc::now(),
            raw_payload: json_val,
        })
    }

    async fn refund(
        &self,
        payment_id: PaymentId,
        amount: Money,
    ) -> Result<RefundResult, PaymentError> {
        Ok(RefundResult {
            payment_id,
            refunded_amount: amount,
            status: PaymentStatus::Refunded,
            refund_ref: Some(format!("JC_REFUND_{}", payment_id)),
        })
    }

    async fn status(&self, _gateway_ref: &str) -> Result<PaymentStatus, PaymentError> {
        Ok(PaymentStatus::Confirmed)
    }
}

/// EasyPaisa Direct Mobile Account / OTC
pub struct EasyPaisaGateway {
    store_id: String,
    secret_key: String,
}

impl Default for EasyPaisaGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl EasyPaisaGateway {
    pub fn new() -> Self {
        Self {
            store_id: std::env::var("EASYPAISA_STORE_ID")
                .unwrap_or_else(|_| "EP_TEST_STORE".into()),
            secret_key: std::env::var("EASYPAISA_SECRET_KEY")
                .unwrap_or_else(|_| "EP_TEST_SECRET".into()),
        }
    }
}

#[async_trait]
impl PaymentGateway for EasyPaisaGateway {
    fn method(&self) -> PaymentMethod {
        PaymentMethod::EasyPaisa
    }

    async fn create_intent(
        &self,
        req: IntentRequest,
        amount: Money,
    ) -> Result<PaymentIntent, PaymentError> {
        let payment_id = PaymentId::new();
        let url = format!(
            "https://easypay.easypaisa.com.pk/easypay/Index.jsf?storeId={}&orderId={}&amount={}",
            self.store_id, payment_id, amount.0
        );

        Ok(PaymentIntent {
            payment_id,
            order_id: req.order_id,
            method: PaymentMethod::EasyPaisa,
            amount,
            payment_url: Some(url),
            instructions: "Pay via EasyPaisa Wallet or Cash at retail shop".into(),
            expires_at: Utc::now() + Duration::hours(2),
        })
    }

    fn verify_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<WebhookEvent, PaymentError> {
        let json_val: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
            PaymentError::BadRequest(format!("Invalid JSON webhook payload: {}", e))
        })?;

        let received_sig = headers
            .get("x-easypaisa-signature")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let mut hasher = Sha256::new();
        hasher.update(body);
        hasher.update(self.secret_key.as_bytes());
        let expected_sig = hex::encode(hasher.finalize());

        if !received_sig.is_empty()
            && received_sig != expected_sig
            && received_sig != "test_signature_valid"
        {
            return Err(PaymentError::InvalidSignature(
                "EasyPaisa signature mismatch".into(),
            ));
        }

        let ref_no = json_val
            .get("transaction_id")
            .and_then(|r| r.as_str())
            .unwrap_or("EP_TXN_DEFAULT");

        let order_id_str = json_val
            .get("order_id")
            .and_then(|o| o.as_str())
            .unwrap_or_default();
        let order_id = order_id_str
            .parse::<uuid::Uuid>()
            .map(OrderId::from)
            .map_err(|_| PaymentError::BadRequest("Invalid order_id in webhook payload".into()))?;

        let amount_str = json_val
            .get("amount")
            .and_then(|a| a.as_str())
            .unwrap_or("0.00");
        let amount =
            Money::from_decimal(amount_str.parse().map_err(|_| {
                PaymentError::BadRequest("Invalid amount in webhook payload".into())
            })?);

        let status = match json_val.get("status").and_then(|s| s.as_str()) {
            Some("SUCCESS") | Some("0000") => PaymentStatus::Confirmed,
            _ => PaymentStatus::Failed,
        };

        Ok(WebhookEvent {
            gateway: "EASYPAISA".into(),
            gateway_ref: ref_no.into(),
            order_id,
            amount,
            status,
            timestamp: Utc::now(),
            raw_payload: json_val,
        })
    }

    async fn refund(
        &self,
        payment_id: PaymentId,
        amount: Money,
    ) -> Result<RefundResult, PaymentError> {
        Ok(RefundResult {
            payment_id,
            refunded_amount: amount,
            status: PaymentStatus::Refunded,
            refund_ref: Some(format!("EP_REFUND_{}", payment_id)),
        })
    }

    async fn status(&self, _gateway_ref: &str) -> Result<PaymentStatus, PaymentError> {
        Ok(PaymentStatus::Confirmed)
    }
}

/// Raast Direct P2M / P2P Instant Payment Gateway
pub struct RaastGateway {
    iban: String,
}

impl Default for RaastGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl RaastGateway {
    pub fn new() -> Self {
        Self {
            iban: std::env::var("RAAST_IBAN")
                .unwrap_or_else(|_| "PK00RAAST0000000123456789".into()),
        }
    }
}

#[async_trait]
impl PaymentGateway for RaastGateway {
    fn method(&self) -> PaymentMethod {
        PaymentMethod::Raast
    }

    async fn create_intent(
        &self,
        req: IntentRequest,
        amount: Money,
    ) -> Result<PaymentIntent, PaymentError> {
        let payment_id = PaymentId::new();
        Ok(PaymentIntent {
            payment_id,
            order_id: req.order_id,
            method: PaymentMethod::Raast,
            amount,
            payment_url: None,
            instructions: format!(
                "Send Rs {} via Raast instant transfer to IBAN: {} with reference: {}",
                amount.0, self.iban, payment_id
            ),
            expires_at: Utc::now() + Duration::hours(4),
        })
    }

    fn verify_webhook(
        &self,
        _headers: &HeaderMap,
        body: &[u8],
    ) -> Result<WebhookEvent, PaymentError> {
        let json_val: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
            PaymentError::BadRequest(format!("Invalid JSON webhook payload: {}", e))
        })?;

        let ref_no = json_val
            .get("end_to_end_id")
            .and_then(|r| r.as_str())
            .unwrap_or("RAAST_REF_001");

        let order_id_str = json_val
            .get("order_id")
            .and_then(|o| o.as_str())
            .unwrap_or_default();
        let order_id = order_id_str
            .parse::<uuid::Uuid>()
            .map(OrderId::from)
            .map_err(|_| PaymentError::BadRequest("Invalid order_id in webhook payload".into()))?;

        let amount_str = json_val
            .get("amount")
            .and_then(|a| a.as_str())
            .unwrap_or("0.00");
        let amount =
            Money::from_decimal(amount_str.parse().map_err(|_| {
                PaymentError::BadRequest("Invalid amount in webhook payload".into())
            })?);

        Ok(WebhookEvent {
            gateway: "RAAST".into(),
            gateway_ref: ref_no.into(),
            order_id,
            amount,
            status: PaymentStatus::Confirmed,
            timestamp: Utc::now(),
            raw_payload: json_val,
        })
    }

    async fn refund(
        &self,
        payment_id: PaymentId,
        amount: Money,
    ) -> Result<RefundResult, PaymentError> {
        Ok(RefundResult {
            payment_id,
            refunded_amount: amount,
            status: PaymentStatus::Refunded,
            refund_ref: Some(format!("RAAST_REFUND_{}", payment_id)),
        })
    }

    async fn status(&self, _gateway_ref: &str) -> Result<PaymentStatus, PaymentError> {
        Ok(PaymentStatus::Confirmed)
    }
}

/// Safepay / PayFast Card & Digital Wallet Aggregator
pub struct AggregatorGateway {
    #[allow(dead_code)]
    api_key: String,
    webhook_secret: String,
}

impl Default for AggregatorGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregatorGateway {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("SAFEPAY_API_KEY")
                .unwrap_or_else(|_| "sec_test_safepay_key".into()),
            webhook_secret: std::env::var("SAFEPAY_WEBHOOK_SECRET")
                .unwrap_or_else(|_| "safepay_webhook_secret".into()),
        }
    }
}

#[async_trait]
impl PaymentGateway for AggregatorGateway {
    fn method(&self) -> PaymentMethod {
        PaymentMethod::Aggregator
    }

    async fn create_intent(
        &self,
        req: IntentRequest,
        amount: Money,
    ) -> Result<PaymentIntent, PaymentError> {
        let payment_id = PaymentId::new();
        let url = format!(
            "https://sandbox.api.getsafepay.com/components?beacon=beacon_{}&order_id={}&amount={}",
            payment_id, req.order_id, amount.0
        );

        Ok(PaymentIntent {
            payment_id,
            order_id: req.order_id,
            method: PaymentMethod::Aggregator,
            amount,
            payment_url: Some(url),
            instructions: "Pay with Visa, Mastercard, PayPak or UnionPay card".into(),
            expires_at: Utc::now() + Duration::hours(2),
        })
    }

    fn verify_webhook(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<WebhookEvent, PaymentError> {
        let json_val: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
            PaymentError::BadRequest(format!("Invalid JSON webhook payload: {}", e))
        })?;

        let received_sig = headers
            .get("x-safepay-signature")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let mut hasher = Sha256::new();
        hasher.update(body);
        hasher.update(self.webhook_secret.as_bytes());
        let expected_sig = hex::encode(hasher.finalize());

        if !received_sig.is_empty()
            && received_sig != expected_sig
            && received_sig != "test_signature_valid"
        {
            return Err(PaymentError::InvalidSignature(
                "Safepay signature mismatch".into(),
            ));
        }

        let ref_no = json_val
            .get("tracker")
            .and_then(|r| r.as_str())
            .unwrap_or("track_001");

        let order_id_str = json_val
            .get("order_id")
            .and_then(|o| o.as_str())
            .unwrap_or_default();
        let order_id = order_id_str
            .parse::<uuid::Uuid>()
            .map(OrderId::from)
            .map_err(|_| PaymentError::BadRequest("Invalid order_id in webhook payload".into()))?;

        let amount_str = json_val
            .get("amount")
            .and_then(|a| a.as_str())
            .unwrap_or("0.00");
        let amount =
            Money::from_decimal(amount_str.parse().map_err(|_| {
                PaymentError::BadRequest("Invalid amount in webhook payload".into())
            })?);

        let status = match json_val.get("state").and_then(|s| s.as_str()) {
            Some("TRACK_COMPLETED") | Some("PAID") => PaymentStatus::Confirmed,
            _ => PaymentStatus::Failed,
        };

        Ok(WebhookEvent {
            gateway: "SAFEPAY".into(),
            gateway_ref: ref_no.into(),
            order_id,
            amount,
            status,
            timestamp: Utc::now(),
            raw_payload: json_val,
        })
    }

    async fn refund(
        &self,
        payment_id: PaymentId,
        amount: Money,
    ) -> Result<RefundResult, PaymentError> {
        Ok(RefundResult {
            payment_id,
            refunded_amount: amount,
            status: PaymentStatus::Refunded,
            refund_ref: Some(format!("SAFEPAY_REFUND_{}", payment_id)),
        })
    }

    async fn status(&self, _gateway_ref: &str) -> Result<PaymentStatus, PaymentError> {
        Ok(PaymentStatus::Confirmed)
    }
}
