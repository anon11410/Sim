---
phase: 02-books-journal-and-invariants
plan: 06
subsystem: testing
tags: [rust, clippy, compile-fail, catch_unwind, proptest, lints, ledger, ledg-02]

requires:
  - phase: 02-books-journal-and-invariants
    provides: "Books, the journal, the four public ledger operations and the private recorder (02-02, 02-03)"
  - phase: 02-books-journal-and-invariants
    provides: "the invariant pipeline, CheckSet::from_params and the liveness gate (02-03, 02-04)"
  - phase: 02-books-journal-and-invariants
    provides: "the pub(crate) fault-injection vocabulary and the four seeded corruptions (02-05)"
  - phase: 02-books-journal-and-invariants
    provides: "the tick-boundary property whose residual clause could not fail, and the WINDOWS entry naming this plan (02-07)"
provides:
  - "tests/ledger_atomicity.rs — panic-atomicity over seven refusal paths, plus the naive mutant that fails the same harness"
  - "tests/lint-probes/books_borrow_probe.rs.txt — the shared-borrow-across-mutation compile-fail probe (E0502)"
  - "tests/lint-probes/books_cfg_test_probe.rs.txt — the fault-injection reachability compile-fail probe (E0599)"
  - "tests/lints.sh checks 5, 6 and 7 — two executed probes and eight source guards, each with a positive control"
  - "clippy.toml — eight shared-mutability and reference-counted bans, with the two exclusions the clean tree forces recorded and explained"
  - "src/books.rs — the tick-boundary residual test with teeth, discharging WINDOWS entry 12"
affects: [03-world-and-tick-loop, 06-labour-market, phases adding a proptest strategy, phases adding a Books method]

actuals:
  tokens: 13453
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Fixture-first grep guards: a pattern is proved to match a hazard fixture, and to leave a permitted lookalike alone, before it is asserted absent from the tree"
    - "Compile-fail probes assert the DIAGNOSTIC CODE, not a bare build failure"
    - "A positive test ships next to the mutant it discriminates against, in the same file"

key-files:
  created:
    - tests/ledger_atomicity.rs
    - tests/lint-probes/books_borrow_probe.rs.txt
    - tests/lint-probes/books_cfg_test_probe.rs.txt
  modified:
    - tests/lints.sh
    - clippy.toml
    - src/books.rs

key-decisions:
  - "std::sync::Arc could not join the disallowed-types list: proptest's prop_oneof! expands to code naming it, 9 diagnostics across 7 call sites in tests/, and check 4b forbids a lint exemption. Covered by guard 7c instead, exactly as RefCell is."
  - "Guards 7e and 7h are scoped to the production half of src/invariants.rs, and 7e counts the qualified field read rather than the bare identifier — written literally as planned, both fail on the real tree."
  - "The mutant lives in the test file, not in a fixture directory: the discrimination claim is only readable if the two designs sit side by side."
  - "Guard 7d searches the raw files, comments included, on the same terms as the float-name rule in tests/numeric_det.rs — the way to say 'this is not a debug_assert' is to not write the token."

patterns-established:
  - "Every guard asserts its search set is non-empty before searching it (02-RESEARCH Pitfall 4)"
  - "A guard's failure message states the reason, not only the rule, so the next author reads it as an argument rather than as arbitrary"
  - "assert_fires / assert_ignores / assert_absent_in: the three-part discipline any new grep guard in this repo follows"

requirements-completed: [LEDG-01, LEDG-02, LEDG-10]

coverage:
  - id: D1
    description: "A transfer that cannot complete leaves the books exactly as it found them, over every refusal a caller can reach"
    requirement: LEDG-02
    verification:
      - kind: integration
        ref: "tests/ledger_atomicity.rs#an_overdraft_leaves_the_books_exactly_as_it_found_them (and six siblings: negative amount, unknown account, self-dealing, exchange short stock, consume short stock, unknown good)"
        status: pass
      - kind: integration
        ref: "cargo test --locked --release --test ledger_atomicity — 10 passed in both profiles"
        status: pass
    human_judgment: false
  - id: D2
    description: "The atomicity harness is shown to discriminate: the naive write-then-check ordering unwinds and leaves -400 against an opening 100 under the identical harness"
    requirement: LEDG-02
    verification:
      - kind: integration
        ref: "tests/ledger_atomicity.rs#the_naive_ordering_unwinds_and_corrupts_its_total_under_the_same_harness"
        status: pass
      - kind: integration
        ref: "tests/ledger_atomicity.rs#the_real_books_answer_the_mutant_case_by_returning_instead"
        status: pass
      - kind: other
        ref: "mutation check: making NaiveBooks compute-then-commit fails the discrimination test at the -400 assertion"
        status: pass
    human_judgment: false
  - id: D3
    description: "A shared borrow of the books held live across a mutation does not compile, and the borrow-conflict diagnostic is asserted rather than a bare build failure"
    requirement: LEDG-02
    verification:
      - kind: other
        ref: "bash tests/lints.sh — check 5, E0502 asserted"
        status: pass
      - kind: other
        ref: "control: deleting the probe's trailing use of the borrow makes check 5 fail with COMPILED"
        status: pass
    human_judgment: false
  - id: D4
    description: "The fault-injection vocabulary is a hard compile error from an integration test, proved by an executed probe"
    requirement: LEDG-10
    verification:
      - kind: other
        ref: "bash tests/lints.sh — check 6, E0599 asserted"
        status: pass
      - kind: other
        ref: "control: breaking the probe so it fails without E0599 makes check 6 fail on the missing code"
        status: pass
    human_judgment: false
  - id: D5
    description: "No method that borrows the books mutably takes a callback, and the ledger names no shared-mutability wrapper"
    requirement: LEDG-02
    verification:
      - kind: other
        ref: "bash tests/lints.sh — guards 7a, 7b, 7c"
        status: pass
      - kind: other
        ref: "controls: an impl Fn parameter, a RefCell accessor and an Arc mention each observed to fail their guard"
        status: pass
    human_judgment: false
  - id: D6
    description: "Neither ledger module names the debug-only assertion vocabulary, and the guard leaves the permitted cfg(test) predicate alone"
    requirement: LEDG-10
    verification:
      - kind: other
        ref: "bash tests/lints.sh — guard 7d, with a permitted fixture asserting cfg(test) is not a hit"
        status: pass
      - kind: other
        ref: "control: a debug_assert! in Books::end_of_tick observed to fail guard 7d, while the 5 cfg(test) blocks in src/books.rs stay silent"
        status: pass
    human_judgment: false
  - id: D7
    description: "Only the ledger writes a balance, no accessor returns a mutable reference to one, and one file reads the liveness gate once"
    requirement: LEDG-01
    verification:
      - kind: other
        ref: "bash tests/lints.sh — guards 7e, 7f, 7g"
        status: pass
      - kind: other
        ref: "controls: a second gate read, a balance identifier outside the ledger, a set_cash declaration and a &mut return each observed to fail their guard; firm_cash_total confirmed not a false hit"
        status: pass
    human_judgment: false
  - id: D8
    description: "No path, clock or process type can reach a halt message from the violation module's source"
    requirement: LEDG-10
    verification:
      - kind: other
        ref: "bash tests/lints.sh — guard 7h"
        status: pass
      - kind: other
        ref: "control: a PathBuf in CheckSet::from_params observed to fail guard 7h"
        status: pass
    human_judgment: false
  - id: D9
    description: "Ending a tick leaves a non-zero running residual of either kind untouched — the truth WINDOWS entry 12 recorded as unmet"
    requirement: LEDG-04
    verification:
      - kind: unit
        ref: "src/books.rs#books::tests::ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched"
        status: pass
      - kind: other
        ref: "mutation check, both profiles: zeroing both residual fields in end_of_tick fails this test (left 0, right 1) while the integration property stays green"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-08-31
status: complete
---

# Phase 02 Plan 06: LEDG-02's Four Legs Summary

**All four legs of LEDG-02 now carry evidence that was watched working: an executable panic-atomicity test standing next to the mutant that fails it, an E0502 borrow probe, an E0599 reachability probe, and eight fixture-first source guards — replacing a ROADMAP criterion ("a test observing the books mid-transaction is impossible to write") that could be asserted but never verified.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-08-31T10:25Z
- **Completed:** 2026-08-31T10:50Z
- **Tasks:** 3 (plus the extra work wave 6 assigned)
- **Files modified:** 6 (3 created, 3 modified)

## Accomplishments

- **Panic-atomicity is positive and executable.** `tests/ledger_atomicity.rs` drives seven refusal paths — overdraft, negative amount, unknown account, self-dealing, exchange short stock, consume short stock, unknown good — through `catch_unwind` + `AssertUnwindSafe`, asserting four separate claims each: it did not unwind, it returned the caller-visible refusal, every captured quantity is unchanged, and the journal grew by nothing. A commit control stops a ledger that refused everything from passing all seven.
- **The harness is shown to discriminate.** A private `NaiveBooks` with the opposite write ordering reproduces the measured −400 against an opening 100 through the identical harness, and a companion test runs the mutant's exact scenario against the real ledger, which returns `Err(Overdraft)` and ends at 100. Verified by mutation: making `NaiveBooks` compute-then-commit fails the discrimination test.
- **Two compile-fail probes, each asserting its diagnostic code.** Check 5 refuses a shared borrow held live across a transfer with E0502; check 6 refuses an integration test calling a fault-injection method with E0599. Both were controlled: tidying the borrow probe's trailing use makes check 5 fail with "COMPILED", and breaking the reachability probe so it fails for another reason makes check 6 fail on the missing code.
- **Eight source guards, none of them taken on trust.** Every guard proves its pattern matches the exact number of hazard lines in its own fixture before it is asserted absent, and where a lookalike is legitimate it proves the pattern leaves a permitted fixture alone. Every guard asserts its search set is non-empty. All eight were then injected against for real, one at a time, and each was observed to fail with its own message.
- **WINDOWS entry 12 discharged with teeth.** The tick-boundary residual truth now has a unit test that seeds a non-zero residual of both kinds with the corruption vocabulary; zeroing the residuals in `end_of_tick` fails it in both profiles while the integration property stays green — reproducing wave 6's finding from the other side.

## Task Commits

1. **Task 1: Panic-atomicity and its mutant** — `59effde` (test)
2. **Extra work assigned by wave 6: WINDOWS entry 12** — `98590df` (test)
3. **Task 2: Two compile-fail probes and the lint entries** — `39a320b` (test)
4. **Task 3: Eight source guards** — `5ad7344` (test)

**Plan metadata:** see the final `docs(02-06)` commit.

## Files Created/Modified

- `tests/ledger_atomicity.rs` (new, 447 lines) — the panic-atomicity tests, the snapshot type they compare, and the `NaiveBooks` mutant with the measured numbers in its docs
- `tests/lint-probes/books_borrow_probe.rs.txt` (new) — the E0502 probe, with the load-bearing trailing use documented in the file itself
- `tests/lint-probes/books_cfg_test_probe.rs.txt` (new) — the E0599 probe
- `tests/lints.sh` (+408/−3) — checks 5, 6 and 7; the `assert_fires`, `assert_ignores`, `assert_absent_in` and `production_source` helpers; the required-input loop, the cleanup trap, the header comment and the success line all extended
- `clippy.toml` (+43) — eight new `disallowed-types` entries and the comment recording the two exclusions
- `src/books.rs` (+79) — one unit test in `mod tests`; no production code changed

## Decisions Made

- **`std::sync::Arc` is excluded from `clippy.toml`, for the same class of reason `RefCell` is.** It was added as the plan specified, and the clean tree then failed check 1 with 9 diagnostics across 7 call sites — all from `prop_oneof!` expansions in `tests/ledger_props.rs` and `tests/money_props.rs`. Check 4b forbids a lint exemption anywhere in tracked Rust source, so there is no legal way to silence it. Rewriting wave 6's committed strategies to dodge a macro would be changing tested code to suit a lint. Guard 7c covers Arc instead, asserting it is absent from `src/` entirely — which is where it matters and where the macro never reaches.
- **Guard 7e counts the qualified field read `.liveness_enabled`, not the bare identifier.** The one production read binds a local that is used again a few lines later to filter the check table; that second use is the design working, not a second read of the configuration.
- **Guards 7e and 7h are scoped to the production half of `src/invariants.rs`.** The unit-test modules legitimately load the shipped configuration from a path and legitimately set the liveness key on a `Params` value. A guard that fired on them would be forbidding the tests from testing.
- **The mutant lives in `tests/ledger_atomicity.rs`, private and unexported.** A fixture directory would separate the two designs, and the whole claim is that a reader can see them side by side.
- **Guard 7d searches the raw files, comments included.** Same discipline as the float-name rule in `tests/numeric_det.rs`: prose about a banned token is written around the token.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `std::sync::Arc` removed from the nine planned `disallowed-types` entries**
- **Found during:** Task 2 (clippy entries)
- **Issue:** The plan lists nine entries and states that adding them is cheap. Eight are. `std::sync::Arc` makes the clean tree fail check 1: `prop_oneof!` expands to code naming it, producing 9 diagnostics across 7 call sites in `tests/ledger_props.rs` and `tests/money_props.rs`, and `tests/lints.sh` check 4b forbids the exemption that would silence it. This is the identical failure mode the plan's own `RefCell` finding exists to prevent, on a type the plan did not know about.
- **Fix:** Dropped the entry. Extended the `clippy.toml` comment to record both exclusions, both reasons, and both scope edges of the guard-7c substitution. Extended guard 7c to assert `Arc` is absent from `src/` entirely, which is strictly stronger than the lint entry would have been inside `src/`.
- **Files modified:** `clippy.toml`, `tests/lints.sh`
- **Verification:** `cargo clippy --all-targets --all-features -- -D warnings` clean; guard 7c observed to fire on an injected `Arc` mention in `src/config.rs`
- **Committed in:** `39a320b`, `5ad7344`

**2. [Rule 1 - Bug] Guards 7e and 7h fail on the real tree if written literally**
- **Found during:** Task 3 (source guards)
- **Issue:** 7e as written ("`liveness_enabled` appears exactly once in `src/invariants.rs` after line comments are stripped") reports 2 in the production half alone and 9 across the file, because the one read binds a local that the check-table filter uses. 7h as written ("`src/invariants.rs` names none of `Path`, `env::` …") fires on six test-module helpers that load the shipped configuration from a path.
- **Fix:** 7e counts the qualified field read `\.liveness_enabled`, which is the "one read site" the plan's own rationale describes; 7e and 7h both search the production half of the file, everything before the first `#[cfg(test)]` line. Both scopings are stated in the script comments with the reason.
- **Files modified:** `tests/lints.sh`
- **Verification:** both guards observed to fire — a second `params.invariants.liveness_enabled` read fails 7e with "read 2 times"; a `PathBuf` in `CheckSet::from_params` fails 7h
- **Committed in:** `5ad7344`

**3. [Rule 2 - Missing Critical] `src/books.rs` modified outside `files_modified`**
- **Found during:** the extra work the orchestrator assigned from WINDOWS entry 12
- **Issue:** Wave 6's must-have truth "ending a tick leaves both running residuals untouched" is asserted but cannot fail from an integration test, because on the honest path both residuals are already zero at every boundary and `tests/` cannot reach the `pub(crate)` corruption vocabulary.
- **Fix:** Added `ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched` to `src/books.rs`'s `mod tests`: seed a cash residual of one cent with `corrupt_recorded_cash` and a goods residual of two units with `corrupt_appended_posting`, then assert `end_of_tick` leaves both, clears what it does own, and does not drift them on a second boundary. No production code changed.
- **Files modified:** `src/books.rs`
- **Verification:** mutation check in both profiles — with both residual fields zeroed in `end_of_tick` the test fails at the cash assertion (left 0, right 1) while the integration property still passes
- **Committed in:** `98590df`

**4. [Rule 3 - Blocking] `grep` read guard 7g's pattern as a flag**
- **Found during:** Task 3
- **Issue:** The return-type pattern begins with `->`, and `grep -cE "$pattern"` treated the leading `-` as an option, exiting 2. The script correctly reported it as "could not search its own hazard fixture" rather than as a pass, which is the grep-status discipline working.
- **Fix:** The three fixture helpers pass the pattern with `-e`, with a comment saying why.
- **Files modified:** `tests/lints.sh`
- **Verification:** `bash tests/lints.sh` exits 0 and reports check 7
- **Committed in:** `5ad7344`

---

**Total deviations:** 4 auto-fixed (2 blocking, 1 bug, 1 missing critical)
**Impact on plan:** No scope creep and no dependency change. Two of the four are the plan's own discipline catching a fact the plan could not have known (Arc's macro expansion; the real content of `src/invariants.rs`), and both are recorded where the next reader will look. `git diff --exit-code -- Cargo.toml Cargo.lock` is empty.

## Issues Encountered

**Control B2 for check 6 did not isolate the branch it aimed at.** Exposing the corruption vocabulary by removing its `#[cfg(test)]` gate — intended to make the reachability probe compile and check 6 fail with "COMPILED" — instead failed at check 1, because the un-gated methods do not lint clean in a non-test build. That is a defensible outcome (the boundary breach is caught earlier, not missed), and check 6's "COMPILED" branch is textually identical to check 5's, which control A exercised directly. Recorded rather than papered over.

**`.planning/STATE.md` and `.planning/ROADMAP.md` are untouched.** No state-advancing command was run: `state.advance-plan`, `state.update-progress`, `state.record-metric`, `state.add-decision`, `state.record-session` and `roadmap.update-plan-progress` were all skipped per the wave shared-artifact rule, which is also why the WINDOWS entry 11 side effect did not occur. `requirements.mark-complete` writes only `.planning/REQUIREMENTS.md` and was run for LEDG-01, LEDG-02 and LEDG-10.

## Known Stubs

None. No file created or modified by this plan contains a hardcoded empty value, a placeholder string or an unwired data source. The one guard that is honestly weaker than it looks — 7f's `set_cash` half, whose subject types arrive in Phase 3 — says so in its own failure message and points at ROADMAP Phase 3 success criterion 7, and it is paired with the positive half that does carry weight today.

## Threat Flags

None. This plan adds tests, probes and lint configuration; it introduces no endpoint, no auth path, no file access on a behaviour path and no schema at a trust boundary. The `<threat_model>`'s nine registered threats are each mitigated by an artefact that was observed working, and the two probes deliberately replace what a compile-fail harness crate would have provided, so no package was installed.

## Verification

Full suite, both profiles, before returning:

| Command | Result |
|---|---|
| `cargo test --locked --all-targets` | 159 + 14 + 14 + 10 + 8 + 7 + 6 + 5 + 4 + 4 + 3 passed, 0 failed |
| `cargo test --locked --release --all-targets` | 157 + 14 + 14 + 10 + 8 + 7 + 6 + 5 + 4 + 4 + 3 passed, 0 failed |
| `cargo test --locked --release --test ledger_atomicity` | 10 passed |
| `bash tests/lints.sh` | OK — checks 1 through 7 |
| `bash tests/toolchain.sh` | OK |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `git status --porcelain tests/` after the lint script | no probe copy left behind |
| `git diff --exit-code -- Cargo.toml Cargo.lock` | empty — no dependency added |
| `grep -c 'fixture' tests/lints.sh` | 14 (≥ 8) |
| `grep -c 'E0502' tests/lints.sh` / `grep -c 'E0599' tests/lints.sh` | 4 / 4 (≥ 1 each) |
| `grep -c 'books_borrow_probe'` / `'books_cfg_test_probe'` | 3 / 3 (≥ 2 each) |
| `grep -c 'RefCell' clippy.toml` / `grep -cE 'benches\|all-targets' clippy.toml` | 5 / 2 (≥ 1 each) |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **LEDG-01, LEDG-02 and LEDG-10 are marked complete**, which closes the ten-requirement set for Phase 2. This was the final wave; every plan in the phase now has a SUMMARY.
- **Phase 3 inherits one named obligation.** The commit that introduces `Household` and `Firm` must extend guard 7f to name them, and must not add a balance field or a cash setter to either. The obligation is recorded in ROADMAP Phase 3 success criterion 7, in `src/books.rs`'s module documentation, and in guard 7f's failure message.
- **Two lint-list exclusions must survive.** `RefCell` and `Arc` are deliberately absent from `clippy.toml` and covered by guard 7c. A future reader who "fixes" the omission will break the clean tree; the reason is in the file.
- **A phase adding a `Books` method inherits guards 7a, 7b and 7g:** no callback parameter, no shared-mutability field, no mutable-reference return. A logging hook reads the journal after the call.
- **`.planning/STATE.md` and `.planning/ROADMAP.md` are left for the orchestrator**, as are the ten remaining open WINDOWS entries.

---
*Phase: 02-books-journal-and-invariants*
*Completed: 2026-08-31*

## Self-Check: PASSED

Every file this summary claims exists is on disk, every commit hash resolves in
`git log --all`, and the three named test symbols are present in their files
(`the_naive_ordering_unwinds_and_corrupts_its_total_under_the_same_harness`,
`ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched`, and
`tests/lints.sh`'s check 7 progress line).
