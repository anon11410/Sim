//! The one seeded RNG, and the sub-stream facade that keeps draw sites
//! independent of one another (CORE-04, D-01 … D-05).
//!
//! One master 32-byte key. Every draw site opens a short-lived [`Stream`]
//! addressed by a `u64` nonce that bit-packs `(tick, agent, purpose)` as
//! `tick:24 | agent:24 | purpose:16`, high bits to low. The packing is
//! **bijective**, so distinct tuples yield distinct nonces by arithmetic
//! rather than by a collision-resistance argument. `ChaCha8Rng::set_stream`
//! selects the keystream and resets `word_pos` to 0, so a sub-stream's output
//! does not depend on how many draws any other sub-stream took.
//!
//! Hazard (D-04): re-entering an already-issued key **replays** it. A key must
//! be opened at most once per run; a site needing two independent sequences for
//! the same `(tick, agent)` uses two [`Purpose`] variants, not two visits. Plan
//! 01-04 adds the debug-build issued-key guard that makes a double-open loud.
//!
//! `agent` carries `FirmId.slot`, never `gen` (D-03): a respawned firm must not
//! inherit its predecessor's keystream position, and `gen` must not widen the
//! key.

use rand::rngs::ChaCha8Rng;
use rand::{Rng, SeedableRng};

/// What a sub-stream is *for*.
///
/// `#[repr(u16)]` with hand-assigned discriminants. Discriminants are
/// **append-only and never renumbered** (D-02): renumbering silently re-keys
/// every historical run. Plan 01-04 appends the real draw sites; this is the
/// tracer's single probe.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Purpose {
    /// The Phase 1 spine probe. Not a behaviour draw site.
    TracerProbe = 1,
}

/// The master seed, and the factory for every sub-stream in a run.
pub struct Rngs {
    master: [u8; 32],
}

impl Rngs {
    /// Build the master key from the effective run seed.
    pub fn new(master_seed: u64) -> Self {
        let mut master = [0u8; 32];
        master[..8].copy_from_slice(&master_seed.to_le_bytes());
        Self { master }
    }

    /// Open the sub-stream named by `(tick, agent, purpose)`.
    ///
    /// Key layout is `tick:24 | agent:24 | purpose:16` (D-01/D-02). The bit
    /// allocation is fixed and deliberately far wider than the realistic need,
    /// so adding a purpose or raising the tick count never forces a re-key.
    pub fn stream(&self, tick: u32, agent: u32, p: Purpose) -> Stream {
        assert!(tick < (1u32 << 24), "tick {tick} exceeds the 24-bit stream-key field");
        assert!(agent < (1u32 << 24), "agent {agent} exceeds the 24-bit stream-key field");

        let key = ((tick as u64) << 40) | ((agent as u64) << 16) | (p as u16 as u64);

        let mut generator = ChaCha8Rng::from_seed(self.master);
        generator.set_stream(key); // also resets word_pos to 0
        Stream(generator, 0)
    }
}

/// One open sub-stream, plus the count of draws taken from it.
///
/// The draw count is the divergence localiser: two runs that diverge show it
/// first as a differing per-tick draw-count series.
pub struct Stream(ChaCha8Rng, u32);

impl Stream {
    /// A uniform value in `0..n`, using **exactly one** 64-bit draw.
    ///
    /// Multiply-high, no rejection loop: bias is at most `n / 2^64`. A variable
    /// draw count would defeat the isolation guarantee from the inside (D-05).
    pub fn below(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0, "below(0) has no valid result");
        self.1 += 1;
        ((self.0.next_u64() as u128 * n as u128) >> 64) as u64
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
