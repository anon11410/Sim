//! CORE-03/04/05 at the library surface: the sub-stream facade reached through
//! `use sim::rng::…`, exactly as every later phase will reach it.
//!
//! The centrepiece is `extra_draws_in_one_purpose_cannot_perturb_another`. It is
//! CORE-04's whole point observed rather than asserted, and it goes red the
//! instant a future phase reintroduces a shared sequential draw source.

use sim::rng::{ALL_PURPOSES, Purpose, Rngs, Stream, pack_stream_key};
use std::collections::BTreeSet;

const SEED: u64 = 20260830;

/// Take `n` draws from an open sub-stream. Exists so `Stream` is named at this
/// boundary: later phases receive a `Stream` and never a generator.
fn take(s: &mut Stream, n: usize) -> Vec<u64> {
    (0..n).map(|_| s.below(u64::MAX)).collect()
}

// ---- CORE-03: one master seed, and it is actually wired in ----------------

#[test]
fn same_master_seed_identical_streams() {
    let a = Rngs::new(SEED);
    let b = Rngs::new(SEED);
    let xa: Vec<u64> = (0..64)
        .map(|i| a.stream(i, 0, Purpose::PriceStep).below(u64::MAX))
        .collect();
    let xb: Vec<u64> = (0..64)
        .map(|i| b.stream(i, 0, Purpose::PriceStep).below(u64::MAX))
        .collect();
    assert_eq!(
        xa, xb,
        "the same master seed must reproduce the run exactly"
    );
}

#[test]
fn different_master_seed_differs() {
    // The counter-check: without it an accidentally-constant generator passes
    // `same_master_seed_identical_streams` trivially.
    let a = Rngs::new(SEED);
    let b = Rngs::new(SEED + 1);
    assert_ne!(
        a.stream(0, 0, Purpose::PriceStep).below(u64::MAX),
        b.stream(0, 0, Purpose::PriceStep).below(u64::MAX),
        "adjacent master seeds must produce different streams"
    );
}

// ---- CORE-04: sub-stream isolation ---------------------------------------

#[test]
fn extra_draws_in_one_purpose_cannot_perturb_another() {
    // A fresh `Rngs` per arm, so the debug re-entry guard is not tripped by the
    // deliberate second visit to the goods key.
    let baseline: Vec<u64> = {
        let r = Rngs::new(SEED);
        let mut s = r.stream(10, 7, Purpose::GoodsSample);
        (0..4).map(|_| s.below(1_000_000)).collect()
    };

    let after: Vec<u64> = {
        let r = Rngs::new(SEED);
        // Simulate a code change that adds three draws to the labour market:
        // seven where the baseline arm took none.
        {
            let mut labour = r.stream(10, 7, Purpose::LabourSample);
            for _ in 0..7 {
                labour.below(1_000_000);
            }
            assert_eq!(labour.draws(), 7);
        }
        let mut s = r.stream(10, 7, Purpose::GoodsSample);
        (0..4).map(|_| s.below(1_000_000)).collect()
    };

    assert_eq!(
        baseline, after,
        "an added draw in one purpose perturbed another — CORE-04 is broken"
    );
}

#[test]
fn extra_draws_in_one_purpose_cannot_perturb_another_when_a_pool_is_involved() {
    // The arm above uses no pool at all, so the isolation property it certifies
    // is narrower than the property the module claims. `sample_k` permutes the
    // caller's pool IN PLACE, which is a second channel the keystream design
    // does not cover: if a later phase allocates one Vec of firm indices at
    // setup and reuses it across purposes — the natural way to avoid a per-tick
    // allocation — then the goods sample depends on how many times the labour
    // market permuted the same buffer, and CORE-04 is defeated from outside the
    // RNG entirely.
    //
    // This is the documented contract observed: build the pool FRESH per draw
    // site and the isolation holds.
    let fresh_pool = || -> Vec<u32> { (0..20).collect() };

    let baseline: Vec<u32> = {
        let r = Rngs::new(SEED);
        let mut pool = fresh_pool();
        r.stream(10, 7, Purpose::GoodsSample).sample_k(&mut pool, 5)
    };

    let after: Vec<u32> = {
        let r = Rngs::new(SEED);
        // The labour market samples first, repeatedly, from its OWN pool.
        {
            let mut labour_pool = fresh_pool();
            let mut labour = r.stream(10, 7, Purpose::LabourSample);
            for _ in 0..7 {
                labour.sample_k(&mut labour_pool, 3);
            }
        }
        let mut pool = fresh_pool();
        r.stream(10, 7, Purpose::GoodsSample).sample_k(&mut pool, 5)
    };

    assert_eq!(
        baseline, after,
        "a pool-based sample in one purpose was perturbed by another purpose's \
         sampling — CORE-04 is broken"
    );

    // The other half of the contract, stated as an observation rather than left
    // to the reader: a SHARED pool does couple the two purposes. This is why
    // `sample_k`'s doc calls pool aliasing a constraint on the caller. If this
    // assertion ever flips to equality the hazard has gone away and the
    // documented constraint can be relaxed — until then it is real.
    let shared: Vec<u32> = {
        let r = Rngs::new(SEED);
        let mut pool = fresh_pool();
        {
            let mut labour = r.stream(10, 7, Purpose::LabourSample);
            for _ in 0..7 {
                labour.sample_k(&mut pool, 3);
            }
        }
        r.stream(10, 7, Purpose::GoodsSample).sample_k(&mut pool, 5)
    };

    assert_ne!(
        baseline, shared,
        "a pool shared across purposes was expected to couple them through \
         state; if it no longer does, sample_k's pool-aliasing warning is stale"
    );
}

#[test]
fn distinct_keys_give_distinct_streams() {
    let r = Rngs::new(SEED);
    let mut seen = BTreeSet::new();
    let mut swept = 0usize;
    for tick in 0..40u32 {
        for agent in 0..40u32 {
            for p in ALL_PURPOSES {
                swept += 1;
                assert!(
                    seen.insert(r.stream(tick, agent, p).below(u64::MAX)),
                    "first-draw collision at ({tick}, {agent}, {p:?})"
                );
            }
        }
    }
    assert_eq!(swept, 40 * 40 * ALL_PURPOSES.len());
    assert_eq!(
        seen.len(),
        swept,
        "distinct keys must give distinct streams"
    );
}

#[test]
fn key_boundary_packs_and_one_step_past_it_panics() {
    let max = (1u32 << 24) - 1;
    let p = Purpose::GoodsSample;

    let top = pack_stream_key(max, max, p);
    let one_tick_below = pack_stream_key(max - 1, max, p);
    let one_agent_below = pack_stream_key(max, max - 1, p);
    assert_ne!(top, one_tick_below);
    assert_ne!(top, one_agent_below);

    // The maximum key yields a usable stream, not a degenerate one.
    let r = Rngs::new(SEED);
    let mut s = r.stream(max, max, p);
    assert_eq!(s.draws(), 0);
    let _ = s.below(1_000_000);
    assert_eq!(s.draws(), 1);

    // The panicking halves are the two `#[should_panic]` tests below.
}

#[test]
#[should_panic(expected = "tick")]
fn key_boundary_packs_and_one_step_past_it_panics_on_the_tick_field() {
    pack_stream_key(1 << 24, 0, Purpose::GoodsSample);
}

#[test]
#[should_panic(expected = "agent")]
fn key_boundary_packs_and_one_step_past_it_panics_on_the_agent_field() {
    pack_stream_key(0, 1 << 24, Purpose::GoodsSample);
}

// ---- CORE-05: every sampler consumes an exact, stated draw count ----------

#[test]
fn sample_k_consumes_exactly_k_draws() {
    let r = Rngs::new(1);

    let mut s = r.stream(0, 0, Purpose::GoodsSample);
    let mut pool: Vec<u32> = (0..20).collect();
    let picked = s.sample_k(&mut pool, 5);
    assert_eq!(s.draws(), 5, "sample_k must consume exactly k draws");
    assert_eq!(picked.len(), 5);
    assert_eq!(
        picked.iter().collect::<BTreeSet<_>>().len(),
        5,
        "sample_k must return k distinct elements"
    );

    // k == pool.len(): the whole pool, still exactly k draws.
    let mut s = r.stream(0, 1, Purpose::GoodsSample);
    let mut pool: Vec<u32> = (0..20).collect();
    let picked = s.sample_k(&mut pool, 20);
    assert_eq!(s.draws(), 20);
    assert_eq!(picked.iter().collect::<BTreeSet<_>>().len(), 20);

    // k == 0, and an empty pool: zero draws, no panic, no loop.
    let mut s = r.stream(0, 2, Purpose::GoodsSample);
    assert!(s.sample_k(&mut [], 0).is_empty());
    assert_eq!(s.draws(), 0);
}

#[test]
fn below_consumes_exactly_one_draw_for_every_n() {
    let r = Rngs::new(SEED);
    let ns: [u64; 6] = [1, 2, 3, 20, u32::MAX as u64, u64::MAX];
    for (i, n) in ns.into_iter().enumerate() {
        // A fresh sub-stream per n — a distinct agent index, never a re-entry.
        let mut s = r.stream(0, i as u32, Purpose::PriceStep);
        let v = s.below(n);
        assert_eq!(s.draws(), 1, "below({n}) must consume exactly one draw");
        assert!(v < n, "below({n}) returned {v}, which is not in 0..{n}");
    }
}

#[test]
fn coin_ppm_consumes_exactly_one_draw() {
    let r = Rngs::new(SEED);

    let mut s = r.stream(0, 0, Purpose::PriceInactionCoin);
    let _ = s.coin_ppm(500_000);
    assert_eq!(s.draws(), 1, "coin_ppm must consume exactly one draw");

    // The two degenerate probabilities are exact, not approximate.
    let mut s = r.stream(0, 1, Purpose::PriceInactionCoin);
    assert!(!s.coin_ppm(0), "p_ppm = 0 must never fire");
    let mut s = r.stream(0, 2, Purpose::PriceInactionCoin);
    assert!(s.coin_ppm(1_000_000), "p_ppm = 1_000_000 must always fire");
}

#[test]
fn shuffle_in_place_consumes_exactly_len_minus_one_draws() {
    let r = Rngs::new(SEED);

    let mut s = r.stream(0, 0, Purpose::ActivationOrderHouseholds);
    let mut pool: Vec<u32> = (0..20).collect();
    s.shuffle_in_place(&mut pool);
    assert_eq!(s.draws(), 19, "shuffle must consume exactly len - 1 draws");
    assert_eq!(
        pool.iter().collect::<BTreeSet<_>>().len(),
        20,
        "a shuffle is a permutation — no element gained or lost"
    );

    // The empty and single-element slices consume nothing and do not loop.
    let mut s = r.stream(0, 1, Purpose::ActivationOrderHouseholds);
    s.shuffle_in_place(&mut []);
    s.shuffle_in_place(&mut [7]);
    assert_eq!(s.draws(), 0);
}

// ---- The empty case, and the boundary seed -------------------------------

#[test]
fn unopened_stream_has_zero_draws() {
    let r = Rngs::new(SEED);
    let s = r.stream(0, 0, Purpose::GoodsSample);
    assert_eq!(s.draws(), 0, "an unopened stream consumes nothing");
}

#[test]
fn seed_zero_is_a_legal_master_seed() {
    let r = Rngs::new(0);
    let mut s = r.stream(0, 0, Purpose::GoodsSample);
    let drawn = take(&mut s, 4);
    assert_eq!(s.draws(), 4);
    assert_eq!(
        drawn.iter().collect::<BTreeSet<_>>().len(),
        4,
        "seed 0 must give a valid stream, not a degenerate one"
    );
}
