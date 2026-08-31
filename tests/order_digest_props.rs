//! Distinct activation permutations map to distinct digests (TICK-10).
//!
//! **This is deliberately the only property test in Phase 3, and the reasoning
//! is recorded here rather than left to be inferred from an absence.** A
//! generated input domain earns its cost where the input domain is where the
//! risk lives. In this phase it is not: the tick pipeline is a fixed table of
//! nine entries, the tick row is a fixed shape of nine integer columns, and the
//! run log is a fixed file layout. Generating inputs for any of those would
//! mean generating economies the model never produces, and a counterexample
//! drawn from one would be a false alarm rather than a finding. The phase's
//! real risk lives in the bytes that reach disk, and those are covered by the
//! in-module end-to-end tests, the binary-level tests, and two mutation-proved
//! source guards.
//!
//! The digest is the exception. It is a pure function of an input domain — the
//! space of activation permutations — that is far too large to enumerate, and
//! the whole of TICK-10 rests on it separating one permutation from another.
//! That is exactly the shape a property test is for.
//!
//! The generated values are genuine **permutations of an index range**, built
//! by shuffling `0..n` rather than by drawing an arbitrary integer vector. An
//! arbitrary vector is an input `shuffle_activation` cannot produce, so a
//! counterexample from one would say nothing about the model. Shrinking
//! preserves permutation-ness for the same reason: this library's shrinking is
//! integrated with its generators, which is why Phase 1 chose it.
//!
//! `.proptest-regressions/order_digest_props.txt` is committed, so a
//! counterexample found once is replayed on every future run.

use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

use sim::phases::order_digest;

/// The shipped population sizes. The generated space is the space the model
/// actually draws from, not a convenient smaller one.
const HOUSEHOLDS: u32 = 200;
const FIRMS: u32 = 20;

/// A genuine permutation of `0..n`.
fn permutation(n: u32) -> impl Strategy<Value = Vec<u32>> {
    Just((0..n).collect::<Vec<u32>>()).prop_shuffle()
}

/// Swap the last two entries. On a permutation the two are always distinct, so
/// this always produces a different sequence — a tail-only change, which is
/// precisely the divergence a "first activated agent" column would miss.
fn swap_the_tail(order: &[u32]) -> Vec<u32> {
    let mut changed = order.to_vec();
    let last = changed.len() - 1;
    changed.swap(last - 1, last);
    changed
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        // Explicit, so counterexamples land at the committed repository path
        // rather than wherever the default source-parallel rule resolves to.
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(
            ".proptest-regressions/order_digest_props.txt",
        ))),
        ..ProptestConfig::default()
    })]

    /// The digest is a function of the permutation and of nothing else.
    ///
    /// The negative half of every other property here: without it, a digest
    /// that returned a fresh value on each call would satisfy all three
    /// "different orders differ" claims and be useless in a log.
    #[test]
    fn one_order_always_gives_one_digest(
        households in permutation(HOUSEHOLDS),
        firms in permutation(FIRMS),
    ) {
        prop_assert_eq!(
            order_digest(&households, &firms),
            order_digest(&households, &firms),
        );
    }

    /// Two orders differ if and only if their digests do.
    ///
    /// Stated as an equivalence rather than as "different orders give different
    /// digests", because two independently shuffled pairs can legitimately come
    /// out equal, and a test that asserted inequality unconditionally would be
    /// asserting something false about its own generator.
    #[test]
    fn distinct_orders_give_distinct_digests(
        households_a in permutation(HOUSEHOLDS),
        firms_a in permutation(FIRMS),
        households_b in permutation(HOUSEHOLDS),
        firms_b in permutation(FIRMS),
    ) {
        let a = order_digest(&households_a, &firms_a);
        let b = order_digest(&households_b, &firms_b);

        if households_a == households_b && firms_a == firms_b {
            prop_assert_eq!(a, b, "one order produced two digests");
        } else {
            prop_assert_ne!(a, b, "two different orders produced one digest");
        }
    }

    /// A change confined to the TAIL of the household order changes the digest.
    ///
    /// This is the case the design exists for. A column holding the first
    /// activated household would be identical across this change, and a run
    /// that diverged only in who activated last would look byte-identical.
    #[test]
    fn a_tail_only_change_in_the_household_order_changes_the_digest(
        households in permutation(HOUSEHOLDS),
        firms in permutation(FIRMS),
    ) {
        let changed = swap_the_tail(&households);
        prop_assert_ne!(households.clone(), changed.clone());
        prop_assert_ne!(
            order_digest(&households, &firms),
            order_digest(&changed, &firms),
        );
    }

    /// A change confined to the FIRM order alone changes the digest.
    ///
    /// This is what the separator between the two sequences guarantees: without
    /// it, the two orders would be one concatenated sequence and a change that
    /// merely moved the boundary between them could collide.
    #[test]
    fn a_change_in_the_firm_order_alone_changes_the_digest(
        households in permutation(HOUSEHOLDS),
        firms in permutation(FIRMS),
    ) {
        let changed = swap_the_tail(&firms);
        prop_assert_ne!(firms.clone(), changed.clone());
        prop_assert_ne!(
            order_digest(&households, &firms),
            order_digest(&households, &changed),
        );
    }
}
