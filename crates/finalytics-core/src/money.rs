//! Exact amounts of money.

use std::fmt;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use crate::{Currency, Error, Result};

/// How to round an amount to its currency's minor units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Rounding {
    /// To the nearest; halves go to the even neighbour (banker's rounding).
    #[default]
    HalfEven,
    /// To the nearest; halves go away from zero.
    HalfAwayFromZero,
    /// Towards zero (truncation).
    TowardZero,
    /// Away from zero.
    AwayFromZero,
}

impl Rounding {
    fn strategy(self) -> RoundingStrategy {
        match self {
            Rounding::HalfEven => RoundingStrategy::MidpointNearestEven,
            Rounding::HalfAwayFromZero => RoundingStrategy::MidpointAwayFromZero,
            Rounding::TowardZero => RoundingStrategy::ToZero,
            Rounding::AwayFromZero => RoundingStrategy::AwayFromZero,
        }
    }
}

/// An exact decimal amount in one currency.
///
/// Arithmetic between amounts checks that the currencies match and that no overflow occurs.
/// Amounts keep their full precision until they are rounded.
///
/// ```
/// # mod finalytics { pub use finalytics_core::*; }
/// use finalytics::{Currency, Decimal, Money, Rounding};
///
/// let price = Money::new(Decimal::new(10_125, 3), Currency::USD); // 10.125 USD
/// let cost = price.multiply(Decimal::from(3))?;                   // 30.375 USD
/// assert_eq!(cost.round(Rounding::HalfEven).to_string(), "30.38 USD");
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    /// An amount of `currency`.
    pub fn new(amount: Decimal, currency: Currency) -> Money {
        Money { amount, currency }
    }

    /// Zero in `currency`.
    pub fn zero(currency: Currency) -> Money {
        Money {
            amount: Decimal::ZERO,
            currency,
        }
    }

    /// The amount.
    pub fn amount(&self) -> Decimal {
        self.amount
    }

    /// The currency.
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Whether the amount is zero.
    pub fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }

    /// Whether the amount is below zero.
    pub fn is_negative(&self) -> bool {
        self.amount.is_sign_negative() && !self.amount.is_zero()
    }

    /// The sum of two amounts in the same currency.
    pub fn checked_add(&self, other: Money) -> Result<Money> {
        self.same_currency(other)?;
        let amount = self
            .amount
            .checked_add(other.amount)
            .ok_or(Error::Overflow)?;
        Ok(Money {
            amount,
            currency: self.currency,
        })
    }

    /// The difference of two amounts in the same currency.
    pub fn checked_sub(&self, other: Money) -> Result<Money> {
        self.same_currency(other)?;
        let amount = self
            .amount
            .checked_sub(other.amount)
            .ok_or(Error::Overflow)?;
        Ok(Money {
            amount,
            currency: self.currency,
        })
    }

    /// This amount multiplied by a plain number, such as a quantity.
    pub fn multiply(&self, factor: Decimal) -> Result<Money> {
        let amount = self.amount.checked_mul(factor).ok_or(Error::Overflow)?;
        Ok(Money {
            amount,
            currency: self.currency,
        })
    }

    /// The amount with its sign reversed.
    pub fn negate(&self) -> Money {
        Money {
            amount: -self.amount,
            currency: self.currency,
        }
    }

    /// The amount rounded to its currency's minor units.
    pub fn round(&self, rounding: Rounding) -> Money {
        let amount = self
            .amount
            .round_dp_with_strategy(self.currency.minor_units(), rounding.strategy());
        Money {
            amount,
            currency: self.currency,
        }
    }

    fn same_currency(&self, other: Money) -> Result<()> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(Error::CurrencyMismatch {
                left: self.currency,
                right: other.currency,
            })
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn usd(text: &str) -> Money {
        Money::new(text.parse().unwrap(), Currency::USD)
    }

    #[test]
    fn arithmetic_requires_one_currency() {
        assert_eq!(usd("1.10").checked_add(usd("2.25")).unwrap(), usd("3.35"));
        assert_eq!(usd("1.10").checked_sub(usd("2.25")).unwrap(), usd("-1.15"));
        let euros = Money::new(Decimal::ONE, Currency::EUR);
        assert_eq!(
            usd("1").checked_add(euros),
            Err(Error::CurrencyMismatch {
                left: Currency::USD,
                right: Currency::EUR
            })
        );
    }

    #[test]
    fn rounding_follows_the_currency_and_the_rule() {
        assert_eq!(usd("2.345").round(Rounding::HalfEven), usd("2.34"));
        assert_eq!(usd("2.355").round(Rounding::HalfEven), usd("2.36"));
        assert_eq!(usd("2.345").round(Rounding::HalfAwayFromZero), usd("2.35"));
        assert_eq!(usd("-2.349").round(Rounding::TowardZero), usd("-2.34"));
        assert_eq!(usd("2.341").round(Rounding::AwayFromZero), usd("2.35"));
        let yen = Money::new("1234.5".parse().unwrap(), Currency::JPY);
        assert_eq!(yen.round(Rounding::HalfEven).amount(), Decimal::from(1234));
    }

    #[test]
    fn signs() {
        assert!(usd("-0.01").is_negative());
        assert!(!usd("-0.00").is_negative());
        assert!(usd("0.00").is_zero());
        assert_eq!(usd("5").negate(), usd("-5"));
    }

    #[test]
    fn overflow_is_an_error() {
        let huge = Money::new(Decimal::MAX, Currency::USD);
        assert_eq!(huge.checked_add(usd("1")), Err(Error::Overflow));
        assert_eq!(huge.multiply(Decimal::TWO), Err(Error::Overflow));
    }

    #[test]
    fn serialises_amounts_as_exact_text() {
        let json = serde_json::to_string(&usd("1234.50")).unwrap();
        assert_eq!(json, r#"{"amount":"1234.50","currency":"USD"}"#);
        assert_eq!(
            serde_json::from_str::<Money>(&json).unwrap(),
            usd("1234.50")
        );
    }

    proptest! {
        #[test]
        fn addition_is_exact(a in -1_000_000_000i64..1_000_000_000, b in -1_000_000_000i64..1_000_000_000) {
            let x = Money::new(Decimal::new(a, 2), Currency::USD);
            let y = Money::new(Decimal::new(b, 2), Currency::USD);
            prop_assert_eq!(x.checked_add(y).unwrap().checked_sub(y).unwrap(), x);
            prop_assert_eq!(x.checked_add(y).unwrap(), y.checked_add(x).unwrap());
        }
    }
}
