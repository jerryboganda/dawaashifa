use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::MigrationError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub kind: String,
    pub table: Option<String>,
    pub connection_env: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub from: String,
    #[serde(default)]
    pub required: bool,
    pub transform: Option<String>,
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasConfig {
    pub generate_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupeConfig {
    pub strategy: String,
    pub match_on: Vec<String>,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    #[serde(default = "default_on_match")]
    pub on_match: String, // "skip", "update", "create_duplicate_flagged"
}

fn default_threshold() -> f64 {
    0.90
}

fn default_on_match() -> String {
    "skip".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub rule: String, // "greater_than_zero", "unique_within_batch", "not_empty"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    pub source: SourceConfig,
    pub target: String, // "products", "customers", "orders", "inventory"
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    pub fields: HashMap<String, FieldMapping>,
    pub aliases: Option<AliasConfig>,
    pub dedupe: Option<DedupeConfig>,
    pub validations: Option<Vec<ValidationRule>>,
}

fn default_batch_size() -> usize {
    500
}

impl MappingConfig {
    pub fn from_yaml_str(yaml_str: &str) -> Result<Self, MigrationError> {
        serde_yaml::from_str(yaml_str).map_err(|e| MigrationError::Mapping(e.to_string()))
    }
}
