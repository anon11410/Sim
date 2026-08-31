//! Property tests for the money invariants that a unit test cannot pin down.
//!
//! This file reaches `Money` through the library surface — `use sim::money::…`
//! — rather than through a private path, which is also part of CORE-08's proof
//! that integration tests under `tests/` can reach all code.
//!
//! The load-bearing property is the second one. ROADMAP criterion 1 is explicit
//! that the strategy must generate amounts that do **not** divide evenly:
//! without that case, a `vec![a / n; n]` implementation which destroys the
//! remainder passes on round numbers and the cent leakage only surfaces
//! thousands of ticks later as unattributable drift.

use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

use sim::money::{Money, MoneyOverflow};

/// Any representable amount, with the three edges drawn deliberately often.
///
/// The strategies here all used to read `1i64..1_000_000` — a strictly positive
/// amount below one million — while the claimed invariant (this module's own
/// header, and `CLAUDE.md` §7) is "the parts always sum exactly back to the
/// whole, **for all** `(amount, n)`". The untested region was zero, the
/// negatives and everything near the `i64` boundaries, and that is exactly
/// where CR-01 lived: `Money::from_cents(i64::MAX).split(1)` panicked.
///
/// A uniform `any::<i64>()` would reach the boundaries with probability
/// effectively zero over 512 cases, so the three edges are drawn explicitly
/// rather than hoped for.
fn any_amount() -> impl Strategy<Value = i64> {
    prop_oneof![
        2 => Just(i64::MAX),
        2 => Just(i64::MIN),
        2 => Just(0i64),
        14 => any::<i64>(),
    ]
}

/// Any recipient count in `1..64`, with the single-recipient case drawn often.
///
/// `n` used to start at 2. One is not a degenerate case to skip: it is a real
/// dividend paid to a sole owner, and it is where CR-01 aborted. Weighting it
/// is deliberate — the defect needs `amount == i64::MAX` AND a zero remainder
/// AT THE SAME TIME, and a uniform `1..64` reaches that pair about once per six
/// hundred cases, which is indistinguishable from not testing it at all.
fn any_part_count() -> impl Strategy<Value = u32> {
    prop_oneof![
        3 => Just(1u32),
        1 => Just(2u32),
        6 => 1u32..64,
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Explicit, so the run time is a property of this file rather than of
        // whatever environment happens to invoke it.
        cases: 512,
        // Explicit, so counterexamples land at the committed repository path
        // rather than wherever the default source-parallel rule resolves to.
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(
            ".proptest-regressions/money_props.txt",
        ))),
        ..ProptestConfig::default()
    })]

    /// No cent is created or destroyed by a split, for any amount and any
    /// recipient count.
    ///
    /// `n` starts at 1, not 2. The single-recipient case is not a degenerate
    /// one to skip — it is the case CR-01 aborted on.
    #[test]
    fn split_parts_sum_to_the_whole(amount in any_amount(), n in any_part_count()) {
        let whole = Money::from_cents(amount);
        let parts = whole.split(n);
        prop_assert_eq!(parts.len(), n as usize);
        // `Sum` folds through the checked `Add`, so this also asserts that no
        // intermediate partial sum overflows on the way to the total.
        prop_assert_eq!(parts.into_iter().sum::<Money>(), whole);
    }

    /// The same property, restricted to the amounts that actually exercise the
    /// remainder path. A remainder-dropping implementation passes the property
    /// above on round numbers and fails here.
    #[test]
    fn split_parts_sum_to_the_whole_when_not_evenly_divisible(
        amount in any_amount(),
        n in 2u32..64,
    ) {
        prop_assume!(amount % i64::from(n) != 0);
        let whole = Money::from_cents(amount);
        let parts = whole.split(n);
        prop_assert_eq!(parts.into_iter().sum::<Money>(), whole);
    }

    /// The remainder is distributed one cent at a time rather than dumped on a
    /// single recipient, so no part differs from another by more than a cent.
    ///
    /// Holds on both signs: a negative amount distributes a negative extra
    /// cent, so the bumped parts sit one cent BELOW the base rather than above
    /// it, and the spread is still exactly zero or one.
    #[test]
    fn split_part_spread_is_at_most_one_cent(amount in any_amount(), n in any_part_count()) {
        let parts: Vec<i64> = Money::from_cents(amount)
            .split(n)
            .into_iter()
            .map(Money::cents)
            .collect();
        let largest = *parts.iter().max().expect("split never returns no parts");
        let smallest = *parts.iter().min().expect("split never returns no parts");
        prop_assert!(
            largest - smallest == 0 || largest - smallest == 1,
            "spread was {} for {} split {} ways",
            largest - smallest,
            amount,
            n
        );
    }

    /// Subtraction and addition are exact inverses — no rounding, no drift.
    /// The range is narrow enough that no intermediate value can overflow, so
    /// the property tests the arithmetic and not the panic.
    #[test]
    fn add_then_subtract_round_trips(
        a in -1_000_000_000i64..1_000_000_000,
        b in -1_000_000_000i64..1_000_000_000,
    ) {
        let ma = Money::from_cents(a);
        let mb = Money::from_cents(b);
        prop_assert_eq!(ma - mb + mb, ma);

        // The named Result API agrees with the operator on every value where
        // the operator does not panic.
        let checked: Result<Money, MoneyOverflow> = ma.checked_sub(mb);
        prop_assert_eq!(checked, Ok(ma - mb));
    }
}
