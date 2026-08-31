---
phase: 01-primitives-and-the-determinism-spine
fixed_at: 2026-08-31T00:00:00Z
review_path: .planning/phases/01-primitives-and-the-determinism-spine/01-REVIEW.md
iteration: 1
findings_in_scope: 16
fixed: 16
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-08-31
**Source review:** `.planning/phases/01-primitives-and-the-determinism-spine/01-REVIEW.md`
**Iteration:** 1

**Summary:**

- Findings in scope: 16 (3 Critical + 13 Warning)
- Fixed: 16
- Skipped: 0
- Out of scope, untouched: the 6 Info findings (IN-01 … IN-06)

All three Critical findings reproduced by execution before being fixed. No finding turned out
to be a false positive.

## Verification

Every gate was run in the **main checkout** (`workflow.use_worktrees` is `false` in
`.planning/config.json`, so no worktree was created and no isolation teardown was needed). The
numbers below are therefore reproducible from the tree as it stands.

| Gate | Baseline (before) | After | Δ |
|---|---|---|---|
| `cargo test` | 112 passed | **139 passed, 0 failed** | +27 |
| `cargo test --release` | 110 passed | **137 passed, 0 failed** | +27 |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean | **clean** | — |
| `cargo fmt --check` | clean | **clean** | — |
| `bash tests/toolchain.sh` | exit 0 | **exit 0** | — |
| `bash tests/lints.sh` | exit 0 | **exit 0** (60 bans fire, was 58) | — |

Test count moved only upward. Working tree is clean; every fix is committed.

**Each repaired guard was proven to bite** rather than accepted on inspection — the tree was
corrupted so the guard *should* fail, the failure was observed, and the corruption reverted. Those
observations are recorded per finding below and in the commit messages.

## Fixed Issues

### CR-01: `Money::split` panics on a valid amount

**Files modified:** `src/money.rs`
**Commit:** `bfa118c`
**Status:** fixed

Reproduced first: `Money::from_cents(i64::MAX).split(1)` panicked at `src/money.rs:152`.

The bump is now computed **inside** the branch that consumes it, so the dead `+1` on the
even-division path is gone. **The remainder distribution rule is byte-for-byte unchanged** — the
first `|amount mod n|` recipients get the extra cent by ascending index, on both signs — because
STATE.md records Phase 2 LEDG-03 and Phase 8 OWN-06 as written against it. The two pre-existing
tests pinning that rule (`[334, 333, 333]` and `[-334, -333, -333]`) still pass untouched.

Added four domain-edge cases to `split_tests` (`i64::MAX`, `i64::MIN`, `0`, at n = 1 and across
several part counts). Two of them fail before the fix.

### CR-02: `config::load` validates nothing beyond TOML shape

**Files modified:** `src/config.rs`, `tests/config_strict.rs`
**Commit:** `4d12940`
**Status:** fixed

Reproduced first: a baseline mutated to `households = 0`, `ticks = 99999999`,
`total_money_cents = -2000000`, `initial_expected_demand = nan` loaded cleanly and the binary
printed `money_cents=-2000000` and exited 0. It now exits 1 naming `sim.ticks` and its reason.

Added `Params::validate` plus `ConfigError::Domain` carrying a typed `DomainViolation { key, why }`,
called from `load` after the parse. Every bound cites the consumer that imposes it.

Two deliberate scope decisions, both to avoid inventing constraints:

- The probability bound covers only the four **coin-fed** `*_prob_ppm` keys. A blanket
  `_ppm <= PPM_SCALE` rule would reject the shipped baseline, because
  `price_ceiling_over_mc_ppm = 1150000` and `entrant_price_ratio_ppm = 1260000` are ratios, not
  probabilities. There is a regression test asserting exactly this distinction.
- Bounds not imposed by a consumer in this crate are **absent**. This validates the domain; it does
  not calibrate the economy, which stays CAL-01/CAL-02's job in Phase 11.

The float-field check is written so every line touching it contains `expected_demand`, satisfying
`tests/numeric_det.rs`'s narrower per-line rule for `config.rs`.

### CR-03: `FirmArena::live_ids` truncates the slot index

**Files modified:** `src/ids.rs`
**Commit:** `4e77827`
**Status:** fixed

Reproduced first, and the aliasing observed directly: on an arena of 65 537 slots, index 0 and
index 65 536 **both** returned `FirmId { slot: FirmSlot(0), generation: 0 }`.

Closed at construction as directed, rather than by widening the type: `FirmSlot`'s `u16` is the
committed log identity and the RNG sub-stream key's agent field is written against it, so widening
would have been the larger change. `live_ids` now uses a checked `u16::try_from`, so the reasoning
is expressed rather than left as silence.

The bound (`<= u16::MAX`) matches CR-02's `sim.firms` check exactly, so the two cannot drift.

### WR-01: The two clock bans were exercised by nothing

**Files modified:** `tests/lint-probes/float_ban_probe.rs.txt`, `tests/lints.sh`
**Commit:** `ed10d08`
**Status:** fixed

**Proven to bite:** with the `Instant::now` path corrupted to `Instannt`, check 3 now reports 59
diagnostics against 60 call sites and exits 1. Before this change that exact typo passed green.

MARKED rises 58 → 60.

### WR-02: Serde-default grep defeated by attribute order; schema/config leaf gap

**Files modified:** `tests/config_strict.rs`
**Commit:** `cf1523d`
**Status:** fixed

**Both halves proven to bite:**

- `#[serde(rename = "firms_per_owner", default)]` — invisible to the old whole-file
  `contains("serde(default")`, now reported with file, line and text.
- A defaulted `hidden_knob` added to `Ownership` and not to `baseline.toml` — `every_key_is_required`
  still passes (it can only see keys already in the shipped file), and the new
  `the_schema_and_the_shipped_config_name_the_same_leaves` fails.

### WR-03: `ALL_PURPOSES` completeness compared only against itself

**Files modified:** `src/rng.rs`
**Commit:** `3ea7a4f`
**Status:** fixed

**Proven to bite:** appending `Purpose::SomethingNew = 51` now fails the build with
`error[E0004]: non-exhaustive patterns: 'Purpose::SomethingNew' not covered`.

A `const` block ties the guard fn to the array so it cannot later be removed as dead code.

### WR-04: check 4 guarded only `disallowed_types`, and missed `pub(crate)`

**Files modified:** `tests/lints.sh`
**Commit:** `a47920f`
**Status:** fixed

**Both proven to bite:**

- `#![allow(clippy::disallowed_methods)]` injected into `src/lib.rs` — previously invisible, now
  fails check 4 by name. (This one attribute silences all 68 method bans at once.)
- The alias regex was tested against all five visibility forms: the old pattern caught 2 of 5, the
  new one catches 5 of 5 (bare, `pub`, `pub(crate)`, `pub(super)`, `pub(in ...)`).

### WR-05: `below(0)` and out-of-range `coin_ppm` were debug-only

**Files modified:** `src/rng.rs`
**Commit:** `635c0ca`
**Status:** fixed

**Proven to bite:** the three new tests pass under `--release` with real asserts, and two of them
**fail** under `--release` with the original `debug_assert!`s restored — which is the exact release
behaviour the finding described.

### WR-06: the float→integer crossing mapped `NaN` to `0`

**Files modified:** `src/numeric.rs`
**Commit:** `78a3531`
**Status:** fixed

`demand_to_units`'s finiteness check and `pow_frac_det`'s two domain preconditions are now
unconditional. This closes the release-build chain "bad config → `pow_frac` returns NaN → crossing
returns 0" at the crossing end; CR-02 closes it at the config end. Both are in.

### WR-07: split property strategies excluded the region CR-01 lived in

**Files modified:** `tests/money_props.rs`, `.proptest-regressions/money_props.txt`
**Commit:** `5d9a741`
**Status:** fixed

**Proven to bite, and this one needed a second pass.** Widening only the *amount* strategy was not
enough: against the reverted defect it passed, because the panic needs `amount == i64::MAX` **and**
a zero remainder simultaneously, which uniform sampling reaches roughly once per six hundred cases.
The part count is therefore weighted toward n = 1 as well. With that, **5 of 5 independent runs
fail against the reverted defect**, and proptest shrinks to exactly
`amount = 9223372036854775807, n = 1`.

That counterexample seed is committed to `.proptest-regressions/money_props.txt`, so it is replayed
on every future run — which is what CLAUDE.md says that file is for.

### WR-08: rayon banned by a line-anchored grep of the manifest

**Files modified:** `tests/toolchain.sh`
**Commit:** `e11eb64`
**Status:** fixed

**Proven to bite:** a `[dependencies.rayon]` table-form dependency returns "no match" from the old
grep and fails the new graph search. `$TREE` is hoisted above check 2 and shared with check 4.

### WR-09: grade-B row cites 0.2 but ships 0.8

**Files modified:** `config/PROVENANCE.md`, `config/baseline.toml`
**Commit:** `cbfe606`
**Status:** fixed (as a recorded open item — **no value was changed**)

**Deliberately not resolved.** Per D-20 and the orchestrator's direction, the parameter's meaning
was not settled from model memory. Recorded as **PROVENANCE.md item V-3a** in the same style as the
existing V-1…V-5 items, listing all three possible readings (transcription error / derived `1 − 0.2`
which would regrade the row to C / misread source parameter) and the action for each, so the Phase 6
verification gate has something concrete to look for. V-3 alone flagged the BAM rows as unread but
did not record this specific numeric mismatch.

A pointer comment was added to `baseline.toml` **above** the `GRADE:` line — above, so the
annotation stays adjacent to its key and `no_annotation_is_orphaned` still holds. The config hash
changes as a result, which is correct: the comments carry the source grades and CORE-11 makes them
load-bearing.

### WR-10: float confinement matched type *names* only

**Files modified:** `tests/numeric_det.rs`
**Commit:** `7e2402a`
**Status:** fixed

**Proven to bite:** an inferred-`f64` smoothing function added to `src/ids.rs` names no float type
(invisible to the old check) and produces **0 clippy errors** (invisible to the lint) — and the new
literal check reports it with file, line and source text.

Line comments are stripped for the **literal** check only, because `src/rng.rs` legitimately writes
the crate version "0.10.2" in its module docs; the **type** check still reads whole lines. The
asymmetry is deliberate and documented at the helper. The allowlist now matches the path relative to
`src/` rather than the bare basename.

### WR-11: `sha2` pin comment cited a rationale the code disclaims

**Files modified:** `Cargo.toml`
**Commit:** `2530891`
**Status:** fixed

Confirmed in the source before changing anything: `config_hash` formats each byte with `{b:02x}` and
never uses a `LowerHex` impl on the digest type — its own comment says it is written that way "so
the idiom survives a future `sha2` major bump".

Comment only; no version change, `Cargo.lock` untouched. **No claim is made about sha2 0.11's actual
API**, because that was not checked here and asserting one would repeat the original mistake.

### WR-12: `raw_i64_overflow_panics_in_release` is vacuous under `cargo test`

**Files modified:** `tests/toolchain.sh`, `tests/tracer_end_to_end.rs`
**Commit:** `7ca0e43`
**Status:** fixed

**Proven to bite, and the vacuity confirmed:** with `overflow-checks` commented out, the debug
suite stayed green at 5 passed — exactly the finding's claim — while `tests/toolchain.sh` check 4b
now fails by name.

Test renamed to `raw_i64_overflow_panics_when_overflow_checks_are_on`, with the limitation stated in
the comment instead of contradicted by it. The manifest fact is matched with a stateful `awk` scan
rather than `grep -Pzo`, which needs a PCRE-enabled grep that is not present everywhere; the scan
tracks the profile section so an `overflow-checks` line under a different profile cannot satisfy it.

### WR-13: `sample_k` permutes the caller's pool

**Files modified:** `src/rng.rs`, `tests/determinism_rng.rs`
**Commit:** `f16f0cc`
**Status:** fixed

Added a `# Pool aliasing` section to both samplers and a second test arm with two halves: fresh
pools per draw site stay isolated (`assert_eq`), and a **shared** pool demonstrably does couple the
purposes (`assert_ne`). The second half is what keeps the warning honest — if it ever flips to
equality, the hazard is gone and the documented constraint is stale.

## Skipped Issues

None. All 16 in-scope findings were fixed.

## Out of Scope (untouched, as directed)

IN-01 (duplicated numeric tests), IN-02 (copy-pasted `rust_sources`), IN-03 (tracer temp-dir leak),
IN-04 (`tests/_probe.rs` not gitignored), IN-05 (`>= 40` vs 41 leaf keys), IN-06 (`Option<` scan too
broad).

Two notes for whoever picks these up:

- **IN-05 is now partly covered.** WR-02's new
  `the_schema_and_the_shipped_config_name_the_same_leaves` compares the schema and shipped leaf sets
  exactly, so deleting a key from `baseline.toml` alone now fails that test. The loose `>= 40` floor
  in `every_key_is_required` is still loose and still worth tightening to `assert_eq!(…, 41)`.
- **IN-06 did not trigger.** `Params::validate` was written without `Option`, so the
  `no_optional_fields_in_the_config_schema` scan still passes. The false-positive risk the finding
  describes remains real for the next person to touch `config.rs`.

## Notes for the verifier

- **CR-02 changes the config hash contract surface, not the hash rule.** `config_hash` is unchanged;
  but `config/baseline.toml` gained comment lines (WR-09), so the shipped config's hash differs from
  any value recorded before this commit series. That is by design — comments carry source grades.
- **No behaviour on the happy path changed.** `Money::split`'s distribution, the RNG key packing,
  the sub-stream layout, `pow_frac`'s bit count and the crossing's rounding are all identical. The
  new asserts fire only on inputs that previously produced a silently wrong answer, and the tracer
  draw for a given seed is unchanged (`draw=934366` at seed 7, before and after).
- **CR-01's fix is semantics-preserving and load-bearing.** If a later phase's ledger disagrees with
  the ascending-index remainder rule, this fix is not the cause — the rule was preserved
  deliberately and is pinned by two tests.

---

_Fixed: 2026-08-31_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
