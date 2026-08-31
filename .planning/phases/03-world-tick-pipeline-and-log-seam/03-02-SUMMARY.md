---
phase: 03-world-tick-pipeline-and-log-seam
plan: 02
subsystem: infra
tags: [rust, csv, serde, sha2, proptest, determinism, tick-pipeline, logging]

# Dependency graph
requires:
  - phase: 01-spine-config-rng-money
    provides: Money, Rngs/Stream/Purpose, FirmArena/HouseholdId/Account, config::load, the float-confinement guard, tests/lints.sh
  - phase: 02-ledger-and-invariants
    provides: Books (accessors + end_of_tick), CheckSet::run, Violation, the ALL_CHECKS construction this plan copies
  - phase: 03-01
    provides: csv + serde_json in [dependencies], assert_cmd + tempfile in [dev-dependencies], the criterion-3 amendment to the activation_digest mechanism
provides:
  - "sim::world — World, Household, Firm; agents that hold no balance"
  - "sim::phases — Ctx, PhaseId, PhaseFn, PHASES (nine phases, fixed order), tick, run, shuffle_activation, order_digest"
  - "sim::log — Sink, TickRow (nine integer columns), NullSink, VecSink, RunWriter, ticks_header, TICKS_FILE"
  - "src/main.rs — the real CLI over the pipeline, replacing the Phase 1 tracer"
  - "tests/lints.sh guard 7f-agents (three clauses), discharging ROADMAP Phase 3 criterion 7"
  - "tests/order_digest_props.rs — the phase's single property test"
  - "A runnable decade: config/baseline.toml -> 3,651-line ticks.csv, seed-sensitive, byte-identical at one seed"
affects: [03-03, 03-04, 03-05, 03-06, phase-04-analysis-harness, phase-05-production, phase-06-labour]

actuals:
  tokens: 15569   # chars/4 over the added lines of the realized diff (62,279 chars), NOT a harness token count
  tasks: 3
  commits: 3

tech-stack:
  added: []   # no new dependency; plan 03-01 added csv/serde_json/assert_cmd/tempfile
  patterns:
    - "const PHASES table + PhaseId::ALL + exhaustive-match position function, copied structurally from src/invariants.rs's ALL_CHECKS"
    - "Sink trait with finish() -> io::Result, called once before the run outcome is inspected"
    - "Eager CSV header, derived from the serde impl rather than hand-typed"
    - "Digest-of-permutation as the seed's route into a diffed byte"

key-files:
  created:
    - src/world.rs
    - src/phases.rs
    - src/log.rs
    - tests/order_digest_props.rs
    - .proptest-regressions/order_digest_props.txt
  modified:
    - src/lib.rs
    - src/main.rs
    - tests/lints.sh
    - tests/tracer_end_to_end.rs
    - .gitignore

key-decisions:
  - "No violation record is written from src/log.rs, and the violating tick is never logged (03-RESEARCH.md Open Question 3). The eager header is what keeps a halted run's artefact openable."
  - "The run-directory default is runs/latest, and /runs is gitignored; the committed golden run of 03-06 lives under tests/, not there."
  - "PhaseFn returns Result<(), Violation> rather than nothing, so the halt is testable in process (LEDG-10)."
  - "Ctx holds &Rngs, not &mut Rngs, because Rngs::stream takes &self."
  - "Sink::tick_row returns nothing and RunWriter keeps its FIRST error for finish() to report — the first error is the attributable one."
  - "different_seed_changes_the_draw was renamed to different_seed_changes_the_activation_digest: the binary no longer prints a draw, and a test name that does not describe what it measures is this project's recurring defect shape."
  - "Only TICK-01 was marked Complete. TICK-03, TICK-08 and TICK-10 are also claimed by plans 03-04 and 03-05 and stay Pending until those land (WINDOWS.md entry 25 is the precedent)."

patterns-established:
  - "Phase table: ordering IS the specification, so PhaseId::ALL is the single source and the position function is an exhaustive match — a tenth phase is a compile error, not a silent gap."
  - "Header derivation: one exemplar row serialised through a throwaway header-enabled writer is the only source of the column names."
  - "Guard extension in the same commit as the types it polices, mutation-proved on the real tree before being trusted to be silent."

requirements-completed: [TICK-01]

coverage:
  - id: D1
    description: "The tick is a fixed nine-entry PHASES table running in a documented order, each phase completing for every agent before the next begins, and a tenth phase cannot be added without placing it"
    requirement: TICK-01
    verification:
      - kind: unit
        ref: "cargo test --locked --lib phases::order (5 tests: the_table_runs_the_documented_sequence, an_identifier_cannot_exist_without_a_table_entry, the_derived_order_agrees_with_the_run_order, the_names_are_distinct_and_spell_their_identifiers, the_check_runs_before_the_log)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A decade of empty ticks executes and writes an integer-only ticks.csv with an eager header, one row per tick, money columns named *_cents"
    requirement: TICK-03
    verification:
      - kind: integration
        ref: "src/phases.rs#end_to_end::a_decade_of_empty_ticks_lands_on_disk"
        status: pass
      - kind: e2e
        ref: "tests/tracer_end_to_end.rs#runs_end_to_end"
        status: pass
      - kind: other
        ref: "./target/debug/sim --config config/baseline.toml --seed 42 --out /tmp/v42 → 3,651 lines, 202,974 bytes, 0 non-integer fields, 0 empty fields, no CR"
        status: pass
    human_judgment: false
  - id: D3
    description: "3,650 empty ticks execute and two runs at one seed diff byte-identically before any economic rule exists"
    requirement: TICK-08
    verification:
      - kind: integration
        ref: "src/phases.rs#end_to_end::the_same_seed_replays_the_same_rows"
        status: pass
      - kind: e2e
        ref: "tests/tracer_end_to_end.rs#same_seed_is_reproducible"
        status: pass
      - kind: other
        ref: "sha256 of two seed-42 runs: 052499a84e9288b4… twice; the release binary agrees byte for byte with the debug binary"
        status: pass
    human_judgment: false
  - id: D4
    description: "A different seed produces a different log, via an activation_digest derived from the tick's permutation"
    requirement: TICK-10
    verification:
      - kind: integration
        ref: "src/phases.rs#end_to_end::the_seed_reaches_the_first_logged_digest"
        status: pass
      - kind: e2e
        ref: "tests/tracer_end_to_end.rs#different_seed_changes_the_activation_digest"
        status: pass
      - kind: unit
        ref: "tests/order_digest_props.rs (4 properties over generated permutations)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Household and Firm exist, carry no balance field and no balance-shaped field name, policed by guard 7f-agents added in the same commit"
    verification:
      - kind: other
        ref: "bash tests/lints.sh (check 7, guard 7f-agents clauses a/b/c) — mutation-proved on three hazard shapes against the real tree"
        status: pass
      - kind: other
        ref: "git show --stat c4ab2ff shows src/world.rs and tests/lints.sh in one commit"
        status: pass
    human_judgment: false
  - id: D6
    description: "The binary runs the pipeline end to end and halts non-zero on a violation with the log flushed and no environment in the message"
    verification:
      - kind: other
        ref: "sim --config <baseline with liveness_enabled=true> → exit 1, stderr `tick 0: liveness — 0 transactions recorded, at least 1 required; no posting, which is the violation`, ticks.csv left at 111 bytes (header only) rather than 0"
        status: pass
    human_judgment: false

# Metrics
duration: ~30min
completed: 2026-08-31
status: complete
---

# Phase 3 Plan 02: Tracer — World, Tick Pipeline and Log Seam Summary

**A runnable decade: `config/baseline.toml` through a fixed nine-phase tick pipeline and the invariant check into a 3,651-line integer-only `ticks.csv` that a different seed changes at tick 0 and the same seed reproduces byte for byte.**

## Performance

- **Duration:** ~30 min (commit span 14:35:34 → 14:40:30 UTC; context read preceded it)
- **Completed:** 2026-08-31T14:41:52Z
- **Tasks:** 3 of 3
- **Files created/modified:** 10 (`1453 insertions(+), 85 deletions(-)`)
- **Test count:** 242 → **263** in debug (+17 library, +4 property), 261 in release (the two `#[cfg(debug_assertions)]` sub-stream re-entry tests in `src/rng.rs` do not exist there — pre-existing, unrelated to this plan)

## Accomplishments

- **The tick pipeline exists and is fixed.** `PHASES` is a nine-entry `const` table declared in run order with `PhaseId::ALL` beside it as the single source of the sequence. The `order` test module carries five claims, including an exhaustive-match `documented_position` — a tenth phase stops the test module compiling until it is placed. The invariant check is position 7 and the log is position 8, which is what makes "the violating tick is never logged" a consequence of the table rather than of a comment.
- **A decade of empty ticks lands on disk.** 3,650 ticks in 0.54 s (debug), producing a 3,651-line, 202,974-byte `ticks.csv` with nine integer columns, no empty cell and no carriage return.
- **The seed reaches a diffed byte.** `activation_digest` is a sha256-derived value over the tick's whole activation permutation, shifted right one bit so it stays positive. Seeds 42 and 43 differ at tick 0 while `rng_draws` is identical at 218 — which is precisely the pair of facts that made the count-only design vacuous.
- **`Household` and `Firm` exist and hold no balance**, with guard `7f-agents` (three clauses) added in the same commit and mutation-proved against the real tree on all three hazard shapes.
- **The CLI is real.** `src/main.rs` loads the config, resolves the effective seed, builds the books, world, generator set and check set, runs the pipeline, **finishes the writer before inspecting the outcome**, and on a violation prints the rendered `Violation` to standard error and exits 1.

## Task Commits

1. **Task 1 (tracer): the world, the fixed nine-phase tick pipeline and the tick log** — `c4ab2ff` (feat)
2. **Task 2: replace the Phase 1 tracer binary with the real CLI** — `fd791d6` (feat)
3. **Task 3: the one property test this phase earns** — `3ef67d2` (test)

## Files Created/Modified

- `src/world.rs` (185 lines) — `World`, `Household` (identity only), `Firm` (posted price in cents only). Three unit tests.
- `src/phases.rs` (555 lines) — `Ctx`, `PhaseId`, `PhaseFn`, `PHASES`, `noop`/`run_invariants`/`run_log`, `shuffle_activation`, `order_digest`, `tick`, `run`, plus the in-module `order` (5 tests) and `end_to_end` (3 tests) modules.
- `src/log.rs` (338 lines) — `Sink`, `TickRow`, `NullSink`, `VecSink`, `RunWriter`, `ticks_header`, `TICKS_FILE`. Six unit tests.
- `src/lib.rs` — three module registrations, flat alphabetical order preserved.
- `src/main.rs` — rewritten as the CLI over the pipeline.
- `tests/lints.sh` — guard `7f-agents` (+63 lines) and the count prose at four sites.
- `tests/tracer_end_to_end.rs` — three tests ported; the four overflow tests and their comment block byte-identical.
- `tests/order_digest_props.rs` (140 lines) — four properties.
- `.proptest-regressions/order_digest_props.txt` — one pinned seed (see *Issues Encountered*).
- `.gitignore` — `/runs`.

### Column names frozen by this plan (`ticks.csv`, in order)

`tick`, `total_money_cents`, `firm_cash_cents`, `stock_units`, `headcount`, `transactions`, `rng_draws`, `activation_digest`, `postings`

## Measurements the plan asked for

| Question | Measured |
|---|---|
| Per-tick draw count | **218** — `199` (200 households − 1) + `19` (20 firms − 1), exactly as the shuffle's documented `len - 1` |
| Constant across the run? | **Yes.** `cut -d, -f7 ticks.csv \| sort -u` over 3,650 data rows returns a single value, `218`. That single-value check *is* the fixed-draw-sampling assertion (CORE-05) at the artefact level. |
| `ticks.csv` byte size at 3,650 ticks | **202,974 bytes** (3,651 lines including the header) |
| Header-only file (halted run) | **111 bytes** — not zero, which is the whole point of the eager header |
| Distinct `activation_digest` values over 3,650 ticks | **3,650** — every tick distinct; **0** negative; max `9223125204018266328`, inside `i64::MAX` |
| Wall clock, debug binary, 3,650 ticks | 0.54 s |
| Same seed, two runs | sha256 `052499a84e9288b4…` both times; the **release** binary also agrees byte for byte with the debug one |
| Seeds 42 vs 43, tick 0 | digest `6004728580991614357` vs `4416797903915156233`; `rng_draws` **218 in both** |

## Decisions Made

### Run-directory default: `runs/latest`

`--out` now has a default so a bare invocation is meaningful. `runs/latest` is the convention the research probe used and CONTEXT.md leaves the naming to discretion. It is a fixed literal joined onto an operator-supplied path and is never assembled from configuration content (T-1-04 / T-03-04). `/runs` was added to `.gitignore` — see the deviation below.

### No violation record in the log module (03-RESEARCH.md Open Question 3 — resolved: decline)

The violating tick is **never logged**, because the check is pipeline position 7 and the log is position 8. No violation record is emitted into any stream and no extra row is written. Two reasons, and the second is the structural one:

1. The tick series stays a series of ticks that **passed** their check. That is what makes a row in it mean something; a series mixing passed and failed ticks would need a status column that every downstream consumer has to remember to filter on.
2. **The guard-7h consequence.** `src/log.rs` is the one module that legitimately holds a filesystem path. Rendering a halt message there would put the environment next to the message — and guard 7h (no path, host name, process identifier or wall-clock reading in anything a halt emits) would then have to be either narrowed at `src/log.rs` or extended over it wholesale. Narrowing a guard to accommodate new code is how a guard stops meaning anything; extending it over the log module would block the module's own reason to exist. Keeping the halt message out of the log module entirely costs nothing and leaves guard 7h untouched.

The evidence for a halted run is carried by three things instead: the eager header (the artefact stays openable), the rendered `Violation` on standard error, and — from plan 03-03 — the run record's outcome field.

### Guard `7f-agents`: the three hazard shapes, and the clean tree

Each clause was first proved to fire on its own fixture and to ignore a legitimate lookalike (the script's `assert_fires` / `assert_ignores` discipline), and then **mutated against the real tree**, since a fixture only proves the pattern is not a typo.

| Mutation to `src/world.rs` (each made to **compile**, so nothing else would catch it) | Clause | Result |
|---|---|---|
| `pub cash: crate::money::Money,` on `Household`, initialised to `Money::ZERO` | (a) money-typed field | **FAILED** — `guard 7f-agents: an agent type in src/world.rs declares a money-typed field … — found: 37:    pub cash: crate::money::Money,` |
| `pub inventory: i64,` on `Firm`, initialised to `0` | (b) balance-shaped name | **FAILED** — `guard 7f-agents: an agent type in src/world.rs declares a balance-shaped field. The books own the quantity — found: 47:    pub inventory: i64, 104:                inventory: 0,` |
| `struct Household` renamed to `struct Consumer` **with `pub type Household = Consumer;` added**, so the whole crate still compiles and every other test still passes | (c) the types exist | **FAILED** — `guard 7f-agents: struct Household is not declared in src/world.rs — the guard polices a set that does not contain the types criterion 7 names` |
| clean tree | all three | **passes** — `bash tests/lints.sh` exits 0, final line reports *eleven source guards* |

The third mutation is the one worth noting: the alias made every downstream mention of `Household` compile and the entire 263-test suite stayed green. Clause (c) was the only thing in the repository that noticed.

### The digest, mutation-proved

Criterion 3 was itself an instance of this project's recurring defect shape, so the mechanism replacing it was broken deliberately:

| Mutation to `order_digest` / `shuffle_activation` | What failed | What did **not** |
|---|---|---|
| `world.activation_digest = 1` (blank the digest) | `phases::end_to_end::the_seed_reaches_the_first_logged_digest` and the varying-digest assertion in `a_decade_of_empty_ticks_lands_on_disk` | `the_same_seed_replays_the_same_rows` — correctly, it is a different claim |
| digest the **first element only** of the household order (the rejected "first activated household" design) | `a_tail_only_change_in_the_household_order_changes_the_digest` | the other three properties |
| drop the firm order from the hash | `a_change_in_the_firm_order_alone_changes_the_digest` | the other three properties |

Each mutation fails exactly the check that names it and nothing else.

### Tests ported out of the Phase 1 tracer file

`tests/tracer_end_to_end.rs` had three tests parsing the binary's single stdout line, which no longer exists. All three were **ported, not deleted**, in the same commit that rewrote `src/main.rs`:

| Was | Is now |
|---|---|
| `runs_end_to_end` — parsed `effective_seed`, `config_sha256`, `draw` and `money_cents` out of the tracer line and recomputed each through `use sim::…` | `runs_end_to_end` — the binary exits cleanly, prints **nothing** on standard output, and leaves a tick file of one header line plus one row per **configured** tick, the count read from the shipped config through `sim::config::load` rather than written out |
| `same_seed_is_reproducible` — compared two runs' stdout | `same_seed_is_reproducible` — two runs at seed 7 into two different output directories leave **byte-identical** tick files |
| `different_seed_changes_the_draw` — compared the `draw=` field at two seeds | `different_seed_changes_the_activation_digest` — two seeds give an **equal** `rng_draws` and a **differing** `activation_digest` in the first data row, both read **by column name** out of the header |

The `assert!(output.stdout.is_empty(), …)` inside the shared `run` helper is the runtime half of "the tracer line is gone": a rewrite that had been additive rather than a replacement would fail there.

**Untouched, as required:** the four overflow tests (`raw_i64_overflow_panics_when_overflow_checks_are_on`, `raw_i64_at_the_maximum_does_not_panic`, `raw_i64_overflow_panics_across_an_inline_boundary`, `raw_i64_overflow_panics_inside_a_generic`, plus `the_held_out_sites_do_not_panic_one_step_below_the_edge`) and their two explanatory comment blocks. Verified by extracting the region from `HEAD~1` and `diff`-ing it — byte-identical, not assumed. The hand-rolled `out_dir` helper is also unchanged: guard 7h's comment names it as one of the two call sites that make a `std::process::id` clippy entry unaddable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — Missing Critical] `/runs` added to `.gitignore`**
- **Found during:** Task 2 (the CLI rewrite)
- **Issue:** Giving `--out` a default of `runs/latest` means a bare `cargo run` in the repository root writes generated run artefacts into the working tree as untracked files. The plan mandates the default but does not mention the ignore.
- **Fix:** Appended `/runs` with a comment recording that the *committed* golden run of plan 03-06 lives under `tests/`, not here — so a future reader cannot mistake the ignore for one that would swallow a reviewed artefact.
- **Files modified:** `.gitignore`
- **Verification:** A bare invocation from a scratch directory writes `runs/latest/ticks.csv`; `git status --short` is clean after a repo-root run.
- **Committed in:** `fd791d6`

**2. [Rule 1 — Correctness of the assertion] `different_seed_changes_the_draw` renamed**
- **Found during:** Task 2
- **Issue:** The plan says to port the test; keeping the old name would leave a test called "changes the draw" measuring an activation digest, while `rng_draws` is in fact *identical* across seeds. That is exactly this project's recurring defect shape — an assertion whose stated claim is not what it measures.
- **Fix:** Renamed to `different_seed_changes_the_activation_digest`, and added the `rng_draws` equality assertion beside the inequality so the file states both halves of why the digest column exists.
- **Verification:** `cargo test --locked --test tracer_end_to_end` — 8 passed, in both profiles.
- **Committed in:** `fd791d6`

**3. [Rule 2 — Internal consistency] `tests/lints.sh:382` prose updated too**
- **Found during:** Task 1
- **Issue:** The plan names four sites of the phrase `ten source guards`. A fifth line — `# THE DISCIPLINE, and it applies to all ten.` — states the same count in a form none of the four greps can see.
- **Fix:** Changed to `all eleven`. This does **not** affect the four measured counts: `grep -c 'eleven source guards'` = 3, `grep -ic` = 4, both `ten` greps = 0, exactly as the plan predicted.
- **Verification:** all four grep counts re-measured after the edit; `bash tests/lints.sh` exits 0.
- **Committed in:** `c4ab2ff`

**4. [Rule 2 — Traceability honesty] Only `TICK-01` marked Complete**
- **Found during:** State updates
- **Issue:** The plan's frontmatter lists `TICK-01, TICK-03, TICK-08, TICK-10`. `TICK-03` is also claimed by plan 03-04, and `TICK-08`/`TICK-10` by plan 03-05. Marking all four Complete now would make the traceability table claim work that two later plans have yet to do — the exact failure `WINDOWS.md` entry 25 records for plan 03-01.
- **Fix:** Marked `TICK-01` (delivered here, claimed by no other plan). Left `TICK-03`, `TICK-08` and `TICK-10` Pending; their coverage entries above record that this plan's half is green and tested.
- **Verification:** `.planning/REQUIREMENTS.md` traceability table inspected after the update.
- **Recorded in:** `WINDOWS.md` as a `deviation`, so it is visible at ship time.

---

**Total deviations:** 4 auto-fixed (1 blocking hygiene, 1 correctness-of-assertion, 2 consistency/traceability)
**Impact on plan:** None on scope. Every deviation makes an artefact say what is true rather than what was convenient; the third and fourth exist because a check or a table that overstates is the defect shape this project keeps finding.

## Issues Encountered

**The committed proptest regression seed came from a mutant, not from a defect.** `.proptest-regressions/order_digest_props.txt` holds one entry, shrunk to the **identity permutation** (`households = [0, 1, …, 199]`, `firms = [0, …, 19]`). It was produced by the two deliberate mutations of `order_digest` described above, not by any shipped build — **no shipped build has ever failed this file**. It was committed anyway, because the plan says to commit any regression file the run produces and because the identity permutation is a genuinely valuable pinned case: it is exactly what a shuffle that stopped shuffling would produce. Recorded here so nobody later chases a defect that never existed.

**`_config_sha256` is loaded and not yet used** in `src/main.rs`. The plan directs the body to "load the configuration and its hash"; plan 03-03 writes it into `run_meta.json`. It is deliberately named with a leading underscore and carries a comment naming the consuming plan. This is a forward reference, not a stub — no fake value is produced and nothing downstream reads a placeholder — so it is recorded here rather than under *Known Stubs*.

## Known Stubs

The seven economic phase functions (`firm_planning` … `bankruptcy`) are no-ops. **These are not stubs.** CONTEXT.md locks that this phase builds the table with all nine phases present and no economics; Phases 5 to 10 replace a `noop` with a real function and change nothing else about the shape. The module doc says so in those terms, so a later reader cannot mistake them for unfinished work. No `TODO`, `FIXME`, skipped test or unrun `<verify>` was left anywhere in this plan.

## Verification Results

Whole-plan block, all re-run at `3ef67d2`:

| Check | Result |
|---|---|
| `cargo test --locked --all-targets` | **0** — 263 passed |
| `cargo test --locked --release --all-targets` | **0** — 261 passed |
| `bash tests/lints.sh` | **0** — final line reports *eleven source guards* |
| `bash tests/toolchain.sh` | **0** |
| `cargo clippy --all-targets --all-features -- -D warnings` | **0** |
| `cargo fmt --check` | **0** |
| Binary vs `config/baseline.toml` into an empty directory | exit 0, 3,651 lines, 0 non-integer fields, 0 empty fields, 0 carriage returns |
| `--seed 42` vs `--seed 43`, first data row | `activation_digest` differs, `rng_draws` identical at 218 |

Per-task grep checks, all as the plan predicted:
`struct Household\|Firm` in `src/world.rs` → **2**; `eleven source guards` → **3** / **4** (ci); `ten source guards` → **0** / **0** (ci); `fn raw_i64_overflow_panics_when_overflow_checks_are_on` → **1**; `tracer effective_seed=` in `src/main.rs` → **0**; `finish` in non-comment `src/main.rs` → **2**; `only property test` in the property file → **1**.

No `<fails_when>` fired at any point.

## Next Phase Readiness

- **03-03 (events, provenance, run record)** — `Sink` currently has two methods; adding `event` and `provenance` is an additive trait change with three implementations to update. `ticks_header`'s derivation pattern is the one `ProvenanceRow` should reuse for its own eager header. `src/main.rs` already loads `_config_sha256`; wiring it into `run_meta.json` is the intended consumer.
- **03-04 (schema)** — the nine column names are frozen and `ticks_header()` is public, so the schema emitter must read them from there and not re-derive them. Note 03-RESEARCH.md Pitfall 10: a semantic-version string in `src/log.rs` will trip the float-literal guard; spell the version without a decimal.
- **03-05 (determinism suite)** — the two cheap binary-level claims already exist in `tests/tracer_end_to_end.rs` at the column level; 03-05's job is the byte level and the cross-process level, and the module doc says the overlap is deliberate. The halt path is verified working: exit 1, rendered violation only, log flushed.
- **03-06 (golden run)** — `/runs` is gitignored; the golden directory must live under `tests/`.
- **Open for later plans:** `TICK-03`, `TICK-08` and `TICK-10` remain Pending in `REQUIREMENTS.md` by design (deviation 4).

---
*Phase: 03-world-tick-pipeline-and-log-seam*
*Completed: 2026-08-31*

## Self-Check: PASSED

All 10 files claimed above exist on disk; all 3 task commits (`c4ab2ff`, `fd791d6`, `3ef67d2`) exist in git and together touch exactly those 10 files.
