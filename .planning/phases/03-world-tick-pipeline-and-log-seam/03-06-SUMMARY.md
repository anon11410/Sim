---
phase: 03-world-tick-pipeline-and-log-seam
plan: 06
subsystem: testing
tags: [rust, golden-run, regression-baseline, determinism, lint-guard, tick-06, tick-09, cross-profile]

# Dependency graph
requires:
  - phase: 01-spine-config-rng-money
    provides: sim::config::load, the seeded ChaCha8 generator, the shipped config/baseline.toml whose bytes this plan derives a run from
  - phase: 02-ledger-and-invariants
    provides: guard 7h and the assert_fires/assert_ignores discipline in tests/lints.sh, the Violation type the narrowing is bounded against
  - phase: 03-02
    provides: src/phases.rs and src/world.rs (the two modules added to guard 7h's file set), the RunWriter and the recorded decision declining a violation record in src/log.rs
  - phase: 03-04
    provides: sim::log::first_difference — the first-differing-line helper the golden comparison reuses; the generated-artifact-plus-drift-test pattern this joins
  - phase: 03-05
    provides: tests/determinism.rs's nine tests, read_nonempty, entries, EXCLUDED_FROM_DIFF, and the one-leaf textual-substitution technique this plan reuses for the tick count
provides:
  - "tests/golden/ — a committed 50-tick run (ticks.csv, events.jsonl, provenance.csv) at the shipped seed, the regression baseline every later phase's rule changes are reviewed against"
  - "tests/regenerate_golden.sh — the single deliberate regeneration command, the only thing in the repository that writes those bytes"
  - "the_golden_run_reproduces — the tenth determinism test; run from both profiles against one artifact it is also the debug-and-release agreement claim"
  - "guard 7h extended to src/phases.rs and src/world.rs, and guard 7h-log: a labelled clause over src/log.rs narrowed in exactly one alternative and bounded by a zero-Violation assertion"
  - "The counting rule for the lint script's guard total, written into the check-7 preamble"
affects: [phase-04-analysis-harness, phase-05-goods-market, phase-06-labour, phase-11-calibration]

actuals:
  tokens: 13245   # 52,979 added chars / 4 over the realized diff, NOT a harness token count
  tasks: 2
  commits: 2

tech-stack:
  added: []   # no package; the golden run replaces `insta`, which was declined on measured evidence
  patterns:
    - "Commit a generated artifact, regenerate it with a named operator command, compare it with a test that never writes — the same shape clippy.toml and schema/schema.json already use"
    - "Derive a test's configuration from the shipped file by one textual leaf substitution, asserting the leaf count and the file shape first, rather than committing a second configuration that would drift"
    - "Enumerate the files to compare from the run directory minus a named exclusion constant, so a log file a later phase adds joins the golden or fails loudly, never silently loses coverage"
    - "Self-check the comparison inside the test body against a fabricated difference, before any of its green is believed"
    - "Narrow a guard in exactly one alternative and bind the narrowing to a positive assertion about the premise that justifies it, so the premise failing reopens the decision instead of widening the hole"
    - "Record the counting rule a script's own output depends on, at the place the output is produced"

key-files:
  created:
    - tests/golden/ticks.csv
    - tests/golden/events.jsonl
    - tests/golden/provenance.csv
    - tests/golden/README.md
    - tests/regenerate_golden.sh
  modified:
    - tests/determinism.rs
    - tests/lints.sh

key-decisions:
  - "The golden configuration is derived from config/baseline.toml by moving one leaf, never committed as a second file — a committed copy would drift and would then certify a configuration nobody runs"
  - "No --seed is passed by either the script or the test: the seed is a shipped parameter, so a deliberate seed change arrives as a golden diff instead of being overridden away"
  - "run_meta.json is excluded from tests/golden/, matching its exclusion from the determinism diff — it carries the compiler version string, and committing it would make a toolchain bump look like a change to the economy"
  - "The debug-and-release agreement claim is discharged by running one test against one committed artifact from both profiles, not by a test that spawns a build (which would deadlock on the build lock)"
  - "7h-log is a CLAUSE of guard 7h, not a guard of its own, so the script's count stays at eleven — the counting rule is now recorded rather than re-derived"

patterns-established:
  - "Golden run instead of a snapshot-testing crate: 8 lines of comparison, zero packages, and the review workflow the repository already has"
  - "A guard exemption must carry an assertion of its own premise"

requirements-completed: [TICK-06, TICK-09]

coverage:
  - id: D1
    description: "A committed 50-tick run reproduces byte for byte, so a deliberate rule change is a reviewable diff of the economy rather than a silent trajectory shift"
    requirement: TICK-09
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism the_golden_run_reproduces"
        status: pass
      - kind: integration
        ref: "bash tests/regenerate_golden.sh && git status --porcelain tests/golden"
        status: pass
    human_judgment: false
  - id: D2
    description: "Debug and release builds agree on every written byte, proved by comparing both profiles against one committed artifact"
    requirement: TICK-09
    verification:
      - kind: integration
        ref: "cargo test --locked --release --test determinism the_golden_run_reproduces"
        status: pass
    human_judgment: false
  - id: D3
    description: "The halt-message environment guard covers every module that can render a violation, and its one narrowed clause is bounded by an assertion that keeps the narrowing honest"
    requirement: TICK-06
    verification:
      - kind: other
        ref: "bash tests/lints.sh (guard 7h over four modules, clause 7h-log over src/log.rs)"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-08-31
status: complete
---

# Phase 3 Plan 06: The Golden Run and the Extended Halt Guard Summary

**A 50-tick run of the shipped model is now committed and reproduced byte for byte from both build profiles, and guard 7h covers all five modules on the halt path — four under its full pattern, one under a narrowing that asserts its own premise.**

## Performance

- **Duration:** ~15 min
- **Tasks:** 2/2
- **Commits:** 2
- **Suite:** 309 debug / 307 release (was 308/306; the two-test gap is the pre-existing `#[cfg(debug_assertions)]` re-entry pair in `src/rng.rs`)

## Commits

| Commit | Subject |
|--------|---------|
| `5e6200e` | `test(03-06): a committed 50-tick run, and the one command that rewrites it` |
| `c7e43fa` | `test(03-06): guard 7h over every module that renders a halt, narrowed once` |

## The committed golden run

| File | Bytes | Lines | What it holds |
|------|-------|-------|---------------|
| `tests/golden/ticks.csv` | **2,795** | 51 | a header plus 50 tick rows |
| `tests/golden/events.jsonl` | **18,560** | 220 | 200 household + 20 firm endowment records, all at tick 0 |
| `tests/golden/provenance.csv` | **49** | 1 | the seven-column header and no rows — correct for a phase in which nothing decides anything, and asserted rather than assumed by `provenance_has_a_header_even_with_no_rows` |
| `tests/golden/README.md` | — | — | the note: window length and both of its reasons, the seed, the derivation, the exclusion, the regeneration command |

`ticks.csv` at 2,795 bytes reproduces the research's measured figure for a 50-tick window exactly.

`run_meta.json` is **not** committed. It is the one file excluded from the determinism diff, it carries the compiler version string and a wall clock, and committing it would make a toolchain bump look like a change to the economy.

## What a review of a deliberate change looks like — measured

Measured against the committed artifact, not quoted from the research.

| Change | `diff` output on `ticks.csv` |
|--------|------------------------------|
| **Localised** — one value on one tick (row 25's `stock_units`, 3300 → 3299) | **4 lines** — a single `27c27` hunk naming the tick |
| **Trajectory-wide** — `seed = 42` → `43` | **102 lines** |
| `initial_price_cents = 105` → `106` | **0 lines** (see the caveat below) |
| `total_money_cents = 2000000` → `2000001` | run refused at startup: *"the endowment sums to 2000000 cents but the configured money stock is 2000001 cents; the books would begin the run already broken"* |

So the number a later phase should expect for a rule change that moves the whole trajectory is **~100 diff lines**, and for one that moves a single tick, **4**. Both are reviewable. The decade window would have produced 7,302 lines and rewritten 203 KB of the repository for the first case.

A seed change moves `ticks.csv` and leaves `events.jsonl` and `provenance.csv` byte-identical — the endowment stream is seed-independent by construction, which is the correct and expected shape.

### What the golden does not yet discriminate — record this before Phase 5

`initial_price_cents = 105 → 106` produces a **zero-line diff**. That is not a defect in the golden; it is the true state of the model. No rule reads a price yet, so the parameter reaches no written byte. The golden currently discriminates exactly what the pipeline computes: the activation digest, the money totals, the counts and the endowment stream.

This matters for the phase that first makes prices load-bearing. From that point the same one-value change **must** produce a non-zero diff, and if it does not, the new rule is not wired to the log. Worth using as a first check when Phase 5 or 6 lands.

## The narrowed pattern, exactly

```
full     (guard 7h)      env::|env!|(^|[^A-Za-z0-9_])Path(Buf)?[^A-Za-z0-9_]|SystemTime|Instant|std::process
narrowed (guard 7h-log)  env::|env!|SystemTime|Instant|std::process
```

Exactly one alternative dropped: the path type. Nothing else. The two are written adjacent in the script so a reader can diff them by eye.

**Applied to:**

- **full pattern** — the production halves of `src/invariants.rs`, `src/books.rs`, and now `src/phases.rs` and `src/world.rs`. The latter two can carry a violation to the code that renders it and neither holds a filesystem path, so the pattern reaches them unchanged (T-03-28).
- **narrowed pattern** — the production half of `src/log.rs`. The run-directory writer holds a `Path` because that is its whole job; the full pattern would block the module outright, and applying nothing would leave the environment unguarded in the one module that touches the disk.

**And the bound:**

```bash
VIOLATION_TYPE_PATTERN='\bViolation\b'
LOG_PRODUCTION_CODE=$(printf '%s\n' "$LOG_PRODUCTION" | sed 's://.*::')
assert_absent_in "guard 7h-log: src/log.rs renders or names a Violation, and guard 7h-log's
  narrowed pattern is only honest while it does not. Plan 03-02 DECIDED to keep the halt message
  out of the module that holds a filesystem path, and this assertion is what couples that decision
  to the narrowing it justifies. If emitting a violation record here is now deliberate, that
  decision has to be reopened — widen the pattern back to guard 7h's, or move the rendering out
  (T-03-29)" \
    "$VIOLATION_TYPE_PATTERN" "$LOG_PRODUCTION_CODE"
```

`src/log.rs`'s production half names `Violation` **zero** times today. Line comments are stripped first, the same treatment guards 7d and 7g give their own patterns, so a doc comment stating this very rule cannot trip it.

The narrowing is legitimate only while no halt message is rendered there. That is plan 03-02's recorded decision — *"narrowing a guard to accommodate new code is how a guard stops meaning anything"* — and this assertion is what keeps the exemption and its premise coupled. A later phase that decides to emit a violation record into the event stream makes it fail, and the decision returns to a human instead of the hole quietly widening.

The **ignore** half of 7h-log is demonstrated live on the real tree, not only on a fixture: `src/log.rs` already names `Path` twice (`use std::path::Path;` at line 86 and `pub fn new(dir: &Path)` at line 537), and the clean tree passes.

## The counting rule, and why two plans in one phase had to decide it

Now written into the check-7 preamble of `tests/lints.sh`:

> A guard is COUNTED when a requirement or a roadmap criterion names it as an obligation of its own. Everything else is a CLAUSE of a guard that already exists, and clauses are not counted however much work they do.

- **Counted:** `7f-agents` — ROADMAP Phase 3 criterion 7 is about it, so it answers for itself. Plan 03-02 raised the count from ten to eleven on that basis.
- **Not counted:** the cash-setter clause of 7f, and `7h-log`. Neither has a requirement of its own; each narrows or extends the guard it sits under, and each is proved by its own fixtures regardless.

Two plans in this phase both had to decide this and neither had a written rule to appeal to: 03-02 decided *up* (ten → eleven), 03-06 decided *not to move* (eleven → eleven). Both decisions are defensible under the rule; neither was derivable from the script before it was written down. The number appears in the script's own output, so the next reader would otherwise have had to reconstruct the rule from arithmetic.

The count therefore **stays at eleven**, and the stability greps confirm it survived this task unchanged: `grep -c 'eleven source guards'` = 3, `grep -ic` = 4 (the fourth being the capitalised section header at the check-7 preamble), and both `ten source guards` greps = 0.

## Every new check watched failing before it was trusted

The file's own stated principle is that a gate never observed to block has never been shown to work. Every mutation below was confirmed to have actually changed the file (`cmp`) before the check was run, and confirmed byte-identical again after restore.

### The golden test

| Mutation | Result |
|----------|--------|
| One field of `tests/golden/ticks.csv` row 12 changed (`2000000` → `1999999`) | **FAILED** — *"the golden run no longer reproduces: ticks.csv differs at line 12"*, printing both lines and naming `bash tests/regenerate_golden.sh` |
| `tests/golden/provenance.csv` removed | **FAILED** — *"the golden directory holds a different set of files than a run produces"*, printing both sets |
| `run_meta.json` placed in `tests/golden/` | **FAILED** — *"run_meta.json is committed in the golden directory … committing it would make a toolchain bump look like a change to the economy"* |
| The regeneration script's target leaf changed to `ticks = 60` | **FAILED** — *"…does not mention `ticks = 50`, so it is not performing the substitution this test performs"* |
| The fabricated self-check row set equal to the real first data row | **FAILED** — *"the fabricated row equals the real first data row, so perturbing with it would prove nothing"* |
| `GOLDEN_TICKS` shrunk to 20 | **FAILED** — *"the golden window is 20 ticks but the shipped cadence is a 21-day month; below two planning cycles a cadence-length effect is invisible"* |
| clean tree | **passes**, in both profiles |

The last two are the adversarial ones. The self-check inside the test body fabricates a difference in a data row and asserts the comparison reports it at line 2 — so the comparison is proved to have teeth in the same run in which it is trusted, rather than being a comparison of a file with itself. And the window floor is asserted against the shipped `month_days`, so shrinking the window to twenty fails loudly instead of quietly losing every cadence-length effect.

### The guard

| Mutation (each made to **compile**, so nothing else would catch it) | Result |
|---|---|
| `pub fn halt_note() -> &'static str { env!("CARGO_MANIFEST_DIR") }` in `src/phases.rs` | **FAILED** — guard 7h, naming the file and the line |
| `pub fn halt_note(p: &std::path::Path) -> bool { p.is_absolute() }` in `src/world.rs` | **FAILED** — guard 7h |
| `pub fn halt_note() -> &'static str { env!("CARGO_MANIFEST_DIR") }` in `src/log.rs` | **FAILED** — guard **7h-log**: *"exempt from guard 7h's PATH alternative only … and from nothing else"* |
| `pub fn render_halt(v: &crate::invariants::Violation) -> String { format!("{v}") }` in `src/log.rs` | **FAILED** — the bounding assertion, naming plan 03-02's decision and T-03-29 |
| clean tree | **passes** — `bash tests/lints.sh` exits 0, reporting eleven source guards |

The third and fourth are the pair that make the narrowing honest: the log module is exempt from the path alternative and from nothing else, and it is exempt at all only while it renders no violation.

## Verification results

Every `<automated>` check from the plan, run and confirmed against its `<fails_when>`.

### Task 1

| Check | Result |
|-------|--------|
| `bash tests/regenerate_golden.sh && git status --porcelain tests/golden` | exit 0, **empty** status (run post-commit) |
| `cargo test --locked --test determinism the_golden_run_reproduces` | **1 passed** |
| `cargo test --locked --release --test determinism the_golden_run_reproduces` | **1 passed** |
| `wc -l < tests/golden/ticks.csv` | **51** |
| `ls tests/golden` | `README.md  events.jsonl  provenance.csv  ticks.csv` — three diffed files plus one note, no run record |
| `grep -c 'ticks = 3650' tests/regenerate_golden.sh` | **1** |
| `grep -c 'regenerate_golden' tests/determinism.rs` | **2** |
| `cargo test --locked --test determinism` | **10 passed** (the nine from 03-05 plus the golden) |
| `cargo test --locked --all-targets` | exit 0, **309 passed** |

### Task 2

| Check | Result |
|-------|--------|
| `bash tests/lints.sh` | exit 0; final line reports eleven source guards, 7h over the four halt-message modules and its 7h-log clause over the path-holding writer |
| `grep -c 'eleven source guards' tests/lints.sh` | **3** |
| `grep -ic 'eleven source guards' tests/lints.sh` | **4** |
| `grep -c 'ten source guards' tests/lints.sh` | **0** |
| `grep -ic 'ten source guards' tests/lints.sh` | **0** |
| `grep -c '7h-log' tests/lints.sh` | **14** (fire fixture, ignore fixture and absence assertion for each of the two patterns, plus commentary) |
| `grep -cE 'src/(phases\|world)\.rs' tests/lints.sh` | **8** |
| `bash tests/toolchain.sh` | exit 0 |
| `cargo test --locked --all-targets` | exit 0, **309 passed** |
| `cargo test --locked --release --all-targets` | exit 0, **307 passed** |

### Whole-plan gate

| Check | Result |
|-------|--------|
| `bash tests/regenerate_golden.sh` | exit 0, working tree clean |
| `cargo test --locked --test determinism` (both profiles) | 10 passed / 10 passed |
| `bash tests/lints.sh` | exit 0, eleven source guards |
| `bash tests/toolchain.sh` | exit 0 |
| `bash tests/schema_drift_negative.sh` | exit 0, observed failing on the perturbed schema (exit 101) and passing again after the digest-verified restore; working tree clean |
| `cargo test --locked --all-targets` / `--release --all-targets` | 309 / 307 |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `git status --porcelain` | empty |

## Deviations from Plan

No deviation rule was invoked. Two measurement notes, neither of which changed the work.

**1. Guard 7h's line citation in the plan's `read_first`.** The plan cites `tests/lints.sh:682-727`. Measured against the tree at `b68f2cf`, guard 7h's comment block opens at **line 735** (`# 7h. No environment in a halt message.`) and guard 7i's header is at **line 783**; line 682 is the `MONEY_FIELD` pattern inside guard 7f-agents. This is a `read_first` pointer rather than a check, so nothing was blocked and nothing was changed on account of it — but the citation is stale in the same way the one this task was asked to correct was, and is recorded here so the next reader does not re-measure it.

**2. The stale reference the task *was* asked to correct.** Guard 7h's commentary named `tests/config_strict.rs:275` and `tests/tracer_end_to_end.rs:21` as the two legitimate `std::process::id` call sites that make the clippy entry unaddable. Measured: the first is still exact; the second moved to **line 33**. Corrected, with a note in the comment recording that it moved and why a stale citation in a comment explaining a *declined* lint entry is how the reasoning stops being checkable.

## Known Stubs

None. No production code was added by this plan.

## Threat Flags

None. This plan adds no modules, endpoints, schema changes or trust boundaries; it commits a generated artifact and tightens a static guard.

Both threats the plan dispositioned as `mitigate` are discharged and were watched firing:

- **T-03-28** (environment in a halt message from an unsearched module) — guard 7h's file set now covers all four modules that can render one.
- **T-03-29** (a guard narrowed for convenience becoming a hole) — 7h-log drops one alternative and asserts its own premise.
- **T-03-30** (a golden artifact regenerated by the test that checks it) — the test never writes; regeneration is a separate script named in the failure message.
- **T-03-31** (a committed run record making a toolchain bump look like an economic change) — excluded, and asserted excluded.
- **T-03-32** (a trajectory-wide change landing unreviewed) — measured at 102 diff lines.

## Phase 3 requirement index

One place to look during the phase's verification pass. Every command below was run at `c7e43fa` and exits 0.

| Req | Claim | Command |
|-----|-------|---------|
| **TICK-01** | The tick is a fixed `PHASES` table run in order, each phase completing for all agents before the next | `cargo test --locked --lib phases::order` |
| **TICK-02** | The generated schema equals the committed `schema/schema.json` | `cargo test --locked --test log_schema schema_matches_the_committed_file` |
| **TICK-02** | …and the drift test has been watched failing | `bash tests/schema_drift_negative.sh` |
| **TICK-03** | `ticks.csv` is flat, integer-only, and money is in `*_cents` columns | `cargo test --locked --test log_schema ticks_csv_is_flat_and_integer_only` |
| **TICK-03** | …and the run directory is well formed: row count, `\n` terminator, no empty field | `cargo test --locked --test determinism the_run_directory_is_well_formed` |
| **TICK-04** | Every `Event` variant round-trips, is flat, and appears in the schema | `cargo test --locked --lib log::events` |
| **TICK-04** | The endowment stream sums to the configured money stock | `cargo test --locked --test determinism endowment_events_sum_to_the_money_stock` |
| **TICK-05** | `run_meta.json` carries seed, config hash and toolchain, held out of the diff | `cargo test --locked --test determinism run_meta_carries_the_three_fields` |
| **TICK-06** | No path, host name, PID or timestamp in any diffed file — and the exclusion is enforced, not documented | `cargo test --locked --test determinism the_exclusion_is_enforced_not_documented` |
| **TICK-06** | The halt message carries no environment, at the message level, and the process exits 1 naming tick 0 | `cargo test --locked --test determinism the_binary_halts_on_a_liveness_violation_at_tick_zero` |
| **TICK-06** | …and at the source level, over all five modules on the halt path | `bash tests/lints.sh` (guard 7h + clause 7h-log) |
| **TICK-07** | `provenance.csv` is a joinable flat table with its exact header, written eagerly | `cargo test --locked --test log_schema provenance_has_a_header_even_with_no_rows` |
| **TICK-08** | 3,650 empty ticks execute, invariants pass, the run directory is complete | `cargo test --locked --release --test determinism the_empty_decade_runs` |
| **TICK-09** | Same seed, byte-identical, in one process | `cargo test --locked --test determinism same_seed_identical_in_process` |
| **TICK-09** | Same seed, byte-identical, across two processes | `cargo test --locked --test determinism two_processes_at_one_seed_write_identical_bytes` |
| **TICK-09** | Debug and release agree on every written byte | `cargo test --locked --release --test determinism the_golden_run_reproduces` (against the same artifact the debug pass compares) |
| **TICK-09** | A deliberate rule change is a reviewable diff, not a silent trajectory shift | `cargo test --locked --test determinism the_golden_run_reproduces` |
| **TICK-10** | A different seed produces different logs | `cargo test --locked --test determinism different_seed_differs` |
| **TICK-10** | …and the counter-check has teeth (blank `activation_digest` → red) | recorded in `tests/determinism.rs`'s module docs; measured in plan 03-05 |

The validation map's TICK-09 row *"Debug and release bytes agree"* proposed a test named `debug_and_release_agree` or a CI step. It is discharged instead by running `the_golden_run_reproduces` from both profiles against one committed artifact, exactly as this plan prescribed: a test that spawned a second build from inside a test would deadlock on the build lock, and comparing both profiles to a shared committed artifact is the same claim without that.

## Self-Check: PASSED

All seven created/modified files exist on disk; both commit hashes (`5e6200e`, `c7e43fa`) are present in the log; neither commit deletes a tracked file; no untracked files remain outside `.planning/`.
