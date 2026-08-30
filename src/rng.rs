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
