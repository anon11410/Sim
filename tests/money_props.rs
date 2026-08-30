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
    #[test]
    fn split_parts_sum_to_the_whole(amount in 1i64..1_000_000, n in 2u32..64) {
        let whole = Money::from_cents(amount);
        let parts = whole.split(n);
        prop_assert_eq!(parts.len(), n as usize);
        prop_assert_eq!(parts.into_iter().sum::<Money>(), whole);
    }

    /// The same property, restricted to the amounts that actually exercise the
    /// remainder path. A remainder-dropping implementation passes the property
    /// above on round numbers and fails here.
    #[test]
    fn split_parts_sum_to_the_whole_when_not_evenly_divisible(
        amount in 1i64..1_000_000,
        n in 2u32..64,
    ) {
        prop_assume!(amount % i64::from(n) != 0);
        let whole = Money::from_cents(amount);
        let parts = whole.split(n);
        prop_assert_eq!(parts.into_iter().sum::<Money>(), whole);
    }

    /// The remainder is distributed one cent at a time rather than dumped on a
    /// single recipient, so no part differs from another by more than a cent.
    #[test]
    fn split_part_spread_is_at_most_one_cent(amount in 1i64..1_000_000, n in 2u32..64) {
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
