---
phase: 02-books-journal-and-invariants
plan: 05
subsystem: testing
tags: [negative-test, fault-injection, cfg-test, localisation, halt, invariants, books]

# Dependency graph
requires:
  - phase: 02-books-journal-and-invariants
    plan: 04
    provides: "`check_non_negative`/`check_zero_sum`, the five-entry `ALL_CHECKS`, `ZeroSumDetail`'s eight shapes, `Violation::Negative`/`ZeroSum`, `Books::accounts`, `PostError::EmptyExchange`"
  - phase: 02-books-journal-and-invariants
    plan: 03
    provides: "`Posting`'s two units legs, the goods residual, `Violation`'s boxed optional posting"
  - phase: 02-books-journal-and-invariants
    plan: 02
    provides: "`Books`, the private recorder `Books::record`, `CheckSet`/`CheckId`/`CheckFn`, `first_breaking_cash_posting`, the public-API halt proof in `tests/invariant_halt.rs`"
  - phase: 01-primitives-and-the-determinism-spine
    provides: "`Account`/`FirmId`/`FirmSlot`/`HouseholdId` and their `Display`, `Money`, the release profile's overflow checks"
provides:
  - "`sim::books::Books::corrupt_recorded_cash` — an unbalanced recorded posting with a signed cash delta, routed through the production recorder (test configuration, crate visibility)"
  - "`sim::books::Books::corrupt_silent_cash` — a cash adjustment that records nothing, so no posting can be named"
  - "`sim::books::Books::corrupt_conserving_deficit` — a conserving move that drives one account below zero"
  - "`sim::books::Books::corrupt_appended_posting` — an arbitrary posting appended with no balance change"
  - "`books::corrupt` — five tests proving each corruption's documented effect in isolation from any invariant"
  - "`invariants::negative` — seven tests: one per violation class, the seeded-leak tick-loop halt, and its no-corruption control"
  - "`invariants::localise` — the measured cancelling-residual case and the monotone case"
  - "`invariants::message` — one test per violation variant plus the no-path assertion, over eight zero-sum detail shapes"
affects: [02-06, 02-07, phase-03-tick-pipeline, phase-05-production, phase-07-goods-market, phase-10-bankruptcy]

actuals:
  tokens: 12241
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Fault injection as `#[cfg(test)] impl` methods with `pub(crate)` visibility — compiler-enforced absence from every non-test build, no cargo feature, no runtime flag, no production hole to prove shut"
    - "Every recorded corruption routes through the same private recorder a real posting uses, so a seeded fault exercises the production residual arithmetic rather than a hand-faked number"
    - "Each corruption carries its own unit test proving its documented effect, so a broken corruption cannot produce a green negative test for the wrong reason"
    - "Violations asserted by whole-value equality; message substring assertions confined to the module whose subject is the message contract"
    - "Compile-checked exhaustiveness: a `position` match per enum, asserted to cover 0..N, so a new variant stops the test module compiling rather than going untested"

key-files:
  created: []
  modified:
    - "src/books.rs — the four `corrupt_*` methods behind the crate's test configuration, plus `mod corrupt`"
    - "src/invariants.rs — `mod negative`, `mod localise`, `mod message`"

key-decisions:
  - "The corruption vocabulary is gated on the crate's test configuration with crate visibility. A cargo feature was rejected: its only gain is reachability from `tests/`, and it costs a features entry, a second CI invocation and a standing assertion that the feature stayed out of the default set — a production hole that must be proved shut on every run."
  - "`corrupt_recorded_cash` takes a base amount as well as a delta, so its posting is a realistic over/under-credited transfer (100 out, 105 in) rather than a degenerate zero-legged one. That makes the `CashLegsDiffer` detail name two real amounts and makes the one-fault-two-checks ordering test read honestly."
  - "The corruptions that write balances directly (`corrupt_silent_cash`, `corrupt_conserving_deficit`) record nothing, which is what produces the `posting: None` branch of the conservation and non-negativity violations."
  - "Both two-account corruptions assert their addresses differ: resolving one account twice would lose the first write and silently seed a different fault than the one asked for."
  - "The localisation test asserts the early posting by whole-value equality AND asserts explicitly against the late one, so the test states in its own text what a bisection would have answered."

patterns-established:
  - "Negative-test discipline: a check never observed to fire has never been shown to work — every violation class in this crate now has a seeded fault that produces it"
  - "Halt proof has two separate claims: `assert_eq!` on the returned error AND an out-parameter recording the last tick whose body began. 'It returned an error' and 'it stopped' are not the same claim."
  - "Mutation-check a localisation claim before trusting it: replacing the forward scan with a reverse one must fail the test, or the test is not measuring what it says"

requirements-completed: [LEDG-06, LEDG-07, LEDG-09]

coverage:
  - id: D1
    description: "A dropped cent recorded as a posting halts the run as a money-conservation violation naming the offending posting"
    requirement: "LEDG-09"
    verification:
      - kind: unit
        ref: "src/invariants.rs#invariants::negative::a_dropped_cent_recorded_as_a_posting_is_reported_as_a_leak_and_localised"
        status: pass
    human_judgment: false
  - id: D2
    description: "A drop written outside the posting path reports no offending posting and says so honestly in the rendered message"
    requirement: "LEDG-09"
    verification:
      - kind: unit
        ref: "src/invariants.rs#invariants::negative::a_drop_written_outside_the_posting_path_names_no_posting_and_says_so"
        status: pass
    human_judgment: false
  - id: D3
    description: "An over-credited posting is reported as a leak, the same books also break zero-sum, and the check table's order decides which the caller sees"
    requirement: "LEDG-07"
    verification:
      - kind: unit
        ref: "src/invariants.rs#invariants::negative::an_over_credited_posting_is_a_leak_and_the_same_books_also_break_zero_sum"
        status: pass
    human_judgment: false
  - id: D4
    description: "A conserving move that drives an account below zero fires non-negativity and NOT money conservation — the two checks proved independent in both directions"
    requirement: "LEDG-06"
    verification:
      - kind: unit
        ref: "src/invariants.rs#invariants::negative::a_conserving_move_that_drives_an_account_negative_is_not_a_conservation_failure"
        status: pass
    human_judgment: false
  - id: D5
    description: "A synthesised posting carrying units on a cash-only kind breaks only the structural check; both conservation identities still hold"
    requirement: "LEDG-07"
    verification:
      - kind: unit
        ref: "src/invariants.rs#invariants::negative::a_synthesised_posting_breaks_only_the_structural_check"
        status: pass
    human_judgment: false
  - id: D6
    description: "A seeded leak aborts a library tick loop at the tick it occurred and the loop does not begin the next tick; the identical loop without the leak runs every tick"
    verification:
      - kind: unit
        ref: "src/invariants.rs#invariants::negative::a_seeded_leak_aborts_the_tick_loop_at_the_tick_it_occurred"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#invariants::negative::the_identical_loop_with_no_seeded_leak_runs_every_tick"
        status: pass
    human_judgment: false
  - id: D7
    description: "The reported posting is the first non-conserving one across a residual that cancels — broken at 50, healed at 120, broken again at 200 — and the monotone case is proved separately"
    requirement: "LEDG-09"
    verification:
      - kind: unit
        ref: "src/invariants.rs#invariants::localise::the_first_break_is_reported_even_when_a_later_posting_heals_the_residual"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#invariants::localise::the_monotone_case_reports_the_only_break"
        status: pass
      - kind: other
        ref: "mutation check: replacing the forward scan with a reverse one made both tests fail, reporting posting 200 against an expected 50"
        status: pass
    human_judgment: false
  - id: D8
    description: "Every violation variant's rendered message names the tick, the agent's own rendered address and the posting, or says honestly that none accounts for the discrepancy; no message carries a path separator"
    requirement: "LEDG-09"
    verification:
      - kind: unit
        ref: "cargo test --locked --release --lib invariants::message (6 tests)"
        status: pass
    human_judgment: false
  - id: D9
    description: "The fault-injection vocabulary exists only in the crate's own test build — no cargo feature, no runtime flag, no builder switch, and Cargo.toml/Cargo.lock unchanged"
    verification:
      - kind: unit
        ref: "cargo build --locked --release (exit 0 with the gated block absent)"
        status: pass
      - kind: other
        ref: "git diff --exit-code -- Cargo.toml Cargo.lock (exit 0); grep -c '[features]' Cargo.toml == 0"
        status: pass
    human_judgment: false
    rationale: ""
  - id: D10
    description: "Each corruption method has its own unit test proving the documented effect on balances, journal length and running residual"
    verification:
      - kind: unit
        ref: "cargo test --locked --release --lib books::corrupt (5 tests)"
        status: pass
    human_judgment: false

# Metrics
duration: 12min
completed: 2026-08-31
status: complete
---

# Phase 2 Plan 05: The Negative Tests Summary

**Four seeded corruptions behind `#[cfg(test)]` — no cargo feature, no production hole — driving all five checks to fire, the tick loop to abort at the corrupted tick, and localisation to name posting 50 across a residual that heals at 120 and breaks again at 200.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-08-31T09:57:46Z
- **Completed:** 2026-08-31T10:09:25Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- **The phase gate is real.** Every one of the four violation classes has now been *observed* to fire on a deliberately seeded fault, asserted by whole-value equality including the tick, the numbers and the posting. ROADMAP Phase 2 criterion 2 — "the negative test passes for every check" — is discharged at the library level for the classes the public API cannot reach; `tests/invariant_halt.rs` already covers the one class it can.
- **Conservation and non-negativity are proved independent in both directions.** A conserving move that drives household 0 to −250 leaves the books holding exactly the configured 2,000,000 cents, so money conservation and goods conservation both return `Ok(())` and only `check_non_negative` fires. This is the single most valuable assertion in the plan and it now runs on every build.
- **One fault trips two checks, and the table's order decides which one a caller sees.** An over-credited posting is both non-conserving and malformed. `check_money` and `check_zero_sum` each fire on the identical books, and `CheckSet::run` reports the leak — because money conservation is first and reporting this as "a posting is malformed" would send a debugger to the shape of the posting rather than to the missing money.
- **The localisation claim was falsifiable and survived.** A journal broken at posting 50, healed at 120 and broken again at 200 reports posting 50. The test asserts the early posting *and* asserts against the late one. Replacing the forward scan with a reverse one — the mutation a bisection would resemble — made both localisation tests fail, reporting posting 200 against an expected 50. The test measures what it says it measures.
- **A seeded leak stops the loop, not just returns an error.** The tick loop aborts at tick 3 with the exact `MoneyConservation` value including the corruption's posting (sequence 1, cumulative residual −1), and an out-parameter proves the body never began tick 4. A control run with the corruption removed reaches tick 9 and returns `Ok(())`.
- **Zero production surface.** Four `pub(crate) fn corrupt_*` methods behind the crate's test configuration. `cargo build --locked --release` compiles with the block absent; `Cargo.toml` and `Cargo.lock` are byte-unchanged; there is no `[features]` table to audit and no flag whose state must be asserted.

## Task Commits

1. **Task 1: A corruption vocabulary the compiler keeps out of every non-test build** — `aa4d45a` (test)
2. **Task 2: The negative tests — every check observed to fire, localised, and named** — `cbb403b` (test)

## Files Created/Modified

- `src/books.rs` — four `corrupt_*` methods in a `#[cfg(test)] impl Books` block (an unbalanced recorded posting with a signed cash delta; a silent cash adjustment with no posting; a conserving move that drives an account negative; an arbitrary appended posting), plus `mod corrupt` with five tests proving each one's documented effect on balances, journal length and running residual.
- `src/invariants.rs` — `mod negative` (7 tests), `mod localise` (2 tests), `mod message` (6 tests).

## Decisions Made

- **`corrupt_recorded_cash` takes a base amount as well as a delta.** A zero-legged corruption would have produced `CashLegsDiffer { debit_cents: 0, credit_cents: 1 }` — expressible but degenerate. Taking `(cents, delta_cents)` makes the seeded posting a realistic over/under-credited transfer (100 out, 105 in), which is what makes the one-fault-two-checks test read as the ordering contract rather than as a curiosity.
- **The two direct-write corruptions record nothing.** That is what produces the `posting: None` branch on both `MoneyConservation` and `Negative`, and it keeps "written outside the posting path" literally true rather than approximately so.
- **Both two-account corruptions assert their addresses differ.** Resolving one account twice would apply the debit write and then overwrite it with the credit write, silently seeding a different fault than the one the test asked for — a corruption that lies is worse than no corruption.
- **`check_for(CheckId)` looks the check up in `ALL_CHECKS` by identifier** rather than calling the private function directly, so every negative test drives the function the production check set dispatches to. A check removed from the table fails here rather than continuing to pass in isolation.
- **Message substring assertions live in `mod message`, with one documented exception.** The silent-corruption test in `mod negative` also reads its message, because the honest phrasing when no posting accounts for the discrepancy is the entire reason that field is optional rather than a placeholder — and the plan's action text mandates it. Its primary assertion is still whole-value equality. `grep -c 'to_string().contains' src/invariants.rs` reports 0.
- **The halt test spells out the expected posting as a literal** rather than capturing it out of the loop. Sequence 1 (the tick's own transfer took 0) and a cumulative residual of −1 (three clean ticks left it at zero) are claims about the recorder that the test now states rather than accepts.

## Deviations from Plan

None — plan executed exactly as written.

Three items from the plan's own text were followed rather than deviated from, and are noted because they contradict earlier phase drafts:

- `ZeroSumDetail` ships **eight** shapes, not the six an earlier draft listed. `mod message` asserts `DETAIL_SHAPES == 8` and covers each by a compile-checked position match (ledger entry 10 discharged for this plan's purposes).
- `Violation` carries `Option<Box<Posting>>`; every construction here boxes (ledger entry 6).
- `Books::exchange` refuses an empty leg; no corruption relies on the old permissive behaviour (`EmptyExchange` appears only as a rendered detail in `mod message`, never as a seeded fault).

**Total deviations:** 0
**Impact on plan:** None.

## Issues Encountered

- **A tautological assertion caught before commit.** `mod corrupt`'s appended-posting test originally read `assert_eq!(books.total_stock(FOOD), books.total_stock(FOOD))` — an assertion that cannot fail, and one `clippy::eq_op` would have rejected under `-D warnings`. Replaced with a stock reading captured before the corruption. Caught by re-reading the draft, not by a tool.
- **The localisation test needed proving falsifiable.** A passing localisation test proves nothing unless the wrong scan fails it. Temporarily replacing `first_breaking_cash_posting`'s `.find(…)` with `.rev().find(…)` made both `localise` tests fail with the documented signature (reported posting 200, expected 50; and 20 against 10 in the monotone case). Reverted immediately; the mutant is not committed.

## Verification Run

All green, both profiles:

- `cargo test --locked --all-targets` — 158 lib + 62 integration tests pass
- `cargo test --locked --release --all-targets` — 156 lib + 62 integration tests pass (the two-test difference is pre-existing: `src/rng.rs` carries two debug-build-only tests, untouched by this plan)
- `cargo build --locked --release` — exit 0 with the corruption block absent
- `bash tests/lints.sh` — all four checks, 60 method bans firing
- `bash tests/toolchain.sh` — lockfile, toolchain and release overflow checks intact
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `cargo fmt --check` — clean
- `git diff --exit-code -- Cargo.toml Cargo.lock` — exit 0
- `grep -c 'pub(crate) fn corrupt' src/books.rs` — 4
- `grep -cE '\bf16\b|\bf32\b|\bf64\b|\bf128\b'` on both ledger modules — 0

Per-module counts against the plan's acceptance criteria: `books::corrupt` 5 (≥ 4), `invariants::negative` 7 (≥ 6), `invariants::localise` 2 (≥ 2), `invariants::message` 6 (one per violation variant plus the no-path assertion).

## Requirements

`LEDG-06`, `LEDG-07` and `LEDG-09` are marked complete. `LEDG-04` and `LEDG-10` appear in this plan's frontmatter but are **still owed by sibling plans** — `requirements ready-ids` reports both blocked, `02-07` owes LEDG-04 (the property tests over arbitrary operation sequences) and `02-06` owes LEDG-10 (the executed compile-fail probe and the source-level guards). They are deliberately left unmarked.

## Known Stubs

None. Every test written in this plan runs and asserts; no `#[ignore]`, no `todo!()`, no placeholder.

## User Setup Required

None.

## Next Phase Readiness

- **02-06 inherits a concrete claim to probe.** This plan's `#[cfg(test)] impl Books` block is exactly what 02-06's compile-fail probe must show is unreachable from `tests/`. The block's doc comment states the fact and names 02-06 as the plan that executes it.
- **02-06's source-level guards must distinguish two configuration predicates.** The block added here uses the crate's **test** predicate, which is permitted. The guard 02-06 adds is over the **debug-build** predicate and the debug-only assertion macro, neither of which is named anywhere in `src/books.rs` or `src/invariants.rs`. A guard written as a bare grep for "cfg(" would fire on this block.
- **02-07's property tests can reuse the corruption vocabulary if they need an adversarial ledger**, but should not need to: its subject is what *no sequence of public operations* can do, and the corruptions are by construction outside that set.
- **Phase 3 owns the process level of ROADMAP criterion 2** — a non-zero exit with the message on stderr — as the phase research records. Nothing in this plan blocks it: `Violation` is a library type and `main.rs` can propagate it.
- No blockers.

---
*Phase: 02-books-journal-and-invariants*
*Completed: 2026-08-31*

## Self-Check: PASSED

- Files claimed created/modified exist: `.planning/phases/02-books-journal-and-invariants/02-05-SUMMARY.md`, `src/books.rs`, `src/invariants.rs`
- Commits claimed exist in git history: `aa4d45a`, `cbb403b`
- The 20 tests this plan added run and pass in the release profile
