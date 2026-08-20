use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::TaxError;
use crate::models::{FiscalStatusResponse, FiscalSubmissionResponse, InvoiceDto};

#[async_trait]
pub trait FiscalReporter: Send + Sync {
    async fn submit(&self, invoice: &InvoiceDto) -> Result<FiscalSubmissionResponse, TaxError>;
    async fn status(&self, fbr_reference: &str) -> Result<FiscalStatusResponse, TaxError>;
    async fn void(&self, fbr_reference: &str, reason: &str) -> Result<(), TaxError>;
}

/// Generates FBR standard QR payload format (Doc 13 §8)
pub fn generate_fbr_qr_payload(
    pos_id: &str,
    invoice_no: &str,
    fiscal_invoice_no: &str,
    total_amount: &str,
    tax_amount: &str,
    timestamp: &str,
) -> String {
    format!(
        "POS_ID:{pos_id}|INV:{invoice_no}|FISC:{fiscal_invoice_no}|TOT:{total_amount}|TAX:{tax_amount}|TS:{timestamp}"
    )
}

#[derive(Debug, Clone)]
pub enum MockFbrBehavior {
    AlwaysAccept,
    RejectValidation { reason: String, code: String },
    OutageNetworkFailure { message: String },
}

#[derive(Clone)]
pub struct MockFbrReporter {
    behavior: Arc<Mutex<MockFbrBehavior>>,
    pub submit_count: Arc<AtomicUsize>,
    pub void_count: Arc<AtomicUsize>,
}

impl MockFbrReporter {
    pub fn new(behavior: MockFbrBehavior) -> Self {
        Self {
            behavior: Arc::new(Mutex::new(behavior)),
            submit_count: Arc::new(AtomicUsize::new(0)),
            void_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn set_behavior(&self, behavior: MockFbrBehavior) {
        let mut b = self.behavior.lock().await;
        *b = behavior;
    }
}

#[async_trait]
impl FiscalReporter for MockFbrReporter {
    async fn submit(&self, invoice: &InvoiceDto) -> Result<FiscalSubmissionResponse, TaxError> {
        self.submit_count.fetch_add(1, Ordering::SeqCst);
        let behavior = self.behavior.lock().await.clone();

        match behavior {
            MockFbrBehavior::AlwaysAccept => {
                let fiscal_no = format!("FBR-{}", Uuid::now_v7().simple());
                let qr = generate_fbr_qr_payload(
                    "POS-LHR-01",
                    &invoice.invoice_no,
                    &fiscal_no,
                    &invoice.total_amount.to_string(),
                    &invoice.tax_amount.to_string(),
                    &Utc::now().to_rfc3339(),
                );

                Ok(FiscalSubmissionResponse {
                    fiscal_invoice_no: fiscal_no.clone(),
                    fbr_invoice_number: fiscal_no,
                    qr_code_data: qr,
                    status: "ACCEPTED".into(),
                    raw_response: json!({
                        "response_code": "100",
                        "status": "VALID",
                        "fbr_invoice_number": invoice.invoice_no,
                    }),
                })
            }
            MockFbrBehavior::RejectValidation { reason, code } => Err(TaxError::FbrRejection {
                reason,
                code: Some(code),
            }),
            MockFbrBehavior::OutageNetworkFailure { message } => {
                Err(TaxError::FbrOutage { message })
            }
        }
    }

    async fn status(&self, fbr_reference: &str) -> Result<FiscalStatusResponse, TaxError> {
        Ok(FiscalStatusResponse {
            status: "ACCEPTED".into(),
            fbr_reference: fbr_reference.to_string(),
            verified_at: Utc::now(),
        })
    }

    async fn void(&self, _fbr_reference: &str, _reason: &str) -> Result<(), TaxError> {
        self.void_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
