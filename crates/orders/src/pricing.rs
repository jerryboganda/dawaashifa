use crate::error::OrderError;
use rust_decimal::Decimal;
use shifa_core::money::Money;

/// Calculate line total using exact Decimal precision per Invariant I-8.
pub fn calculate_line_total(qty: i32, unit_price: Money, line_discount: Money) -> Money {
    let qty_dec = Decimal::from(qty);
    let gross = unit_price.amount() * qty_dec;
    let net = (gross - line_discount.amount()).max(Decimal::ZERO);
    Money::from_decimal(net)
}

/// Calculate order total from subtotal, discounts, delivery fee, and tax.
pub fn calculate_order_total(
    subtotal: Money,
    order_discount: Money,
    delivery_fee: Money,
    tax_amount: Money,
) -> Money {
    let net = (subtotal.amount() - order_discount.amount()).max(Decimal::ZERO);
    let gross = net + delivery_fee.amount() + tax_amount.amount();
    Money::from_decimal(gross)
}

/// Enforce that unit price does not exceed product MRP per Doc 10 §8.
pub fn validate_item_price(unit_price: Money, mrp: Money) -> Result<(), OrderError> {
    if unit_price.amount() > mrp.amount() {
        return Err(OrderError::AboveMrp {
            attempted: unit_price.to_string(),
            mrp: mrp.to_string(),
        });
    }
    Ok(())
}
