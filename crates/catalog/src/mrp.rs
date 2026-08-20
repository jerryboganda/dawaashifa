use crate::error::CatalogError;
use crate::models::ProductDto;
use shifa_core::money::Money;

/// Enforce DRAP Maximum Retail Price (MRP) regulation per Doc 05 §4.
/// Invariant: Pharmacies may not charge above printed MRP. Hard block, not a warning.
pub fn validate_sale_price(p: &ProductDto, price: Money) -> Result<(), CatalogError> {
    if price > p.mrp {
        return Err(CatalogError::AboveMrp {
            mrp: p.mrp,
            attempted: price,
        });
    }
    Ok(())
}
