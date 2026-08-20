use crate::error::CoreError;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use utoipa::ToSchema;

/// Monetary amount represented with exact precision using Decimal.
/// Invariant I-8: All money is rust_decimal::Decimal. Never f64 or f32.
/// Serializes over the wire as a quoted decimal string to preserve precision in frontend clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, ToSchema)]
#[schema(value_type = String, example = "1250.00")]
pub struct Money(pub Decimal);

impl Money {
    pub const ZERO: Self = Self(Decimal::ZERO);

    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    pub fn from_decimal(decimal: Decimal) -> Self {
        Self(decimal)
    }

    pub fn from_major(major: i64) -> Self {
        Self(Decimal::from(major))
    }

    pub fn from_minor(minor: i64, scale: u32) -> Result<Self, CoreError> {
        let mut d = Decimal::from(minor);
        d.set_scale(scale)
            .map_err(|_| CoreError::InvalidMoneyScale(scale))?;
        Ok(Self(d))
    }

    pub fn amount(&self) -> Decimal {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn is_positive(&self) -> bool {
        self.0.is_sign_positive() && !self.0.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.0.is_sign_negative()
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub fn checked_mul_qty(self, qty: i64) -> Option<Self> {
        self.0.checked_mul(Decimal::from(qty)).map(Self)
    }

    pub fn format_pkr(&self) -> String {
        format!("Rs {:.2}", self.0)
    }
}

impl From<Decimal> for Money {
    fn from(d: Decimal) -> Self {
        Self(d)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let d = s.parse::<Decimal>().map_err(serde::de::Error::custom)?;
        Ok(Self(d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_money_arithmetic() {
        let m1 = Money::from_decimal(Decimal::from_str("1250.50").expect("valid decimal"));
        let m2 = Money::from_decimal(Decimal::from_str("249.50").expect("valid decimal"));

        let sum = m1.checked_add(m2).expect("addition");
        assert_eq!(sum.amount(), Decimal::from_str("1500.00").expect("valid"));

        let diff = m1.checked_sub(m2).expect("subtraction");
        assert_eq!(diff.amount(), Decimal::from_str("1001.00").expect("valid"));

        let multiplied = m2.checked_mul_qty(3).expect("multiplication");
        assert_eq!(
            multiplied.amount(),
            Decimal::from_str("748.50").expect("valid")
        );
    }

    #[test]
    fn test_money_predicates() {
        let zero = Money::zero();
        assert!(zero.is_zero());
        assert!(!zero.is_positive());
        assert!(!zero.is_negative());

        let pos = Money::from_major(100);
        assert!(pos.is_positive());
        assert!(!pos.is_zero());
        assert!(!pos.is_negative());

        let neg = Money::from_major(-50);
        assert!(neg.is_negative());
        assert!(!neg.is_positive());
        assert!(!neg.is_zero());
    }

    #[test]
    fn test_money_string_serde() {
        let money = Money::from_decimal(Decimal::from_str("450.75").expect("decimal"));
        let json = serde_json::to_string(&money).expect("serialize");
        assert_eq!(json, "\"450.75\"");

        let deserialized: Money = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(money, deserialized);
    }

    #[test]
    fn test_pkr_formatting() {
        let money = Money::from_decimal(Decimal::from_str("1250.00").expect("decimal"));
        assert_eq!(money.format_pkr(), "Rs 1250.00");
    }
}
