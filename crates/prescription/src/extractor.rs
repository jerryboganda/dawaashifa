use crate::models::{RxExtractedLine, RxExtraction};
use async_trait::async_trait;

#[async_trait]
pub trait RxVlmProvider: Send + Sync {
    async fn extract_prescription(&self, image_url: &str) -> Result<RxExtraction, String>;
}

#[derive(Debug, Clone, Default)]
pub struct MockRxVlmProvider;

#[async_trait]
impl RxVlmProvider for MockRxVlmProvider {
    async fn extract_prescription(&self, _image_url: &str) -> Result<RxExtraction, String> {
        // Return a realistic mock extraction per Doc 09 §7
        Ok(RxExtraction {
            doctor_name: Some("Dr. Tariq Mahmood".into()),
            doctor_pmdc_no: Some("12345-P".into()),
            issued_date: Some(chrono::Utc::now().date_naive()),
            patient_name: Some("Muhammad Usman".into()),
            lines: vec![
                RxExtractedLine {
                    line_no: 1,
                    raw_text: "Tab Panadol 500mg 1 TDS x 5 days".into(),
                    drug_text: Some("Panadol".into()),
                    strength_text: Some("500mg".into()),
                    form_text: Some("Tab".into()),
                    qty_text: Some("15".into()),
                    dosage_text: Some("1 tablet three times daily for 5 days".into()),
                    confidence: 0.92,
                },
                RxExtractedLine {
                    line_no: 2,
                    raw_text: "Cap Augmentin 625mg 1 BD x 7 days".into(),
                    drug_text: Some("Augmentin".into()),
                    strength_text: Some("625mg".into()),
                    form_text: Some("Cap".into()),
                    qty_text: Some("14".into()),
                    dosage_text: Some("1 capsule twice daily for 7 days".into()),
                    confidence: 0.88,
                },
                RxExtractedLine {
                    line_no: 3,
                    raw_text: "illegible scribble 20mg".into(),
                    drug_text: None, // Never guess illegible drug
                    strength_text: Some("20mg".into()),
                    form_text: None,
                    qty_text: None,
                    dosage_text: None,
                    confidence: 0.0,
                },
            ],
            overall_confidence: 0.85,
            warnings: vec![],
        })
    }
}
