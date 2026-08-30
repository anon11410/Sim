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
