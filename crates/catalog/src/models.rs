use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{CategoryId, GenericId, ProductId, TenantId};
use shifa_core::money::Money;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductDto {
    pub id: ProductId,
    pub tenant_id: TenantId,
    pub brand_name: String,
    pub generic_name: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub pack_size: Option<String>,
    pub mrp: Money,
    pub tp: Option<Money>,
    pub cost_price: Option<Money>,
    pub is_prescription_only: bool,
    pub is_narcotic: bool,
    pub is_refrigerated: bool,
    pub manufacturer: Option<String>,
    pub barcode: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub brand_name: String,
    pub generic_name: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub pack_size: Option<String>,
    pub mrp: Money,
    pub tp: Option<Money>,
    pub cost_price: Option<Money>,
    pub is_prescription_only: bool,
    pub is_narcotic: bool,
    pub is_refrigerated: bool,
    pub manufacturer: Option<String>,
    pub barcode: Option<String>,
    pub category_id: Option<CategoryId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub brand_name: Option<String>,
    pub generic_name: Option<String>,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub pack_size: Option<String>,
    pub mrp: Option<Money>,
    pub tp: Option<Money>,
    pub cost_price: Option<Money>,
    pub is_prescription_only: Option<bool>,
    pub is_narcotic: Option<bool>,
    pub is_refrigerated: Option<bool>,
    pub manufacturer: Option<String>,
    pub barcode: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum MatchMethod {
    Exact,
    Alias,
    Trigram,
    Phonetic,
    Vector,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatchCandidate {
    pub product_id: ProductId,
    pub brand_name: String,
    pub strength: Option<String>,
    pub score: f32,
    pub method: MatchMethod,
    pub matched_on: String,
    pub is_prescription_only: bool,
    pub mrp: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MatchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub branch_id: Option<Uuid>,
}

fn default_limit() -> usize {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubstitutionCandidate {
    pub product_id: ProductId,
    pub brand_name: String,
    pub generic_name: String,
    pub strength: String,
    pub mrp: Money,
    pub savings_vs_original: Money,
    pub equivalence_type: String,
    pub requires_pharmacist_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductAliasDto {
    pub id: Uuid,
    pub product_id: ProductId,
    pub alias: String,
    pub alias_type: String,
    pub script: String,
    pub weight: f64,
    pub source: String,
    pub hit_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenericDto {
    pub id: GenericId,
    pub name: String,
    pub therapeutic_class: Option<String>,
}
