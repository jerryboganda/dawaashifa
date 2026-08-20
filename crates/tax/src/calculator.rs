use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, RoundingStrategy};
use shifa_core::money::Money;

use crate::error::TaxError;
use crate::models::{TaxCalculationResult, TaxCategoryDto, TaxLine};

#[derive(Debug, Clone)]
pub struct TaxableItemInput {
    pub item_name: String,
    pub unit_price: Money,
    pub quantity: i32,
    pub discount: Option<Money>,
    pub tax_category_name: String,
}

pub struct TaxCalculator;

impl TaxCalculator {
    /// Calculates tax for an order's items based on per-category rates effective at confirmation time (Doc 13 §5).
    ///
    /// Rules:
    /// 1. Rate selected by category's `effective_from` / `effective_to` window at `at`.
    /// 2. Rounding: half-up (MidpointAwayFromZero) to 2 decimals, applied **per line**, then summed.
    /// 3. Exempt and zero-rated items are distinct states and produce zero tax amount.
    /// 4. **No hardcoded tax rate in code** — rates come strictly from configuration/DB data.
    pub fn calculate_tax(
        items: &[TaxableItemInput],
        categories: &[TaxCategoryDto],
        at: DateTime<Utc>,
    ) -> Result<TaxCalculationResult, TaxError> {
        let mut total_taxable = Decimal::ZERO;
        let mut total_tax = Decimal::ZERO;
        let mut lines = Vec::new();

        for item in items {
            // Find applicable category matching name and effective window at `at`
            let category = categories
                .iter()
                .find(|c| {
                    c.name.eq_ignore_ascii_case(&item.tax_category_name)
                        && c.effective_from <= at
                        && c.effective_to.is_none_or(|to| to >= at)
                })
                .ok_or_else(|| TaxError::NoActiveRateForDate {
                    category: item.tax_category_name.clone(),
                    date: at.to_rfc3339(),
                })?;

            let qty_dec = Decimal::from(item.quantity);
            let raw_taxable = item.unit_price.0 * qty_dec;
            let discount_dec = item.discount.as_ref().map(|d| d.0).unwrap_or(Decimal::ZERO);
            let taxable_amount = (raw_taxable - discount_dec).max(Decimal::ZERO);

            let (rate, tax_amount, is_exempt, is_zero_rated) = if category.is_exempt {
                (Decimal::ZERO, Decimal::ZERO, true, false)
            } else if category.is_zero_rated {
                (Decimal::ZERO, Decimal::ZERO, false, true)
            } else {
                let rate = category.rate;
                let raw_tax = taxable_amount * (rate / Decimal::from(100));
                // Rounding: half-up to 2 decimals per line
                let rounded_tax =
                    raw_tax.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
                (rate, rounded_tax, false, false)
            };

            total_taxable += taxable_amount;
            total_tax += tax_amount;

            lines.push(TaxLine {
                item_name: item.item_name.clone(),
                taxable_amount: Money::from_decimal(taxable_amount),
                rate,
                tax_amount: Money::from_decimal(tax_amount),
                category_id: category.id,
                fbr_code: category.fbr_code.clone(),
                is_exempt,
                is_zero_rated,
            });
        }

        let total_amount = total_taxable + total_tax;

        Ok(TaxCalculationResult {
            subtotal: Money::from_decimal(total_taxable),
            tax_amount: Money::from_decimal(total_tax),
            total_amount: Money::from_decimal(total_amount),
            lines,
        })
    }
}
