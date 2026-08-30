//! Integer money. Cents, never floats.
//!
//! `Money` wraps a **private** `i64` count of cents. The private field is the
//! guard: money cannot be conjured outside this module, only moved. The single
//! constructor `from_cents` is used at config ingestion and initial endowment
//! only; everything thereafter moves existing money.
//!
//! Overflow policy (D-07, operator half): the operator impls route through
//! `i64::checked_*` and panic in **every** build profile, including a default
//! release build with no `overflow-checks`. The named `Result`-returning API
//! (`checked_add`, `checked_sub`, `try_scale`) arrives with plan 01-03.
//!
//! Deliberately absent, and to stay absent: any conversion to or from a
//! floating-point type, floating-point multiplication, and a decimal
//! `Display`. A float must never reach money except through an explicit,
//! named rounding function. This module names no floating-point type at all,
//! which is the grep-able form of that rule.

/// An amount of money, in integer cents.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
    serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct Money(i64);

impl Money {
    /// Zero cents.
    pub const ZERO: Money = Money(0);

    /// The ONLY constructor. Config parsing and initial endowment only.
    pub const fn from_cents(cents: i64) -> Money {
        Money(cents)
    }

    /// The amount, in cents.
    pub const fn cents(self) -> i64 {
        self.0
    }
}

impl std::ops::Add for Money {
    type Output = Money;

    /// Panics on overflow in every profile — this is `checked_add`, not `+`,
    /// so it does not depend on `[profile.release] overflow-checks`.
    fn add(self, other: Money) -> Money {
        Money(
            self.0
                .checked_add(other.0)
                .expect("Money overflow on add"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Money, MoneyOverflow};

    // --- The operator half of D-07: panics in EVERY build profile ---------

    #[test]
    fn adding_zero_at_the_maximum_does_not_overflow() {
        let max = Money::from_cents(i64::MAX);
        assert_eq!(max + Money::ZERO, Money::from_cents(i64::MAX));
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn adding_one_cent_past_the_maximum_panics() {
        let _ = Money::from_cents(i64::MAX) + Money::from_cents(1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn subtracting_one_cent_below_the_minimum_panics() {
        let _ = Money::from_cents(i64::MIN) - Money::from_cents(1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn negating_the_minimum_panics() {
        let _ = -Money::from_cents(i64::MIN);
    }

    #[test]
    fn add_assign_and_sub_assign_route_through_the_checked_primitive() {
        let mut m = Money::from_cents(10);
        m += Money::from_cents(5);
        assert_eq!(m, Money::from_cents(15));
        m -= Money::from_cents(20);
        assert_eq!(m, Money::from_cents(-5));
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn add_assign_past_the_maximum_panics() {
        let mut m = Money::from_cents(i64::MAX);
        m += Money::from_cents(1);
    }

    // --- The named Result half of D-07: never panics ---------------------

    #[test]
    fn checked_add_at_the_maximum_returns_the_named_error() {
        assert_eq!(
            Money::from_cents(i64::MAX).checked_add(Money::from_cents(1)),
            Err(MoneyOverflow { lhs: i64::MAX, op: "+", rhs: 1 })
        );
    }

    #[test]
    fn checked_add_returns_ok_for_ordinary_amounts() {
        assert_eq!(
            Money::from_cents(5).checked_add(Money::from_cents(7)),
            Ok(Money::from_cents(12))
        );
    }

    #[test]
    fn checked_sub_returns_ok_and_a_named_error_at_the_minimum() {
        assert_eq!(
            Money::from_cents(7).checked_sub(Money::from_cents(5)),
            Ok(Money::from_cents(2))
        );
        assert_eq!(
            Money::from_cents(i64::MIN).checked_sub(Money::from_cents(1)),
            Err(MoneyOverflow { lhs: i64::MIN, op: "-", rhs: 1 })
        );
    }

    #[test]
    fn try_scale_truncates_toward_zero_and_reports_overflow() {
        assert_eq!(
            Money::from_cents(1_000).try_scale(3, 4),
            Ok(Money::from_cents(750))
        );
        // Truncation is toward zero on both signs, never toward minus infinity.
        assert_eq!(Money::from_cents(-1_000).try_scale(3, 4), Ok(Money::from_cents(-750)));
        assert_eq!(Money::from_cents(7).try_scale(1, 2), Ok(Money::from_cents(3)));
        assert!(Money::from_cents(i64::MAX).try_scale(2, 1).is_err());
        assert!(Money::from_cents(1_000).try_scale(1, 0).is_err());
    }

    // --- Sum routes through the checked Add, never a raw fold (D-08) ------

    #[test]
    fn sum_of_amounts_and_of_an_empty_iterator() {
        let two = [Money::from_cents(1), Money::from_cents(2)];
        assert_eq!(two.into_iter().sum::<Money>(), Money::from_cents(3));
        assert_eq!(two.iter().sum::<Money>(), Money::from_cents(3));

        let empty: [Money; 0] = [];
        assert_eq!(empty.into_iter().sum::<Money>(), Money::ZERO);
        assert_eq!(empty.iter().sum::<Money>(), Money::ZERO);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn a_sum_that_would_overflow_panics() {
        let _ = [Money::from_cents(i64::MAX), Money::from_cents(1)]
            .into_iter()
            .sum::<Money>();
    }

    // --- Exact integer equality and ordering ------------------------------

    #[test]
    fn equality_is_exact_integer_equality() {
        assert_eq!(Money::from_cents(7), Money::from_cents(7));
        assert_ne!(Money::from_cents(7), Money::from_cents(8));
    }

    #[test]
    fn ordering_follows_the_underlying_integer() {
        assert!(Money::from_cents(-1) < Money::ZERO);
        assert!(Money::ZERO < Money::from_cents(1));
        assert!(Money::from_cents(i64::MIN) < Money::from_cents(i64::MAX));
    }
}
