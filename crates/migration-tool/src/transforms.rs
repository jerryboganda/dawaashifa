use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::error::MigrationError;

pub struct TransformEngine;

impl TransformEngine {
    /// Applies a named transform to a raw string value
    pub fn apply(
        name: &str,
        value: &str,
        field_name: &str,
    ) -> Result<serde_json::Value, MigrationError> {
        let val_clean = Self::arabic_digits_to_ascii(value);
        let trimmed = val_clean.trim();

        match name {
            "trim" => Ok(serde_json::Value::String(trimmed.to_string())),
            "trim_upper" => Ok(serde_json::Value::String(trimmed.to_uppercase())),
            "title_case" => Ok(serde_json::Value::String(Self::title_case(trimmed))),
            "parse_decimal" => {
                let dec =
                    Self::parse_decimal(trimmed).map_err(|reason| MigrationError::Transform {
                        field: field_name.to_string(),
                        value: value.to_string(),
                        reason,
                    })?;
                Ok(serde_json::Value::String(dec.to_string()))
            }
            "parse_bool" => {
                let b = Self::parse_bool(trimmed);
                Ok(serde_json::Value::Bool(b))
            }
            "parse_date" => {
                let dt = Self::parse_date(trimmed).map_err(|reason| MigrationError::Transform {
                    field: field_name.to_string(),
                    value: value.to_string(),
                    reason,
                })?;
                Ok(serde_json::Value::String(dt))
            }
            "normalize_strength" => {
                let s = Self::normalize_strength(trimmed);
                Ok(serde_json::Value::String(s))
            }
            "parse_pack_size" => {
                let size = Self::parse_pack_size(trimmed);
                Ok(serde_json::Value::Number(size.into()))
            }
            "normalize_phone" => {
                let phone =
                    Self::normalize_phone(trimmed).map_err(|reason| MigrationError::Transform {
                        field: field_name.to_string(),
                        value: value.to_string(),
                        reason,
                    })?;
                Ok(serde_json::Value::String(phone))
            }
            "arabic_digits_to_ascii" => Ok(serde_json::Value::String(val_clean)),
            "cold_chain_from_storage" => {
                let is_cold = Self::cold_chain_from_storage(trimmed);
                Ok(serde_json::Value::Bool(is_cold))
            }
            unknown => Err(MigrationError::Transform {
                field: field_name.to_string(),
                value: value.to_string(),
                reason: format!("Unknown transform function: '{}'", unknown),
            }),
        }
    }

    pub fn title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn parse_decimal(s: &str) -> Result<Decimal, String> {
        let cleaned: String = s
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        Decimal::from_str(&cleaned).map_err(|e| format!("Invalid decimal value: {}", e))
    }

    pub fn parse_bool(s: &str) -> bool {
        matches!(
            s.trim().to_lowercase().as_str(),
            "1" | "true" | "yes" | "y" | "t" | "rx" | "prescription"
        )
    }

    pub fn parse_date(s: &str) -> Result<String, String> {
        let clean = s.trim();
        // Supported formats: DD/MM/YYYY, DD-MM-YYYY, YYYY-MM-DD, YYYY/MM/DD
        if let Ok(d) = NaiveDate::parse_from_str(clean, "%d/%m/%Y") {
            return Ok(d.format("%Y-%m-%d").to_string());
        }
        if let Ok(d) = NaiveDate::parse_from_str(clean, "%d-%m-%Y") {
            return Ok(d.format("%Y-%m-%d").to_string());
        }
        if let Ok(d) = NaiveDate::parse_from_str(clean, "%Y-%m-%d") {
            return Ok(d.format("%Y-%m-%d").to_string());
        }
        if let Ok(d) = NaiveDate::parse_from_str(clean, "%Y/%m/%d") {
            return Ok(d.format("%Y-%m-%d").to_string());
        }
        Err(format!(
            "Unable to parse date '{}' with supported formats",
            clean
        ))
    }

    pub fn normalize_strength(s: &str) -> String {
        let lower = s.to_lowercase().replace(' ', "");
        if lower.ends_with("mg") {
            return lower;
        }
        if lower.ends_with('g') {
            if let Ok(num) = lower.trim_end_matches('g').parse::<f64>() {
                let mg = (num * 1000.0).round() as i64;
                return format!("{}mg", mg);
            }
        }
        if lower.ends_with("mcg")
            || lower.ends_with("ml")
            || lower.ends_with("iu")
            || lower.ends_with('%')
        {
            return lower;
        }
        // If raw number without unit, default to mg
        if lower.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return format!("{}mg", lower);
        }
        lower
    }

    pub fn parse_pack_size(s: &str) -> i32 {
        let lower = s.to_lowercase();
        // Handle forms like 10x10 -> 100
        if lower.contains('x') {
            let parts: Vec<&str> = lower.split('x').collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (
                    parts[0]
                        .trim()
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<i32>(),
                    parts[1]
                        .trim()
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<i32>(),
                ) {
                    return (a * b).max(1);
                }
            }
        }

        let digits: String = lower.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse::<i32>().unwrap_or(1).max(1)
    }

    pub fn normalize_phone(s: &str) -> Result<String, String> {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return Err("Empty phone number".into());
        }

        // Pakistan number normalization:
        // 03001234567 -> +923001234567
        // 923001234567 -> +923001234567
        // 3001234567 -> +923001234567
        if digits.starts_with("03") && digits.len() == 11 {
            return Ok(format!("+92{}", &digits[1..]));
        }
        if digits.starts_with("923") && digits.len() == 12 {
            return Ok(format!("+{}", digits));
        }
        if digits.starts_with('3') && digits.len() == 10 {
            return Ok(format!("+92{}", digits));
        }
        if digits.len() >= 10 {
            return Ok(format!("+{}", digits));
        }

        Err(format!("Invalid Pakistani phone number length: {}", digits))
    }

    pub fn arabic_digits_to_ascii(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '۰' | '٠' => '0',
                '۱' | '١' => '1',
                '۲' | '٢' => '2',
                '۳' | '٣' => '3',
                '۴' | '٤' => '4',
                '۵' | '٥' => '5',
                '۶' | '٦' => '6',
                '۷' | '٧' => '7',
                '۸' | '٨' => '8',
                '۹' | '٩' => '9',
                other => other,
            })
            .collect()
    }

    pub fn cold_chain_from_storage(s: &str) -> bool {
        let lower = s.to_lowercase();
        lower.contains("refrigerat")
            || lower.contains("cold")
            || lower.contains("2-8")
            || lower.contains("2 - 8")
            || lower.contains("chilled")
            || lower.contains("frozen")
    }
}
