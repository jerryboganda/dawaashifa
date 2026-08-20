use shifa_core::id::ProductId;
use shifa_core::money::Money;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CatalogError {
    #[error(
        "Sale price ({attempted}) cannot exceed maximum retail price ({mrp}) per DRAP regulations"
    )]
    AboveMrp { mrp: Money, attempted: Money },

    #[error("Product with ID '{0}' not found")]
    ProductNotFound(ProductId),

    #[error("Alias '{0}' already belongs to product '{1}' with higher weight")]
    AliasConflict(String, ProductId),

    #[error("Invalid alias '{0}': {1}")]
    InvalidAlias(String, &'static str),

    #[error("Unauthorized action: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    Db(#[from] shifa_db::DbError),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("CSV Import error: {0}")]
    Csv(#[from] csv::Error),
}
