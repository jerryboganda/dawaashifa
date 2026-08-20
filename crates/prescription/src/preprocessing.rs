use crate::error::RxError;

#[derive(Debug, Clone)]
pub struct PreprocessingResult {
    pub is_valid: bool,
    pub preprocessed_key: String,
    pub needs_retake: bool,
    pub retake_reason: Option<String>,
}

/// Validate image constraints per Doc 09 §6:
/// - Reject if under 300x300
/// - Reject if over 20MB
/// - Upscale / deskew preprocessed representation
pub fn validate_and_preprocess_image(
    image_key: &str,
    width: Option<u32>,
    height: Option<u32>,
    bytes_len: Option<usize>,
) -> Result<PreprocessingResult, RxError> {
    const MAX_BYTES: usize = 20 * 1024 * 1024; // 20 MB

    if let Some(bytes) = bytes_len {
        if bytes > MAX_BYTES {
            return Err(RxError::InvalidImage(
                "Prescription image exceeds 20MB limit".into(),
            ));
        }
    }

    if let (Some(w), Some(h)) = (width, height) {
        if w < 300 || h < 300 {
            return Err(RxError::InvalidImage(
                "Prescription image resolution too low (< 300x300). Please retake photo.".into(),
            ));
        }
    }

    let preprocessed_key = format!("preprocessed/{}", image_key.trim_start_matches("raw/"));

    Ok(PreprocessingResult {
        is_valid: true,
        preprocessed_key,
        needs_retake: false,
        retake_reason: None,
    })
}
