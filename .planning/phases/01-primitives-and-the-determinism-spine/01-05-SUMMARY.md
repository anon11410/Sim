---
phase: 01-primitives-and-the-determinism-spine
plan: 05
subsystem: core-primitives
tags: [rust, generational-arena, ids, float-determinism, ieee-754, sqrt, integer-domain]

# Dependency graph
requires:
  - phase: 01-01
    provides: "Cargo.toml (edition 2024, overflow-checks, clippy lint levels), src/lib.rs with #![forbid(unsafe_code)], the lib.rs + thin main.rs layout that lets integration tests reach the whole surface"
provides:
  - "src/ids.rs — HouseholdId, FirmSlot, GoodId, the generational FirmId { slot, generation }, Account, and FirmArena with Option-returning accessors and in-place respawn"
  - "A firm identity that resolves to None after its slot is respawned, rather than silently to the new occupant (CORE-06)"
  - "A derived total order on (slot, generation) so any comparator over agents can be tie-broken by identity (LABR-09's precondition)"
  - "src/numeric.rs — the crate's entire float domain: pow_frac_det, pow_frac, POW_FRAC_BITS, demand_to_units, PPM_SCALE, MILLI_SCALE"
  - "A fractional power that is bit-identical across invocations, computed only from the square root and multiplication, so the powf ban never needs an allow-attribute (D-12)"
  - "demand_to_units — the crate's single named float-to-integer crossing, rounding half away from zero and saturating (D-11)"
  - "tests/numeric_det.rs::confinement_of_the_float_domain — the module-level half of the float ban, as a test rather than a convention"
affects: [01-06 config loader, 01-07 lint wall, 01-08 provenance, Phase 2 ledger, Phase 6 labour matching, Phase 7 household consumption rule, Phase 9 price and wage rules, Phase 10 bankruptcy and respawn]

actuals:
  tokens: 7600
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "Generational arena: identity = (slot, generation), accessors return Option, respawn increments the generation in place and never reorders storage"
    - "Confined float domain: one module names the floating-point type, one named function crosses back to integers, and a test enforces both at file granularity"
    - "Binary-digit fractional power from sqrt and multiplication only — every operation IEEE-754 correctly rounded, so the result is uniquely determined"

key-files:
  created:
    - src/ids.rs
    - src/numeric.rs
    - tests/ids_generational.rs
    - tests/numeric_det.rs
  modified:
    - src/lib.rs
    - src/rng.rs

key-decisions:
  - "The generation field is spelled `generation`, not `gen`: `gen` is a reserved keyword in Rust edition 2024 and does not compile as an identifier. The alternative, `r#gen`, would force a raw identifier at every construction and field access for ten more phases. Type shape, derived total order and the log identity (slot, generation) are unchanged."
  - "FirmArena exposes no element-removal operation at all — not swap_remove, not remove, not retain, drain, truncate or pop. respawn_in_place is the only mutation of the slot vector and it changes no index and no length (BANK-03)."
  - "respawn_in_place panics on an out-of-range slot rather than returning Option: the arena is fixed-length for the life of a run, so an out-of-range slot is a program defect, not a runtime condition to report."
  - "POW_FRAC_BITS = 40 is a committed constant, not a tunable — changing it changes every trajectory exactly as changing an economic parameter would. It is code, not config, per the CORE-10 carve-out (D-14)."
  - "src/numeric.rs contains no occurrence of the substrings powf, exp, ln or log — including in prose. Its documentation is worded to keep the mechanical grep honest ('binary digits of the fractional power', 'written to the run record' rather than 'logged'), so the check needs no comment-stripping heuristic to be argued with."
  - "The confinement test's allowlist is file-level and comment-blind: a floating-point type named in a doc comment counts. This is stricter than necessary and deliberately so — a heuristic that skips comments is a heuristic someone will later widen."

patterns-established:
  - "Generational identity: hold (slot, generation); resolve through an Option-returning accessor that compares the stored generation. A stale identity is a typed miss the compiler forces the caller to handle."
  - "Only FirmId.slot is ever passed as the RNG sub-stream key's agent field, never the generation (D-03) — stated as a comment on both FirmId and FirmArena so the rule is found where it would be broken."
  - "Every mechanical claim in this plan was confirmed by mutation: remove the generation check and the stale-identity test fails; lower POW_FRAC_BITS to 20 and the bit-count test fails; add a float to src/money.rs and the confinement test fails. Each mutation was reverted before commit."

requirements-completed: [CORE-06, CORE-10]

coverage:
  - id: D1
    description: "A firm identity held across a respawn of its slot resolves to None through both accessors, never to the new occupant"
    requirement: CORE-06
    verification:
      - kind: unit
        ref: "src/ids.rs#a_stale_identity_is_a_typed_miss_through_both_accessors"
        status: pass
      - kind: integration
        ref: "tests/ids_generational.rs#stale_identity_after_respawn_is_a_typed_miss"
        status: pass
      - kind: other
        ref: "mutation: `(true || record.generation == id.generation)` in FirmArena::get -> stale-identity test FAILED; reverted"
        status: pass
    human_judgment: false
  - id: D2
    description: "Respawn happens in place at the same slot with the generation incremented, and the arena exposes no storage-reordering removal operation"
    requirement: CORE-06
    verification:
      - kind: unit
        ref: "src/ids.rs#respawn_returns_the_same_slot_at_exactly_one_greater_generation"
        status: pass
      - kind: unit
        ref: "src/ids.rs#respawn_disturbs_no_neighbouring_slot"
        status: pass
      - kind: integration
        ref: "tests/ids_generational.rs#respawn_does_not_disturb_neighbouring_slots"
        status: pass
      - kind: other
        ref: "grep -cE 'swap_remove|\\.remove\\(|retain|drain|truncate|pop\\(' src/ids.rs => 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "FirmId carries a derived total order over (slot, generation), so any agent comparator can be tie-broken by identity"
    requirement: CORE-06
    verification:
      - kind: unit
        ref: "src/ids.rs#firm_ids_order_slot_major_then_generation"
        status: pass
      - kind: unit
        ref: "src/ids.rs#the_newtypes_and_account_carry_equality_and_a_total_order"
        status: pass
      - kind: integration
        ref: "tests/ids_generational.rs#firm_ids_sort_slot_major_then_generation"
        status: pass
      - kind: integration
        ref: "tests/ids_generational.rs#live_ids_are_ascending_and_complete"
        status: pass
    human_judgment: false
  - id: D4
    description: "The fractional power is computed only from IEEE-754 correctly-rounded operations and is bit-identical across 100,000 invocations; the exact square-root cases hold at alpha = 1/2 and 1/4"
    verification:
      - kind: unit
        ref: "src/numeric.rs#pow_frac_returns_one_bit_pattern_across_many_calls"
        status: pass
      - kind: integration
        ref: "tests/numeric_det.rs#pow_frac_is_bit_identical_across_many_invocations"
        status: pass
      - kind: integration
        ref: "tests/numeric_det.rs#pow_frac_matches_repeated_square_roots_at_negative_powers_of_two"
        status: pass
      - kind: other
        ref: "cargo test --release --test numeric_det => 5 passed"
        status: pass
    human_judgment: false
  - id: D5
    description: "POW_FRAC_BITS is load-bearing: 20 and 40 bits differ on the swept range, 40 and 52 agree to within 1e-9 relative, and pow_frac uses the committed constant"
    requirement: CORE-10
    verification:
      - kind: integration
        ref: "tests/numeric_det.rs#bit_count_is_load_bearing"
        status: pass
      - kind: unit
        ref: "src/numeric.rs#pow_frac_uses_the_committed_bit_count"
        status: pass
      - kind: other
        ref: "mutation: POW_FRAC_BITS 40 -> 20 => 2 numeric:: tests FAILED; reverted"
        status: pass
    human_judgment: false
  - id: D6
    description: "demand_to_units is the single named float-to-integer crossing: rounds half away from zero, saturates rather than wrapping, asserts finiteness in debug builds"
    verification:
      - kind: unit
        ref: "src/numeric.rs#the_crossing_rounds_half_away_from_zero"
        status: pass
      - kind: unit
        ref: "src/numeric.rs#the_crossing_maps_zero_to_zero_and_saturates_out_of_range"
        status: pass
      - kind: integration
        ref: "tests/numeric_det.rs#crossing_rounds_half_away_from_zero_and_saturates"
        status: pass
    human_judgment: false
  - id: D7
    description: "The float domain is one module wide: only src/numeric.rs (and src/config.rs, on lines naming expected_demand) may name a floating-point type anywhere under src/"
    verification:
      - kind: integration
        ref: "tests/numeric_det.rs#confinement_of_the_float_domain"
        status: pass
      - kind: other
        ref: "mutation: `pub const PROBE: f64 = 1.0;` added to src/money.rs => confinement test FAILED at src/money.rs:54; reverted"
        status: pass
    human_judgment: false
  - id: D8
    description: "POW_FRAC_BITS, PPM_SCALE and MILLI_SCALE are documented const items in src/numeric.rs with an in-code rationale, not configuration keys (the CORE-10 carve-out)"
    requirement: CORE-10
    verification:
      - kind: unit
        ref: "src/numeric.rs#the_integer_scale_constants_are_ppm_and_milli"
        status: pass
      - kind: other
        ref: "grep -c 'allow(clippy' src/numeric.rs => 0; the module needs no lint exemption"
        status: pass
    human_judgment: true
    rationale: "The tests prove the constants exist with the stated values and that the module needs no lint exemption. They cannot prove the other half of CORE-10's carve-out — that config/PROVENANCE.md carries the matching GRADE: PROJECT entry — because that file is authored by plan 01-08. Verification of the carve-out as a whole must wait until 01-08 lands."

duration: 10 min
completed: 2026-08-30
status: complete
---

# Phase 01 Plan 05: Identity and the Confined Float Domain Summary

**A firm identity that becomes a typed `None` the moment its slot is respawned, and a fractional-power routine built from square roots alone so the `powf` ban never has to be weakened in Phase 7.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-08-30T23:39:30Z
- **Completed:** 2026-08-30T23:49:09Z
- **Tasks:** 3 of 3
- **Files modified:** 6 (4 created, 2 modified)

## Accomplishments

- **`src/ids.rs`: generational identity (CORE-06).** `FirmId` carries a slot and a generation; `FirmArena::get`/`get_mut` compare the stored generation before returning, so an identity captured before a respawn resolves to `None` rather than to the new occupant of the reused slot. This converts the hardest class of defect in an emergent system — a plausible wrong number — into a miss the compiler forces the caller to handle.
- **The arena reorders nothing.** `respawn_in_place` mutates the record at the same index; the vector's length and every other slot's position are untouched. The arena exposes no element-removal operation at all, so BANK-03's "`swap_remove` is never used on agent collections" is enforced by absence rather than by review.
- **A total order for tie-breaking.** `Ord` is derived on every identity type, so `(slot, generation)` is a total order and any comparator that ends in an identity comparison has no unspecified tie order — the precondition LABR-09 is written against.
- **`src/numeric.rs`: the whole float domain, one module wide (D-11, D-12).** `pow_frac_det` computes `x^alpha` from the binary digits of `alpha` using only the square root and multiplication, both IEEE-754 correctly rounded. Measured here: bit-identical across 100,000 invocations, 40-vs-52-bit agreement to 1.9e-12 relative, exactly `x.sqrt()` at alpha = 1/2 and exactly `x.sqrt().sqrt()` at alpha = 1/4. This is what makes the Phase 1 clippy list honest — Phase 7 has a deterministic route to `(m/P_bar)^0.9` and never needs an allow-attribute.
- **One named crossing back to integers.** `demand_to_units` rounds half away from zero, saturates rather than wrapping on an out-of-range magnitude, and asserts finiteness in debug builds.
- **The confinement claim is a test, not a convention.** `confinement_of_the_float_domain` reads every `.rs` file under `src/` in sorted path order and asserts only `numeric.rs` and `config.rs` may name a floating-point type, with `config.rs` narrowed line by line to the one restricted field D-11 permits. Whole-word matching, so a hex literal cannot fire it. Plan `01-07`'s lint wall enforces the method-level half; this enforces the module-level half.

## Task Commits

Each task was committed atomically; Tasks 1 and 2 carried `tdd="true"` and so have a RED and a GREEN commit.

1. **Task 1: Generational identity and an arena that respawns in place**
   - `032d4c8` (test) — 8 failing tests, `FirmArena` methods as `todo!()` stubs
   - `ba522b9` (feat) — the arena implementation; 8/8 pass
2. **Task 2: The confined float domain and the deterministic fractional power**
   - `214500c` (test) — 11 tests, 10 failing against `todo!()` stubs
   - `a9a309b` (feat) — `pow_frac_det`, `pow_frac`, `demand_to_units`, the three constants; 11/11 pass in debug and release
3. **Task 3: Integration tests, and prove the float domain is actually confined**
   - `9cee2e0` (docs) — the `src/rng.rs` deviation fix that Task 3's confinement test required
   - `3ea4307` (test) — both integration test files; 4 + 5 tests pass in debug and release

_No REFACTOR commit was needed: neither GREEN implementation had cleanup to do._

## Files Created/Modified

- `src/ids.rs` (created, 286 lines) — `HouseholdId`, `FirmSlot`, `GoodId`, `FirmId { slot, generation }`, `Account`, `FirmArena<T>` with `with_occupants`, `len`, `is_empty`, `get`, `get_mut`, `id_at`, `respawn_in_place`, `live_ids`, plus 8 unit tests.
- `src/numeric.rs` (created, 235 lines) — `POW_FRAC_BITS`, `PPM_SCALE`, `MILLI_SCALE`, `pow_frac_det`, `pow_frac`, `demand_to_units`, plus 11 unit tests.
- `tests/ids_generational.rs` (created, 96 lines) — 4 integration tests reaching `sim::ids` through the public surface.
- `tests/numeric_det.rs` (created, 179 lines) — 5 integration tests including the source-scanning confinement assertion.
- `src/lib.rs` (modified) — `pub mod ids;` and `pub mod numeric;` added; all five modules now declared.
- `src/rng.rs` (modified, doc comment only) — see Deviation 2.

## Decisions Made

- **The generation field is `generation`, not `gen`.** Forced by the toolchain; see Deviation 1. The requirement text, the research pattern and CONTEXT.md all write `FirmId { slot, gen }`, and the type shape they describe is unchanged — only the spelling differs.
- **`respawn_in_place` panics on an out-of-range slot.** The arena is fixed-length for the life of a run, so an out-of-range slot is a program defect and not a runtime condition worth a `Result`. `get`, `get_mut` and `id_at` still return `None` for an out-of-range slot, because those take caller-supplied identities.
- **The arena has no vacancy concept.** Every slot is occupied for the whole run, so `live_ids()` always returns exactly one identity per slot and its length equals the slot count. Phase 10 replaces occupants; it never empties a slot.
- **`src/numeric.rs` avoids the substrings `powf`, `exp`, `ln` and `log` entirely — including in prose.** The plan's acceptance criterion states the module must contain none of them. Rather than argue that "exponent" and "logged" are prose and not calls, the documentation was worded around them ("binary digits of the fractional power", "written to the run record"). A mechanical check that needs no exception is a check that survives.
- **The confinement allowlist is file-level and comment-blind.** A floating-point type named in a doc comment counts as naming it. This is stricter than a call-site scan would be, and deliberately so: a heuristic that skips comments is a heuristic someone will later widen to skip a string, then a macro.
- **Every mechanical claim was confirmed by mutation and reverted.** Three mutations, all three producing the expected failure: the generation check, the bit count, and a float in `src/money.rs`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `gen` is a reserved keyword in Rust edition 2024**

- **Found during:** Task 1 (Generational identity and an arena that respawns in place)
- **Issue:** The plan, `01-RESEARCH.md` Pattern 5, `01-CONTEXT.md` D-03 and REQUIREMENTS.md CORE-06 all specify the field as `FirmId { slot, gen }`. `gen` was reserved in edition 2024 for generator blocks and does not parse as an identifier. Verified by compiling a minimal `pub struct FirmId { pub slot: u16, pub gen: u32 }` under `--edition 2024`: `error: expected identifier, found reserved keyword `gen``, with rustc suggesting the `r#gen` escape.
- **Fix:** Named the field `generation`. The alternative — `r#gen` — compiles, but would put a raw identifier at every construction site and every field access across the ten remaining phases, in a codebase whose whole point is that later phases read cleanly. The type shape, the derives, the derived total order on `(slot, generation)` and the log identity are all unchanged, so no requirement's substance moves. The rationale is recorded as a module-level doc comment on `src/ids.rs` so the next reader looking for `gen` finds the answer immediately.
- **Files modified:** `src/ids.rs`
- **Verification:** All 8 `ids::` unit tests and 4 `ids_generational` integration tests pass. The plan's acceptance grep for the field `pub gen` still matches, because `pub generation:` contains it as a prefix — the check passes honestly rather than by coincidence of wording.
- **Committed in:** `032d4c8` / `ba522b9`

**2. [Rule 3 - Blocking] `src/rng.rs` named a floating-point type in a doc comment**

- **Found during:** Task 3 (Integration tests, and prove the float domain is actually confined)
- **Issue:** `src/rng.rs:209` read "dispatches between three algorithms on `f32` thresholds". That is the only floating-point type name under `src/` outside the numeric module, and it would have failed `confinement_of_the_float_domain`, whose allowlist the plan specifies at file granularity ("contains a floating-point type name **anywhere**").
- **Fix:** Reworded to "single-precision thresholds", preserving the meaning exactly, with a parenthetical noting this is the same discipline the surrounding paragraph already applies to `rand`'s sampler identifiers, which it deliberately keeps out of the file so a grep cannot return a false positive. The alternative — teaching the test to skip comment lines — was rejected: it would weaken the check permanently to accommodate one sentence.
- **Files modified:** `src/rng.rs` (doc comment only, no behaviour change)
- **Verification:** `grep -rnE '\bf(16|32|64|128)\b' src/ | grep -v numeric.rs` returns nothing. The full `determinism_rng` suite (13 tests) and the whole test suite pass unchanged in debug and release.
- **Committed in:** `9cee2e0`

---

**Total deviations:** 2 auto-fixed (2 × Rule 3 - blocking issues).
**Impact on plan:** Both were prerequisites for the plan's own acceptance criteria, not scope creep. Deviation 1 is a naming change forced by the language edition and is the one thing in this plan a reviewer should confirm they are happy with, since `generation` now propagates into the Phase 3 log schema and every later phase that touches a firm. Deviation 2 is a doc-comment reword with no behaviour change. No plan behaviour was skipped or weakened.

## Issues Encountered

None. Every behavior line in both TDD tasks was validated against a scratch implementation before being written into the module, so no fix-attempt budget was consumed.

Two notes for the record:

- **`cargo fmt --check` still fails on `src/money.rs` and `tests/tracer_end_to_end.rs`.** That is the pre-existing deferred item owned by plan `01-07`; those two files were deliberately not reformatted. Every file this plan touched is `rustfmt`-clean, verified individually with `rustfmt --check --edition 2024`.
- **`cargo clippy --all-targets --all-features -- -D warnings` is clean** across the whole crate after this plan, with no `allow` attribute anywhere in the new code.

## Verification Results

Plan-level `<verification>` block, all re-run at close:

| Check | Result |
|---|---|
| `cargo test` (debug, whole crate) | 55 lib + 31 integration tests, **0 failed** |
| `cargo test --release` (whole crate) | 53 lib + 31 integration tests, **0 failed** |
| `cargo test --lib ids::` | 8 passed |
| `cargo test --lib numeric::` (debug and release) | 11 passed each |
| `cargo test --test ids_generational` | 4 passed |
| `cargo test --test numeric_det` (debug and release) | 5 passed each |
| Float type names under `src/` outside `numeric.rs` | 0 |
| Storage-reordering removal in the arena | 0 (`swap_remove`, `remove(`, `retain`, `drain`, `truncate`, `pop(`) |
| `src/lib.rs` module declarations | 5 (`config`, `ids`, `money`, `numeric`, `rng`) |
| `grep -c 'powf' src/numeric.rs` | 0 |
| `grep -c 'allow(clippy' src/numeric.rs` | 0 |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |

## Known Stubs

None. No `todo!`, `unimplemented!`, `TODO`, `FIXME` or placeholder remains in any file this plan created — the two `todo!()` bodies used to establish the TDD RED state were both replaced in their GREEN commits and are absent from `HEAD`. No test is `#[ignore]`d, and every `<verify>` command in the plan was run.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `01-06` (the config loader). Two handoffs matter:

- **`01-06` owns `src/config.rs`.** `confinement_of_the_float_domain` already allows that file to name a floating-point type, but only on lines that also contain `expected_demand`. A config struct that declares the demand field as a float passes; one that lets a float leak into any other field fails immediately. The test is written so that it passes today, before `01-06` lands.
- **`01-08` owns the other half of CORE-10's carve-out.** `POW_FRAC_BITS`, `PPM_SCALE` and `MILLI_SCALE` ship here as documented code constants with their in-code rationale; the matching `GRADE: PROJECT` entry in `config/PROVENANCE.md` is `01-08`'s. CORE-10 is declared by both this plan and `01-06`, so it should not read Complete until both have summaries.

One item for the verifier, flagged while reversal is still cheap: **the `gen` → `generation` rename** (Deviation 1). It is forced by the language edition, but it diverges in spelling from CORE-06, `01-RESEARCH.md` Pattern 5 and `01-CONTEXT.md` D-03, and it propagates into the Phase 3 log schema. A human should confirm the rename rather than discover it in Phase 10.

## Self-Check: PASSED

- All 4 created files present on disk (`src/ids.rs`, `src/numeric.rs`, `tests/ids_generational.rs`, `tests/numeric_det.rs`); both modified files present (`src/lib.rs`, `src/rng.rs`).
- All 6 task commits found in `git log`: `032d4c8`, `ba522b9`, `214500c`, `a9a309b`, `9cee2e0`, `3ea4307`.
- Working tree clean before the metadata commit; no unintended deletions (`git diff --diff-filter=D` empty across every task commit) and no untracked files.
- All three plan mutations executed and reverted; `git diff` confirms `src/money.rs` byte-identical to its pre-mutation state.

---
*Phase: 01-primitives-and-the-determinism-spine*
*Completed: 2026-08-30*
