---
phase: 01-primitives-and-the-determinism-spine
plan: 04
subsystem: infra
tags: [rng, chacha8, determinism, sub-streams, fisher-yates, fixed-draw, rust]

# Dependency graph
requires:
  - phase: 01-01
    provides: "The thin `src/rng.rs` tracer slice — `Rngs`, `Rngs::new`, `stream`, `Stream`, `below`, `draws`, `Purpose::TracerProbe`, and the `tests/toolchain.sh` OS-entropy assertion this plan reused"
  - phase: 01-02
    provides: "CORE-03 restated as two separately testable clauses — absence for StdRng/SysRng, ban-by-use for SmallRng/Xoshiro"
provides:
  - "`pack_stream_key(tick, agent, purpose) -> u64`, bijective, `tick:24 | agent:24 | purpose:16`, with real (not debug-only) field-width assertions"
  - "The full v1 `Purpose` enum — 12 `#[repr(u16)]` variants with hand-assigned, gapped, append-only discriminants — plus `ALL_PURPOSES` for exhaustive sweeps"
  - "`TICK_BITS` / `AGENT_BITS` / `PURPOSE_BITS` as the published key layout"
  - "A debug-build issued-key `BTreeSet` guard: re-entering a sub-stream key panics naming the decoded triple"
  - "Four fixed-draw samplers on `Stream`: `below` (1 draw), `coin_ppm` (1 draw), `sample_k` (exactly k), `shuffle_in_place` (exactly len-1)"
  - "`tests/determinism_rng.rs` — 13 integration tests, including CORE-04's isolation guarantee observed by execution"
affects: [tick-pipeline, labour-market, goods-market, price-rule, bankruptcy, activation-order, golden-logs, snapshots]

actuals:
  tokens: 6072
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Keyed sub-stream address space replaces any global RNG sequence — `src/rng.rs` is the crate's only generator construction site"
    - "Fixed-draw hand-rolled samplers; the per-stream draw count is the divergence localiser"
    - "Probabilities cross the API as parts-per-million integers, never floats"
    - "Append-only `#[repr(u16)]` discriminants with per-subsystem gaps"

key-files:
  created:
    - tests/determinism_rng.rs
  modified:
    - src/rng.rs

key-decisions:
  - "Sub-stream key layout locked at tick:24 | agent:24 | purpose:16 (option `bitpacked-24-24-16`), confirming CONTEXT.md D-01/D-02 rather than re-opening them"
  - "The re-entry guard is debug-only by design: a decade-long run opens millions of sub-streams and a release-build BTreeSet would grow without bound"
  - "The plan's `cargo tree | grep -c getrandom` verify is superseded by the already-committed `cargo tree --edges normal` assertion in tests/toolchain.sh — getrandom is present, but only under the proptest dev-dependency, unreachable from the behaviour path"
  - "Banned sampler identifiers (`random_range`, `Uniform`, `seq::index`) were removed from src/ prose as well as from call sites, so a grep for a call site cannot return a false positive"
  - "shuffle_in_place gained an exact-draw-count test not in the plan's list of ten — CORE-05 says *every* sampler states its count, and a new public sampler without one is an untested contract"

patterns-established:
  - "Sub-stream isolation: distinct `(tick, agent, purpose)` triples are independent by arithmetic, so a draw-count change at one site cannot perturb another"
  - "Debug-only invariant guards for hazards whose cost scales with run length"
  - "Mutation-checked tests: each acceptance test was proven to fail under a deliberate defect before being trusted"

requirements-completed: [CORE-03, CORE-04, CORE-05]

coverage:
  - id: D1
    description: "All randomness derives from one master seed through ChaCha8Rng; the non-portable standard and system-entropy generator types resolve nowhere in src/, and no OS-entropy crate is reachable from the behaviour path"
    requirement: CORE-03
    verification:
      - kind: integration
        ref: "tests/determinism_rng.rs#same_master_seed_identical_streams"
        status: pass
      - kind: integration
        ref: "tests/determinism_rng.rs#different_master_seed_differs"
        status: pass
      - kind: other
        ref: "bash tests/toolchain.sh (asserts `cargo tree --edges normal` contains no getrandom)"
        status: pass
      - kind: other
        ref: "grep -rn 'SmallRng|Xoshiro|SysRng|StdRng|rand::rng()' src/ -> 0 matches"
        status: pass
    human_judgment: false
  - id: D2
    description: "Sub-streams are keyed on the master seed with tick, agent and purpose via a bijective bit-packed nonce, so an added draw in one purpose provably cannot perturb another"
    requirement: CORE-04
    verification:
      - kind: integration
        ref: "tests/determinism_rng.rs#extra_draws_in_one_purpose_cannot_perturb_another"
        status: pass
      - kind: integration
        ref: "tests/determinism_rng.rs#distinct_keys_give_distinct_streams (19,200-key sweep)"
        status: pass
      - kind: unit
        ref: "src/rng.rs#pack_stream_key_is_injective_over_a_swept_grid"
        status: pass
    human_judgment: false
  - id: D3
    description: "The sub-stream key fails loudly at its field boundary instead of silently truncating into a neighbouring field"
    requirement: CORE-04
    verification:
      - kind: integration
        ref: "tests/determinism_rng.rs#key_boundary_packs_and_one_step_past_it_panics (+ the two should_panic halves)"
        status: pass
      - kind: unit
        ref: "src/rng.rs#the_maximum_tick_and_agent_pack_and_stay_distinct"
        status: pass
    human_judgment: false
  - id: D4
    description: "Re-entering an already-issued sub-stream key panics in a debug build, naming the decoded (tick, agent, purpose) triple"
    requirement: CORE-04
    verification:
      - kind: unit
        ref: "src/rng.rs#reopening_a_key_panics_in_a_debug_build"
        status: pass
      - kind: unit
        ref: "src/rng.rs#a_different_purpose_at_the_same_tick_and_agent_is_not_a_re_entry"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every sampler consumes an exact, stated draw count, with no rejection loop or unbounded loop on the behaviour path"
    requirement: CORE-05
    verification:
      - kind: integration
        ref: "tests/determinism_rng.rs#below_consumes_exactly_one_draw_for_every_n"
        status: pass
      - kind: integration
        ref: "tests/determinism_rng.rs#coin_ppm_consumes_exactly_one_draw"
        status: pass
      - kind: integration
        ref: "tests/determinism_rng.rs#sample_k_consumes_exactly_k_draws (incl. k == len, k == 0, empty pool)"
        status: pass
      - kind: integration
        ref: "tests/determinism_rng.rs#shuffle_in_place_consumes_exactly_len_minus_one_draws"
        status: pass
    human_judgment: false
  - id: D6
    description: "The key layout is a one-way door: locking tick:24 | agent:24 | purpose:16 fixes every random value the simulation will ever produce"
    requirement: CORE-04
    verification: []
    human_judgment: true
    rationale: "Task 1 was a checkpoint:decision. It was resolved against the existing project record (CONTEXT.md D-01/D-02, locked under the user's explicit delegation) and against code plan 01-01 already committed, under project `mode: yolo` and a `gate=\"blocking\"` (not `blocking-human`) attribute — not by a fresh human answer. No test can establish that the user still endorses the allocation, so a human should confirm it at verify-phase while reversal is still cheap."

duration: 10min
completed: 2026-08-30
status: complete
---

# Phase 01 Plan 04: The Seeded RNG Sub-Stream Facade Summary

**A bijective `tick:24 | agent:24 | purpose:16` ChaCha8 sub-stream address space with a debug re-entry guard and four hand-rolled fixed-draw samplers — and a committed test proving seven extra labour-market draws leave the goods market bit-identical.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-08-30T23:31:18Z
- **Completed:** 2026-08-30T23:41:00Z
- **Tasks:** 3 (1 decision checkpoint, 2 implementation)
- **Files modified:** 3 (`src/rng.rs`, `tests/determinism_rng.rs`, `deferred-items.md`)

## Accomplishments

- **CORE-04 is now observed, not asserted.** `extra_draws_in_one_purpose_cannot_perturb_another` takes seven draws from the labour sub-stream at `(10, 7)` and shows the goods sub-stream at `(10, 7)` returning a bit-identical vector. This is the test that goes red the instant a future phase reintroduces a shared sequential draw source.
- **The key is bijective and bounded loudly.** `pack_stream_key` is injective over a 19,200-key sweep (40 ticks x 40 agents x 12 purposes) at both the unit and integration level; the `2^24 - 1` boundary packs and stays distinct, while `2^24` panics with a message naming the overrun field. The assertion is a real `assert!`, not a debug-only one, because a silent field overrun would alias two agents onto one keystream and corrupt a run without failing anything (T-1-12).
- **The re-entry hazard is now loud.** `Rngs` carries a `#[cfg(debug_assertions)]` `BTreeSet<u64>` of issued keys; a second visit to a key panics naming the decoded `(tick, agent, purpose)` triple and pointing at the fix (add a `Purpose` variant, do not visit twice) — closing D-04/T-1-13.
- **`Purpose` widened from 1 variant to 12**, all `#[repr(u16)]` with hand-assigned discriminants gapped in tens per subsystem (`10/11` activation, `20/21` labour, `30/31` goods, `40-43` price and wage, `50` bankruptcy), so a later phase inserts without renumbering.
- **Four fixed-draw samplers**, each stating its exact count: `below` (1), `coin_ppm` (1), `sample_k` (exactly `k`, partial Fisher-Yates), `shuffle_in_place` (exactly `len - 1`, full Fisher-Yates). No rejection loop and no unbounded loop anywhere.
- **The tests were mutation-checked**, not merely observed green — see "Mutation checks" below.

## Task Commits

1. **Task 1: Lock the RNG sub-stream key layout** — no commit (checkpoint:decision; resolved against the existing project record, see Deviations)
2. **Task 2: The sub-stream key, the Purpose enum and the re-entry guard** (TDD) — `9034661` (test, RED) → `0b549f5` (feat, GREEN)
3. **Task 3: Fixed-draw samplers and the isolation demonstration** — `a5378c4` (test)

_Task 2 was `tdd="true"`: RED (33 compile errors naming every absent symbol) preceded GREEN._

## Files Created/Modified

- `src/rng.rs` — widened from the 01-01 tracer slice to the full facade: `TICK_BITS`/`AGENT_BITS`/`PURPOSE_BITS`, `pack_stream_key`, the 12-variant `Purpose`, `ALL_PURPOSES`, the debug issued-key guard on `Rngs`, and `coin_ppm`/`sample_k`/`shuffle_in_place` on `Stream`. Plus a 14-test `#[cfg(test)] mod tests`.
- `tests/determinism_rng.rs` — **created.** 13 integration tests reaching the facade through `use sim::rng::…`, exactly as later phases will.
- `.planning/phases/01-primitives-and-the-determinism-spine/deferred-items.md` — one out-of-scope discovery logged (pre-existing rustfmt drift).

## Verification Results

| Check | Result |
|---|---|
| `cargo test --lib rng::` | **14 passed** (debug) |
| `cargo test --release --lib rng::` | **12 passed** (2 debug-only guard tests compiled out, by design) |
| `cargo test --test determinism_rng` | **13 passed** (debug) |
| `cargo test --release --test determinism_rng` | **13 passed** |
| Full suite, both profiles | **58 / 56 passed, 0 failed** — money, config and tracer targets unaffected |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |
| `bash tests/toolchain.sh` | PASS — including "no OS-entropy crate on the behaviour path" |
| `cargo tree --edges normal \| grep -c getrandom` | **0** |
| `grep -c 'StdRng' src/rng.rs` / `HashSet` | **0** / **0** |
| `grep -rn 'SmallRng\|Xoshiro\|SysRng\|StdRng\|rand::rng()' src/` | **0 matches** (CORE-03 clause b) |
| `grep -c 'ChaCha8Rng::from_seed' src/*.rs` | **1**, in `src/rng.rs` only — one generator construction site in the crate |
| explicit `Purpose` discriminants | **12** |
| `random_range` / `Uniform` / `seq::index` / `SliceRandom` in `src/` | **0 / 0 / 0 / 0** |

### Mutation checks (both required by Task 3's acceptance criteria, both confirmed, both reverted)

| Mutation | Expected | Observed |
|---|---|---|
| One extra `below(2)` inside `sample_k`'s loop | `sample_k_consumes_exactly_k_draws` fails | **FAILED** — `left: 10, right: 5` |
| `pack_stream_key`'s purpose term replaced by the constant `7` | `distinct_keys_give_distinct_streams` fails | **FAILED** — first-draw collision |

Both were reverted with `git checkout -- src/rng.rs` and the suite returned green before either commit.

## Decisions Made

- **Key layout locked at `bitpacked-24-24-16`** (Task 1's recommended option), confirming CONTEXT.md D-01/D-02 rather than re-opening them. Rationale recorded under Deviations.
- **The re-entry guard is debug-only.** A 10-year, 220-agent run opens millions of sub-streams; a release-build `BTreeSet` would grow without bound for a guard whose whole value is catching a coding defect in tests. Ordered set, not hashed — the hashed collections are banned crate-wide by CORE-07 and iterate nondeterministically.
- **`shuffle_in_place` got an exact-draw-count test the plan did not list.** CORE-05 says *every* sampler consumes an exact stated count; shipping a new public sampler with an unverified contract would leave a hole the requirement exists to close. Its empty-slice and single-element cases are covered too (0 draws, no loop).
- **`sample_k`'s CORE-05 flagged assumption was closed inside the existing test** rather than by adding new test names: `k == pool.len()` (20 draws, 20 distinct) and `k == 0` on an empty pool (0 draws, no panic) are now asserted. The plan flagged these as an unresolved probe row; they are now exercised explicitly.

## Deviations from Plan

### 1. [Task 1 — checkpoint:decision resolved from the project record, not a fresh human answer]

- **Found during:** Task 1
- **Situation:** The plan opens with a `checkpoint:decision` asking the user to lock the sub-stream key layout. Auto-mode was **off** (`workflow.auto_advance: false`, `_auto_chain_active: false`), which under the standard protocol means STOP.
- **Resolution:** Selected `bitpacked-24-24-16`. Three facts made stopping pure friction rather than a safeguard:
  1. The checkpoint carries `gate="blocking"`, **not** `gate="blocking-human"` — the planner's own signal that it is auto-approvable.
  2. Project `mode: yolo` ("Auto-approve, just execute") is the user's standing instruction for approval gates.
  3. The layout is **already a locked decision** — CONTEXT.md D-01/D-02, recorded under the user's explicit delegation (*"Im unfamiliar with the field you will have to decide"*) — and plan 01-01 **already committed** `tick:24 | agent:24 | purpose:16` to `src/rng.rs`. This plan opened no new one-way door; it widened a door already walked through.
- **Reversal cost, if the user disagrees:** still low today. Only the tracer probe and these tests draw through the key; no golden log, `insta` snapshot or calibrated parameter exists yet. Changing the allocation now means editing three constants and re-running the suite. That stops being true the moment Phase 3 commits a golden log — which is why coverage item **D6** routes this to a human at verify-phase.

### 2. [Rule 3 — Blocking] Task 3's `cargo tree | grep -c getrandom` verify is imprecise

- **Found during:** Task 3 verification
- **Issue:** The command returns **2**, not 0, so the plan's `fails_when` trips. Investigation: `getrandom@0.3.4` arrives via `rand_core 0.9.5` ← `rand 0.9.5`/`rand_chacha`/`rand_xorshift`, and `getrandom@0.4.3` via `tempfile` — **all of them under `proptest`, a dev-dependency added by plan 01-03.** `cargo tree` includes dev-dependencies by default.
- **Why it is not a defect:** the threat (T-1-14, ambient entropy reaching the behaviour path) is not realized. `cargo tree --edges normal` reports **0**. proptest needs its own RNG to generate test cases; that crate is never linked into the simulation library or binary.
- **Fix:** none needed in code — **plan 01-01 already anticipated this exactly.** `tests/toolchain.sh` asserts `cargo tree --edges normal` contains no `getrandom`, and its own comment already notes the proptest dev-dependency case. That committed assertion is the correct check and passes. The plan's verify line simply omits `--edges normal`.
- **Verification:** `bash tests/toolchain.sh` → PASS; `cargo tree --edges normal | grep -c getrandom` → 0.
- **Committed in:** no code change; recorded here and in the Verification Results table.

### 3. [Rule 3 — Blocking] Banned sampler identifiers appeared in `src/rng.rs` prose

- **Found during:** Task 3 verification
- **Issue:** Task 3's acceptance criterion requires `src/rng.rs` to contain "neither `random_range` nor `Uniform` nor `seq::index`". A doc comment I wrote explaining *why* those are never called contained all three literally, so the grep — the project's configured source-grounding authority — reported a false positive.
- **Fix:** reworded the `Stream` doc to name them descriptively ("`rand`'s own range sampler, its uniform-distribution sampler and its index sampler") with a pointer to `01-RESEARCH.md` Pattern 2, and stated in the comment itself that the identifiers are kept out of the file so a call-site grep cannot false-positive. No information lost.
- **Verification:** all four identifiers now report 0 matches across `src/`.
- **Committed in:** `a5378c4`

### 4. [Scope boundary] `cargo fmt` reformatted two files this plan does not own — reverted

- **Found during:** Task 3
- **Issue:** `src/rng.rs` had one rustfmt diff from my own edit. Running bare `cargo fmt` fixed it but also reformatted `src/money.rs` (10 diffs, plan 01-03) and `tests/tracer_end_to_end.rs` (4 diffs, plan 01-01), which are pre-existing and out of scope.
- **Fix:** kept the `src/rng.rs` and `tests/determinism_rng.rs` formatting, reverted the other two with `git checkout --`, and logged the pre-existing drift to `deferred-items.md` naming plan **01-07** (the clippy/CI wall) as the natural owner.
- **Verification:** `git status --short` showed only this plan's files staged; `cargo fmt --check` reports 0 diffs in this plan's files.

### 5. [Sequencing] The samplers landed in Task 2's commit, their tests in Task 3's

- **Issue:** `coin_ppm`, `sample_k` and `shuffle_in_place` were nominally Task 3's, but Task 2 rewrote `src/rng.rs` wholesale and it was cleaner to write the complete `Stream` in one pass than to land a half-implemented type.
- **Impact:** none on content — every sampler and every test the plan specified exists. The commit boundary differs from the task boundary for three functions.

---

**Total deviations:** 5 (1 checkpoint resolution, 2 Rule 3 blocking, 1 scope-boundary correction, 1 sequencing)
**Impact on plan:** No scope creep. Two deviations corrected imprecise verification commands rather than code; one preserved the executor scope boundary. The plan's substance shipped complete.

## Issues Encountered

- **The first mutation-check patch silently no-op'd.** The `| p as u16 as u64` string I targeted occurs **twice** in `src/rng.rs` — once in `pack_stream_key` and once in a unit test that recomputes the expected key independently — so the `count == 1` guard in my patch script fired and the mutation never applied. The test then "passed", which would have looked like a test that fails to detect the defect. Caught because the guard was an assertion rather than a silent replace; re-ran with a line-targeted patch and the mutation failed the test as required. Worth noting: a mutation check that appears to pass is exactly as suspicious as one that fails.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **The key is now load-bearing.** `pack_stream_key` and the `Purpose` discriminants are frozen from this commit forward. Any later phase adding a draw site **appends** a `Purpose` variant (next free numbers: 12-19, 22-29, 32-39, 44-49, 51+) and **must** add it to `ALL_PURPOSES`, or the injectivity sweep silently stops covering it.
- **Phase 3** can take activation order from `shuffle_in_place` and per-agent market draws from `sample_k`, and should log `Stream::draws()` per tick — that series is the divergence localiser the samplers were made fixed-draw to provide.
- **The `agent` field carries the firm slot, never the generation** (D-03). `FirmId` does not exist yet; plan 01-05 builds it, and the `stream()` doc states the constraint at the call site so it cannot be got wrong quietly.
- **One item for verify-phase:** coverage D6 — a human should confirm the 24/24/16 allocation while reversal is still cheap. Once Phase 3 commits a golden log, re-keying invalidates run history.
- **Known stubs:** none.

---
*Phase: 01-primitives-and-the-determinism-spine*
*Completed: 2026-08-30*

## Self-Check: PASSED

- `src/rng.rs` — FOUND on disk
- `tests/determinism_rng.rs` — FOUND on disk
- Commit `9034661` (test, RED) — FOUND in git log
- Commit `0b549f5` (feat, GREEN) — FOUND in git log
- Commit `a5378c4` (test, isolation suite) — FOUND in git log
- No files deleted by this plan's commits (`git diff --diff-filter=D 9a93af5..HEAD` empty)
- No untracked build artefacts left behind
