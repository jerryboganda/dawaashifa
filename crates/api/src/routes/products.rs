use crate::error::ApiError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use shifa_catalog::models::*;
use shifa_core::context::TenantContext;
use shifa_core::id::ProductId;

#[derive(Debug, Deserialize)]
pub struct ProductListParams {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/products",
    params(
        ("q" = Option<String>, Query, description = "Search query"),
        ("limit" = Option<i64>, Query, description = "Limit"),
        ("offset" = Option<i64>, Query, description = "Offset")
    ),
    responses(
        (status = 200, description = "List products", body = Vec<ProductDto>),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = [])),
    tag = "Products"
)]
pub async fn list_products(
    State(state): State<AppState>,
    ctx: TenantContext,
    Query(params): Query<ProductListParams>,
) -> Result<Json<Vec<ProductDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    let products = state
        .catalog_service
        .list_products(&ctx, params.q.as_deref(), limit, offset)
        .await?;

    Ok(Json(products))
}

#[utoipa::path(
    get,
    path = "/api/v1/products/{id}",
    params(
        ("id" = uuid::Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product details", body = ProductDto),
        (status = 404, description = "Product not found")
    ),
    security(("bearer_auth" = [])),
    tag = "Products"
)]
pub async fn get_product(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ProductDto>, ApiError> {
    let product = state
        .catalog_service
        .get_product(&ctx, ProductId::from(id))
        .await?;

    Ok(Json(product))
}

#[utoipa::path(
    post,
    path = "/api/v1/products",
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Product created", body = ProductDto),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = [])),
    tag = "Products"
)]
pub async fn create_product(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductDto>), ApiError> {
    let product = state.catalog_service.create_product(&ctx, req).await?;

    Ok((StatusCode::CREATED, Json(product)))
}

#[utoipa::path(
    post,
    path = "/api/v1/products/match",
    request_body = MatchRequest,
    responses(
        (status = 200, description = "Matching candidate products", body = Vec<MatchCandidate>)
    ),
    security(("bearer_auth" = [])),
    tag = "Products"
)]
pub async fn match_products_handler(
    State(state): State<AppState>,
    ctx: TenantContext,
    Json(req): Json<MatchRequest>,
) -> Result<Json<Vec<MatchCandidate>>, ApiError> {
    let candidates = shifa_catalog::matching::match_product(&ctx, &state.pool, &req).await?;
    Ok(Json(candidates))
}

#[utoipa::path(
    get,
    path = "/api/v1/products/{id}/substitutes",
    params(
        ("id" = uuid::Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Substitution candidates", body = Vec<SubstitutionCandidate>),
        (status = 404, description = "Product not found")
    ),
    security(("bearer_auth" = [])),
    tag = "Products"
)]
pub async fn get_substitutes(
    State(state): State<AppState>,
    ctx: TenantContext,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Vec<SubstitutionCandidate>>, ApiError> {
    let substitutes = shifa_catalog::substitutions::substitution_candidates(
        &ctx,
        &state.pool,
        ProductId::from(id),
    )
    .await?;
    Ok(Json(substitutes))
}
