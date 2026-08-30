//! The one seeded RNG, and the sub-stream facade that keeps draw sites
//! independent of one another (CORE-03, CORE-04, CORE-05; D-01 … D-05).
//!
//! One master 32-byte key. Every draw site opens a short-lived [`Stream`]
//! addressed by a `u64` nonce that bit-packs `(tick, agent, purpose)` as
//! `tick:24 | agent:24 | purpose:16`, high bits to low. The packing is
//! **bijective**, so distinct tuples yield distinct nonces by arithmetic
//! rather than by a collision-resistance argument. `ChaCha8Rng::set_stream`
//! selects the keystream and resets `word_pos` to 0, so a sub-stream's output
//! does not depend on how many draws any other sub-stream took. That is
//! CORE-04: an added draw in one market provably cannot perturb another.
//!
//! **Discriminants are append-only.** [`Purpose`] is `#[repr(u16)]` with
//! hand-assigned values, spaced in tens per subsystem so a later phase can
//! insert without renumbering. A discriminant is **never** renumbered and a
//! retired number is **never** reused: renumbering silently re-keys every
//! sub-stream after it, changing the trajectory of every committed run, golden
//! log and snapshot (D-02).
//!
//! Hazard (D-04): re-entering an already-issued key **replays** it. A key must
//! be opened at most once per run; a site needing two independent sequences for
//! the same `(tick, agent)` adds a [`Purpose`] variant — it does not visit the
//! key twice. Debug builds carry an issued-key set that makes a double-open
//! loud.
//!
//! `agent` carries `FirmId.slot`, never `gen` (D-03): a respawned firm must not
//! inherit its predecessor's keystream position, and `gen` must not widen the
//! key.
//!
//! This module is the **only** construction site of a generator in the crate.
//! Every other module receives a [`Stream`] and never a generator, so there is
//! no ambient sequence to fall back to. The non-portable standard generator and
//! the system-entropy generator are not referenced here and do not resolve under
//! the crate's feature set — that compile failure is CORE-03 clause (a)'s
//! enforcement, not a review note.

use rand::rngs::ChaCha8Rng;
use rand::{Rng, SeedableRng};

/// Width of the `tick` field of the sub-stream key, in bits.
///
/// Fixed at 24 by D-02 and **not** a value to tune narrower: the surplus over
/// the realistic need (12 bits for a 3,650-tick decade) is deliberate headroom
/// so that lengthening a run never forces a re-key.
pub const TICK_BITS: u32 = 24;

/// Width of the `agent` field of the sub-stream key, in bits. Fixed at 24 by
/// D-02; deliberate headroom over the realistic need of 8 bits.
pub const AGENT_BITS: u32 = 24;

/// Width of the `purpose` field of the sub-stream key, in bits. Fixed at 16 by
/// D-02; deliberate headroom over the realistic need of ~5 bits, because
/// `purpose` is the field most likely to grow as later phases add draw sites.
pub const PURPOSE_BITS: u32 = 16;

/// What a sub-stream is *for*.
///
/// `#[repr(u16)]` with hand-assigned discriminants, gapped in tens per
/// subsystem. **Append-only: never renumber, never reuse a retired number.**
/// See the module docs for why.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Purpose {
    /// The Phase 1 spine probe. Not a behaviour draw site.
    TracerProbe = 1,
    /// Household activation order for the tick (Phase 3).
    ActivationOrderHouseholds = 10,
    /// Firm activation order for the tick (Phase 3).
    ActivationOrderFirms = 11,
    /// Which firms a household visits in the labour market.
    LabourSample = 20,
    /// Whether an already-employed household searches this tick.
    EmployedSearchCoin = 21,
    /// Which firms a household visits in the goods market.
    GoodsSample = 30,
    /// Whether a household revises its preferred supplier.
    SupplierRevision = 31,
    /// Whether a firm leaves its price unchanged this tick.
    PriceInactionCoin = 40,
    /// The size of a firm's price revision.
    PriceStep = 41,
    /// The size of a firm's wage revision.
    WageStep = 42,
    /// A firm's initial planning-cycle offset, drawn once at setup.
    PlanningOffsetInit = 43,
    /// Which household takes ownership of a respawned firm.
    BankruptcyOwnerDraw = 50,
}

/// Every [`Purpose`] variant, for tests that must sweep the whole key space.
///
/// Extend this whenever a variant is appended; the injectivity sweep is only as
/// complete as this array.
pub const ALL_PURPOSES: [Purpose; 12] = [
    Purpose::TracerProbe,
    Purpose::ActivationOrderHouseholds,
    Purpose::ActivationOrderFirms,
    Purpose::LabourSample,
    Purpose::EmployedSearchCoin,
    Purpose::GoodsSample,
    Purpose::SupplierRevision,
    Purpose::PriceInactionCoin,
    Purpose::PriceStep,
    Purpose::WageStep,
    Purpose::PlanningOffsetInit,
    Purpose::BankruptcyOwnerDraw,
];

/// Pack `(tick, agent, purpose)` into the `u64` sub-stream nonce.
///
/// Layout is `tick:24 | agent:24 | purpose:16`, high bits to low. The field
/// widths are fixed by D-02 and are deliberate headroom, **not** a value to
/// tune narrower — narrowing or widening any field re-keys every run ever
/// committed.
///
/// # Panics
///
/// If `tick` or `agent` does not fit its field. This is a real `assert!` and not
/// a debug-only one on purpose: a silent field overrun would alias two agents
/// onto one keystream and corrupt a run without failing anything (T-1-12).
pub fn pack_stream_key(tick: u32, agent: u32, p: Purpose) -> u64 {
    assert!(
        tick < (1u32 << TICK_BITS),
        "tick {tick} does not fit the {TICK_BITS}-bit tick field of the sub-stream key"
    );
    assert!(
        agent < (1u32 << AGENT_BITS),
        "agent {agent} does not fit the {AGENT_BITS}-bit agent field of the sub-stream key"
    );

    ((tick as u64) << (AGENT_BITS + PURPOSE_BITS))
        | ((agent as u64) << PURPOSE_BITS)
        | p as u16 as u64
}

/// The master seed, and the factory for every sub-stream in a run.
pub struct Rngs {
    master: [u8; 32],
    /// Keys already handed out, so a re-entry fails loudly (D-04, T-1-13).
    ///
    /// Ordered, not hashed: the hashed collections are banned crate-wide by
    /// CORE-07 and their iteration order is nondeterministic. Debug-only — a
    /// decade-long run opens millions of sub-streams and the set would grow
    /// without bound in a release run.
    #[cfg(debug_assertions)]
    issued: std::cell::RefCell<std::collections::BTreeSet<u64>>,
}

impl Rngs {
    /// Build the master key from the effective run seed.
    pub fn new(master_seed: u64) -> Self {
        let mut master = [0u8; 32];
        master[..8].copy_from_slice(&master_seed.to_le_bytes());
        Self {
            master,
            #[cfg(debug_assertions)]
            issued: std::cell::RefCell::new(std::collections::BTreeSet::new()),
        }
    }

    /// Open the sub-stream named by `(tick, agent, purpose)`.
    ///
    /// `agent` carries the firm **slot** and never the generation (D-03): two
    /// firms occupying one slot in different generations never coexist at the
    /// same tick, so `(tick, slot, purpose)` stays unique, while including the
    /// generation would waste key bits — and letting a respawned firm inherit
    /// the previous occupant's keystream position would be a defect. Households
    /// pass their index directly.
    ///
    /// # Panics
    ///
    /// In a debug build, if this key has already been opened on this `Rngs`.
    /// Re-entry replays the same values, silently correlating two decisions that
    /// should be independent. A site needing two independent sequences for the
    /// same `(tick, agent)` adds a [`Purpose`] variant; it does not visit the
    /// key twice.
    pub fn stream(&self, tick: u32, agent: u32, p: Purpose) -> Stream {
        let key = pack_stream_key(tick, agent, p);

        #[cfg(debug_assertions)]
        {
            let fresh = self.issued.borrow_mut().insert(key);
            assert!(
                fresh,
                "sub-stream key {key:#018x} re-entered: (tick {tick}, agent {agent}, \
                 purpose {p:?}). Re-entry replays the same values; add a Purpose \
                 variant instead of visiting the key twice (D-04)."
            );
        }

        let mut generator = ChaCha8Rng::from_seed(self.master);
        generator.set_stream(key); // also resets word_pos to 0
        Stream(generator, 0)
    }
}

/// One open sub-stream, plus the count of draws taken from it.
///
/// The draw count is the divergence localiser: two runs that diverge show it
/// first as a differing per-tick draw-count series. Every sampler below states
/// its exact draw count, and every one of them is fixed-draw — no rejection
/// loop and no unbounded loop anywhere on the behaviour path (CORE-05, D-05).
///
/// `rand`'s own range sampler, its uniform-distribution sampler and its index
/// sampler are deliberately never called, and their identifiers are kept out of
/// this file so that a grep for a call site cannot return a false positive.
/// Research verified from the vendored 0.10.2 source that the first can consume
/// a second word, the second is an unbounded loop, and the third dispatches
/// between three algorithms on `f32` thresholds the crate documents as
/// performance tuning rather than contract. See 01-RESEARCH.md Pattern 2.
pub struct Stream(ChaCha8Rng, u32);

impl Stream {
    /// A uniform value in `0..n`. **Exactly one** 64-bit draw.
    ///
    /// Multiply-high, no rejection loop: bias is at most `n / 2^64`. A variable
    /// draw count would defeat the isolation guarantee from the inside (D-05).
    pub fn below(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0, "below(0) has no valid result");
        self.1 += 1;
        ((self.0.next_u64() as u128 * n as u128) >> 64) as u64
    }

    /// A coin at probability `p_ppm` parts per million. **Exactly one** draw.
    ///
    /// Probabilities enter the model as parts-per-million integers, never as
    /// floats, which keeps every threshold parameter in the integer domain.
    pub fn coin_ppm(&mut self, p_ppm: u32) -> bool {
        self.below(1_000_000) < p_ppm as u64
    }

    /// `k` distinct elements of `pool`, by partial Fisher-Yates.
    /// **Exactly `k`** draws, always — never fewer, never more.
    ///
    /// Chosen elements are swapped into the low positions of `pool`, so `pool`
    /// is permuted in place and its first `k` entries are the returned sample.
    ///
    /// # Panics
    ///
    /// If `k > pool.len()`.
    pub fn sample_k(&mut self, pool: &mut [u32], k: usize) -> Vec<u32> {
        let n = pool.len();
        assert!(
            k <= n,
            "cannot sample {k} distinct elements from a pool of {n}"
        );
        let mut out = Vec::with_capacity(k);
        for i in 0..k {
            let j = i + self.below((n - i) as u64) as usize;
            pool.swap(i, j);
            out.push(pool[i]);
        }
        out
    }

    /// A uniform permutation of `pool`, by full Fisher-Yates.
    /// **Exactly `pool.len() - 1`** draws (and none for an empty or 1-element
    /// slice). For Phase 3's activation-order shuffle.
    pub fn shuffle_in_place(&mut self, pool: &mut [u32]) {
        let n = pool.len();
        for i in 0..n.saturating_sub(1) {
            let j = i + self.below((n - i) as u64) as usize;
            pool.swap(i, j);
        }
    }

    /// How many draws this sub-stream has served.
    pub fn draws(&self) -> u32 {
        self.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // ---- pack_stream_key: the arithmetic ----------------------------------

    #[test]
    fn pack_at_the_origin_is_the_purpose_discriminant() {
        assert_eq!(
            pack_stream_key(0, 0, Purpose::TracerProbe),
            Purpose::TracerProbe as u16 as u64
        );
        for p in ALL_PURPOSES {
            assert_eq!(pack_stream_key(0, 0, p), p as u16 as u64);
        }
    }

    #[test]
    fn one_tick_moves_the_key_by_the_tick_field_shift() {
        for p in ALL_PURPOSES {
            assert_eq!(pack_stream_key(1, 0, p), (1u64 << 40) | p as u16 as u64);
        }
    }

    #[test]
    fn one_agent_moves_the_key_by_the_purpose_field_width() {
        for p in ALL_PURPOSES {
            assert_eq!(pack_stream_key(0, 1, p), (1u64 << 16) | p as u16 as u64);
        }
    }

    #[test]
    fn pack_stream_key_is_injective_over_a_swept_grid() {
        let mut seen = BTreeSet::new();
        let mut swept = 0usize;
        for tick in 0..40u32 {
            for agent in 0..40u32 {
                for p in ALL_PURPOSES {
                    swept += 1;
                    assert!(
                        seen.insert(pack_stream_key(tick, agent, p)),
                        "key collision at ({tick}, {agent}, {p:?})"
                    );
                }
            }
        }
        assert_eq!(swept, 40 * 40 * ALL_PURPOSES.len());
        assert_eq!(seen.len(), swept);
    }

    // ---- pack_stream_key: the field boundary ------------------------------

    #[test]
    fn the_maximum_tick_and_agent_pack_and_stay_distinct() {
        let max = (1u32 << TICK_BITS) - 1;
        let p = Purpose::GoodsSample;
        let top = pack_stream_key(max, max, p);
        let one_below = pack_stream_key(max - 1, max, p);
        assert_ne!(top, one_below);
        assert_eq!(
            top,
            ((max as u64) << (AGENT_BITS + PURPOSE_BITS))
                | ((max as u64) << PURPOSE_BITS)
                | p as u16 as u64
        );
    }

    #[test]
    #[should_panic(expected = "tick")]
    fn one_step_past_the_tick_field_panics() {
        pack_stream_key(1 << TICK_BITS, 0, Purpose::GoodsSample);
    }

    #[test]
    #[should_panic(expected = "agent")]
    fn one_step_past_the_agent_field_panics() {
        pack_stream_key(0, 1 << AGENT_BITS, Purpose::GoodsSample);
    }

    // ---- Rngs / Stream ----------------------------------------------------

    #[test]
    fn seed_zero_is_a_legal_master_seed() {
        let r = Rngs::new(0);
        let v = r.stream(0, 0, Purpose::GoodsSample).below(1_000_000);
        assert!(v < 1_000_000);
    }

    #[test]
    fn an_unopened_stream_reports_zero_draws() {
        let r = Rngs::new(20260830);
        let s = r.stream(0, 0, Purpose::GoodsSample);
        assert_eq!(s.draws(), 0);
    }

    #[test]
    fn the_same_master_seed_gives_the_same_first_draw() {
        let (t, a, p) = (10u32, 7u32, Purpose::LabourSample);
        let x = Rngs::new(20260830).stream(t, a, p).below(u64::MAX);
        let y = Rngs::new(20260830).stream(t, a, p).below(u64::MAX);
        assert_eq!(x, y);
    }

    #[test]
    fn an_adjacent_master_seed_gives_a_different_first_draw() {
        let (t, a, p) = (10u32, 7u32, Purpose::LabourSample);
        let x = Rngs::new(20260830).stream(t, a, p).below(u64::MAX);
        let y = Rngs::new(20260831).stream(t, a, p).below(u64::MAX);
        assert_ne!(x, y);
    }

    // The re-entry guard (D-04) is a debug-build construct by design: it costs
    // a BTreeSet insert per sub-stream, and a 10-year run opens millions.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "sub-stream key")]
    fn reopening_a_key_panics_in_a_debug_build() {
        let r = Rngs::new(20260830);
        let _first = r.stream(10, 7, Purpose::GoodsSample);
        let _second = r.stream(10, 7, Purpose::GoodsSample);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_different_purpose_at_the_same_tick_and_agent_is_not_a_re_entry() {
        let r = Rngs::new(20260830);
        let _a = r.stream(10, 7, Purpose::GoodsSample);
        let _b = r.stream(10, 7, Purpose::LabourSample);
    }

    // ---- Purpose discriminants -------------------------------------------

    #[test]
    fn every_purpose_discriminant_is_distinct_and_non_zero() {
        let mut seen = BTreeSet::new();
        for p in ALL_PURPOSES {
            let d = p as u16;
            assert_ne!(d, 0, "{p:?} must not use discriminant 0");
            assert!(seen.insert(d), "duplicate discriminant {d} at {p:?}");
        }
        assert_eq!(seen.len(), ALL_PURPOSES.len());
    }
}
