---
gsd_state_version: 1.0
current_phase: 01
current_phase_name: Primitives and the Determinism Spine
status: executing
stopped_at: Completed 01-08-PLAN.md
last_updated: "2026-08-31T00:14:13.632Z"
last_activity: 2026-08-30
last_activity_desc: Phase 01 execution started
state_head: e726b3274cc0bc990e6a5a59d2d2439e65488697
progress:
  total_phases: 11
  completed_phases: 0
  total_plans: 8
  completed_plans: 7
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-08-30)

**Core value:** The daily tick loop must be provably correct and demonstrably alive — money conserved to the cent, runs byte-identically reproducible, and an economy that fluctuates rather than pinning or spiralling.
**Current focus:** Phase 01 — Primitives and the Determinism Spine

## Current Position

Phase: 01 (Primitives and the Determinism Spine) — EXECUTING
Plan: 8 of 8
Status: Ready to execute
Last activity: 2026-08-30 — Phase 01 execution started

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P01 | 11 min | 3 tasks | 12 files |
| Phase 01 P02 | 4 min | 4 tasks | 3 files |
| Phase 01 P03 | 5 min | 3 tasks | 3 files |
| Phase 01 P04 | 10 min | 3 tasks | 3 files |
| Phase 01 P05 | 10 min | 3 tasks | 6 files |
| Phase 01 P06 | 4min | 3 tasks | 3 files |
| Phase 01 P08 | 26 min | 3 tasks | 3 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Roadmap]: Cadence is a 21-day month — every published parameter used verbatim at source grade, no daily/weekly rate conversions anywhere. Research SUMMARY.md §C conversion tables are superseded.
- [Roadmap]: Phases 1–4 contain zero economics by design — ledger, invariants, tick pipeline and log schema precede any economic rule, so each rule is born under the check.
- [Roadmap]: Dividends ship with firm accounting in Phase 8, never split — the only cycle-closing flow in a bankless economy.
- [Roadmap]: Tick order is not build order — firm planning runs first in the tick, built ninth.
- [Roadmap]: 5 of 8 HARN requirements moved out of Phase 4 to the phases whose gates they convert into automated checks (9, 10, 11).
- [Phase 01]: CORE-03 restated as two clauses: absence applies only to StdRng/SysRng (what the rand 0.10.2 feature set genuinely removes); SmallRng/Xoshiro are banned by USE via clippy disallowed-types plus a source grep, because absence is unachievable without forking rand
- [Phase 01]: CORE-11 clause (b), the Lengnick paper verification, is gated on Phase 6 per D-19 and added as Phase 6 criterion 6; CORE-11 stays mapped to Phase 1 to preserve the one-requirement-one-phase invariant. Deferred, not dropped.
- [Phase 01]: CORE-10 scope narrowed to simulation/economic parameters; POW_FRAC_BITS, PPM_SCALE and MILLI_SCALE stay consts in src/numeric.rs with a GRADE: PROJECT entry in config/PROVENANCE.md, so a numerical-method constant is never tunable as an economics parameter
- [Phase 01]: Money uses a split overflow API (D-07): the operators panic in every build profile because each routes through an i64::checked_* primitive, while the named checked_add/checked_sub/try_scale return Result<Money, MoneyOverflow> for config ingestion. Both halves ship; neither substitutes for the other.
- [Phase 01]: Money::split distributes the remainder to the first |amount mod n| recipients by ascending index, on both signs, and panics on n == 0 rather than returning an empty vector. Phase 2 LEDG-03 and Phase 8 OWN-06 are written against this rule; changing it later would alter every committed run trajectory.
- [Phase 01]: try_scale multiplies before dividing so the ratio keeps full precision in the integer domain with no intermediate rounding and no float; it truncates toward zero on both signs and reports a zero denominator as Err.
- [Phase 01]: The plain-+ mutation cannot prove the D-07 operator half under this repo profile: with [profile.release] overflow-checks = true a bare + still panics with a message containing "overflow". Only a truly unchecked operation (wrapping_add) distinguishes the two belts, which is precisely why the .expect on checked_* is not redundant with CORE-02.
- [Phase 01]: Sub-stream key layout LOCKED at tick:24 | agent:24 | purpose:16. One-way door — re-keying invalidates every future golden log and insta snapshot. Confirms CONTEXT.md D-01/D-02. CONFIRMED BY THE USER during phase-1 execution, in answer to the 01-04 checkpoint surfaced by the orchestrator; the earlier note that this was resolved from the project record rather than a fresh human answer no longer applies, and SUMMARY coverage item D6 is closed. Reversal was still cheap at the time of confirmation (no golden log or insta snapshot existed yet).
- [Phase 01]: Checkpoint policy set by the user during phase-1 execution: executors resolve blocking checkpoint gates themselves using the plan's recommended option plus locked CONTEXT.md decisions, then report the call in their SUMMARY for after-the-fact review. This is the behaviour 01-04 exhibited, now ratified. Deliberately NOT implemented by setting workflow.auto_advance=true — that boolean also auto-chains one phase into the next, which the user did not ask for; the preference is recorded here instead so it survives a context reset.
- [Phase 01]: Purpose discriminants are append-only and gapped per subsystem (10/11 activation, 20/21 labour, 30/31 goods, 40-43 price+wage, 50 bankruptcy). A later phase adding a draw site appends a variant AND must add it to ALL_PURPOSES, or the injectivity sweep silently stops covering it.
- [Phase 01]: The RNG re-entry guard is debug-only by design — a decade-long run opens millions of sub-streams and a release-build BTreeSet of issued keys would grow without bound. Ordered set, never hashed (CORE-07).
- [Phase 01]: getrandom IS in the dependency graph, but only via the proptest dev-dependency (rand 0.9 and tempfile). The correct CORE-03 check is 'cargo tree --edges normal', already asserted in tests/toolchain.sh from plan 01-01 — a bare 'cargo tree | grep getrandom' produces a false failure.
- [Phase 01]: The generational field is spelled `generation`, not `gen`: `gen` is a reserved keyword in Rust edition 2024 and does not parse as an identifier (verified by compile error). The `r#gen` escape would force a raw identifier at every construction and field access for ten more phases. Type shape, derived total order on (slot, generation) and the log identity are unchanged, but the spelling now diverges from CORE-06, 01-RESEARCH.md Pattern 5 and D-03 and propagates into the Phase 3 log schema — flagged for human confirmation at verify-phase.
- [Phase 01]: FirmArena exposes no element-removal operation at all — not swap_remove, remove, retain, drain, truncate or pop. respawn_in_place is the only mutation of the slot vector and changes no index and no length, so BANK-03 is enforced by absence rather than by review. The arena has no vacancy concept: live_ids always returns one identity per slot.
- [Phase 01]: src/numeric.rs contains no occurrence of the substrings powf, exp, ln or log anywhere, including in prose — its documentation is deliberately worded around them so the mechanical grep for the banned float methods needs no comment-stripping heuristic and no exception to be argued with.
- [Phase 01]: The float-confinement test allowlist is file-level and comment-blind: a floating-point type named in a doc comment counts. src/rng.rs was reworded rather than the test loosened, because a heuristic that skips comments is one someone later widens to skip a string, then a macro. This is the module-level half of the float ban; 01-07 lint wall is the method-level half, and neither is sufficient alone.
- [Phase 01]: MoneyRange range check is stock.checked_add(stock): the money stock must survive doubling, giving the conservation audit headroom and turning an absurd amount into a named ConfigError rather than a panic (T-1-03)
- [Phase 01]: Config strictness is proved by deletion, not by grep: every_key_is_required removes each of the 41 leaf keys in turn and asserts each is rejected by name (Pitfall 7)
- [Phase 01]: Ratios and probabilities enter the config as parts-per-million integers; initial_expected_demand stays the single float in the whole configuration (D-11/D-13)
- [Phase 01]: The config hash is over raw file bytes, proved sensitive to a table reorder and to one comment character, because the comments carry the source grades CORE-11 makes load-bearing
- [Phase 01]: Grade-B provenance rows are marked UNVERIFIED by grade, not by paper name — the no-silent-upgrade test keys off grade B so the BAM rows are held to the same honesty as the Lengnick rows — Grade B is defined as "an annotated replication citing the paper's table/equation numbers", which IS the unverified condition. Keying the test off the string "Lengnick" would have left the two equally-unread BAM rows free to be upgraded silently.
- [Phase 01]: 21 config keys are attributed to the baseline-model paper, not the 18 stated in CONTEXT.md D-19 — the counts measure graded-table rows vs config keys and both are correct; Phase 6 works from 21 — One graded row can expand into two config keys (P(price search) / P(rationing search) is one row and two keys), and several graded rows describe rules rather than parameters and have no key at all. The key count is the set a person must actually check. Reconciliation recorded in config/PROVENANCE.md section 2.

### Pending Todos

None yet.

### Blockers/Concerns

- Lengnick Table 1 values are grade B (from an annotated replication, not read from the paper) — verification is an explicit Phase 1 task (CORE-11). It de-risks the widest-sensitivity parameter group in the model.
- Phases 6, 8 and 11 are research-flagged for `--research-phase` at planning time. Phase 1 carries a light flag (RNG sub-stream keying; `f64` vs `i64` milli-units for `expected_demand`).
- All initial conditions and the total money stock are unspecified in every source — deliberately deferred to Phase 11, which means Phases 5–10 run on provisional placeholder values.
- V-4 (Phase 6): the sense of theta=0.75 is contradictory — the graded table reads it as P(firm CONSIDERS a price change) while the shipped key is named price_inaction_prob_ppm, the complementary event. Which reading is right changes price-move frequency threefold. Flagged in config/PROVENANCE.md open item V-4, not corrected from memory per D-20.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

### Gate overrides (Phase 1 planning)

- **Decision coverage gate (`check.decision-coverage-plan`) — overridden at plan time, 2026-08-30.**
  The gate returned `passed: false, reason: "could-not-parse"`. This is a parser limitation, not a
  coverage gap: its per-bullet regex reads a single line and rejects a title containing `*` or more
  than one `:`, so three CONTEXT.md bullets are unreadable to it — D-09 (`Money::split(n)`, extra
  colons in inline code), D-23 (`*generated*`) and D-26 (`*effective*`). Nine further bullets whose
  bold title had wrapped onto a second line were fixed in `bf48000` (whitespace only), taking the
  parser from 16 to 23 of 26. On a `could-not-parse` outcome the handler emits a hard-coded
  `covered: 0` and never counts, so it cannot certify coverage either way.
  Coverage was instead established directly: all 26 decisions (D-01 … D-26) are cited across the
  eight PLAN.md files, confirmed by grep and independently by the gsd-plan-checker on both of its
  passes. The three remaining bullets were left as written — editing the prose of locked user
  decisions to satisfy a regex is the wrong trade. Re-surface at verify-phase.

## Session Continuity

Last session: 2026-08-31T00:13:31.310Z
Stopped at: Completed 01-08-PLAN.md
Resume file: None
