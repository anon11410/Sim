//! Integer money. Cents, never floats.
//!
//! `Money` wraps a **private** `i64` count of cents. The private field is the
//! guard: money cannot be conjured outside this module, only moved. The single
//! constructor `from_cents` is used at config ingestion and initial endowment
//! only; everything thereafter moves existing money.
//!
//! Overflow policy is a **split API** (D-07), and both halves ship together:
//!
//! * **The operator impls** (`Add`, `Sub`, `AddAssign`, `SubAssign`, `Neg`,
//!   `Sum`) route through `i64::checked_*` and `.expect(...)`. They panic in
//!   **every** build profile, including a default release build with no
//!   `overflow-checks`, because the check is in the code and not in the
//!   profile. Overflow on an operator is a program bug: money here is a fixed
//!   pile that cannot approach `i64::MAX`, so aborting is the right answer.
//! * **The named API** (`checked_add`, `checked_sub`, `try_scale`) returns
//!   `Result<Money, MoneyOverflow>` and never panics. It exists for the one
//!   place overflow is a legitimate runtime condition rather than a bug:
//!   `src/config.rs` ingesting an operator-supplied `total_money_cents`, which
//!   should surface a named `ConfigError` instead of aborting the process.
//!
//! Neither half may be deleted in favour of the other.
//!
//! Deliberately absent, and to stay absent: any conversion to or from a
//! floating-point type, floating-point multiplication, and a decimal
//! `Display`. A float must never reach money except through an explicit,
//! named rounding function, which `src/numeric.rs` owns. This module names no
//! floating-point type at all, which is the grep-able form of that rule.

use thiserror::Error;

/// A checked money operation that could not be represented in `i64` cents.
///
/// Carries the operands and the operator so a `ConfigError` can quote the
/// arithmetic that failed rather than a bare "overflow".
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("money overflow: {lhs} {op} {rhs}")]
pub struct MoneyOverflow {
    pub lhs: i64,
    pub op: &'static str,
    pub rhs: i64,
}

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

    // --- The named, non-panicking API (D-07, Result half) ----------------

    /// `self + other`, reporting overflow as a value instead of a panic.
    pub fn checked_add(self, other: Money) -> Result<Money, MoneyOverflow> {
        self.0
            .checked_add(other.0)
            .map(Money)
            .ok_or(MoneyOverflow { lhs: self.0, op: "+", rhs: other.0 })
    }

    /// `self - other`, reporting overflow as a value instead of a panic.
    pub fn checked_sub(self, other: Money) -> Result<Money, MoneyOverflow> {
        self.0
            .checked_sub(other.0)
            .map(Money)
            .ok_or(MoneyOverflow { lhs: self.0, op: "-", rhs: other.0 })
    }

    /// `self * num / den`, multiplying first and truncating toward zero.
    ///
    /// Multiplying before dividing keeps the full precision of the ratio in
    /// the integer domain — there is no intermediate rounding and no float.
    /// Returns `Err` on multiplication overflow, on a zero denominator, and
    /// on the one division that cannot be represented.
    pub fn try_scale(self, num: i64, den: i64) -> Result<Money, MoneyOverflow> {
        let product = self
            .0
            .checked_mul(num)
            .ok_or(MoneyOverflow { lhs: self.0, op: "*", rhs: num })?;

        product
            .checked_div(den)
            .map(Money)
            .ok_or(MoneyOverflow { lhs: product, op: "/", rhs: den })
    }
}

// --- The operator API: panics in EVERY build profile (D-07) --------------

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

impl std::ops::Sub for Money {
    type Output = Money;

    /// Panics on overflow in every profile.
    fn sub(self, other: Money) -> Money {
        Money(
            self.0
                .checked_sub(other.0)
                .expect("Money overflow on sub"),
        )
    }
}

impl std::ops::Neg for Money {
    type Output = Money;

    /// Panics on overflow in every profile — negating the minimum has no
    /// representable answer.
    fn neg(self) -> Money {
        Money(self.0.checked_neg().expect("Money overflow on neg"))
    }
}

impl std::ops::AddAssign for Money {
    /// Delegates to the checked `Add`, so it panics identically.
    fn add_assign(&mut self, other: Money) {
        *self = *self + other;
    }
}

impl std::ops::SubAssign for Money {
    /// Delegates to the checked `Sub`, so it panics identically.
    fn sub_assign(&mut self, other: Money) {
        *self = *self - other;
    }
}

impl std::iter::Sum for Money {
    /// Folds through the checked `Add` — never over a raw integer
    /// accumulator, which is the one path that would wrap silently (D-08).
    /// An empty iterator sums to `Money::ZERO`.
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |acc, item| acc + item)
    }
}

impl<'a> std::iter::Sum<&'a Money> for Money {
    /// Folds through the checked `Add`, exactly as the owned impl does.
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |acc, item| acc + *item)
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
