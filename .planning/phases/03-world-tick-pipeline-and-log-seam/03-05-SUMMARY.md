---
phase: 03-world-tick-pipeline-and-log-seam
plan: 05
subsystem: infra
tags: [rust, determinism, assert_cmd, tempfile, mutation-testing, tick-06, reproducibility, invariant-halt]

# Dependency graph
requires:
  - phase: 01-spine-config-rng-money
    provides: sim::config::config_hash (the byte digest reused here), sim::config::load, the seeded ChaCha8 generator, tests/tracer_end_to_end.rs's column-level binary tests
  - phase: 02-ledger-and-invariants
    provides: Books::new clearing the endowment postings (which is why the halt lands at tick 0), CheckSet, the liveness check, the library-level half of criterion 6
  - phase: 03-02
    provides: TickRow, ticks_header, RunWriter, phases::run, the real CLI and its run record
  - phase: 03-03
    provides: Event::Endowment and endowment_events, ProvenanceRow, Decision::ALL, Rule::ALL, provenance_header, the eager CSV header
  - phase: 03-04
    provides: sim::log::schema_json — the generator this file derives its closed word vocabulary from
provides:
  - "tests/determinism.rs — nine tests: three reproducibility claims, the enforced exclusion, the run-directory shape, the decade, the endowment sum, the run record, and the process-level halt"
  - "EXCLUDED_FROM_DIFF — the one-entry exclusion list, spelled from sim::log::RUN_META_FILE; the single place a future phase declares that a run-directory file is not diffed"
  - "The measured refusal of a digit-substring process-identifier search (42.0% five-digit coincidence rate on a real ticks.csv), and the four clauses that cover the same ground soundly"
affects: [03-06, phase-04-analysis-harness, phase-06-labour]

actuals:
  tokens: 11789   # chars/4 over the realized diff (47,157 added chars), NOT a harness token count
  tasks: 3
  commits: 3

tech-stack:
  added: []   # assert_cmd and tempfile were already dev-dependencies; this plan adds no package
  patterns:
    - "Route every file read in a determinism test through one non-empty-asserting helper, so 'the inputs to this comparison had content' is structural rather than remembered"
    - "Enumerate the diffed set from the run directory and subtract a named exclusion constant; assert the count actually diffed, never a literal"
    - "Assert the excluded file EXISTS as part of asserting it is excluded — a vacuous exclusion is indistinguishable from one doing its job"
    - "Derive a closed word vocabulary from the generator, never from the artifact under test, then assert every alphabetic run in the artifact is a member"
    - "Self-check every scanner against a fabricated leak in the same test body, before trusting it on the real files"
    - "Refuse a check whose measured false-positive rate makes it a coin flip; say the measured number in the code and cover the ground another way"

key-files:
  created:
    - tests/determinism.rs
  modified: []

key-decisions:
  - "The process-identifier clause is NOT a substring search for the identifier's digits, and the refusal is measured rather than asserted. On a real 3,650-tick ticks.csv, 42.0% of all five-digit numbers occur somewhere in the file by coincidence (4.9% of six-digit, 0.47% of seven-digit); this host's pid_max is 32768, so every identifier is at most five digits. Such a search is red against a CORRECT simulation roughly two runs in five, and what it measures is digit coincidence, not information disclosure. This is the one place this plan departs from its own text, and the departure is toward a stronger check."
  - "Process identity is instead covered by byte-equality across two SPAWNED processes whose identifiers are asserted to differ. A process identifier, a per-run directory name and a clock reading all vary between the two runs; every diffed file being byte-equal is therefore proof none of them reached a diffed byte, with no false positives at all."
  - "The alphabetic half of TICK-06 is a closed vocabulary derived from sim::log::schema_json — from the GENERATOR, never from a run's output. A vocabulary read out of the artifact under test would contain whatever leaked into it and would permit exactly the thing it exists to catch."
  - "Decision::ALL and Rule::ALL are folded into that vocabulary. The schema declares the provenance decision and rule COLUMNS but not the names they hold; without this the clause fires on the first correct provenance row Phase 6 writes, and a guard that goes red on correct output is one the next reader loosens."
  - "The two run directories are named `first` and `second`, not `a` and `b`. The schema vocabulary legitimately contains the single letters `a`, `b` and `v`, so directories with those names would have been admitted by the vocabulary clause; `first` and `second` are not in it, so a directory-name leak is caught by two clauses rather than one."
  - "DECADE_TICKS = 3650 is a literal, and it is the only one in the file. TICK-08 is a claim about a decade; a test that read the run length out of the same configuration it is exercising would certify whatever that file said, including a shortened run left behind while debugging. The literal is compared AGAINST the configuration, so a deliberate change fails here and gets reconsidered."
  - "different_seed_differs asserts on ticks.csv specifically, with the reason in a comment. At this phase events.jsonl carries only the seed-independent opening endowment — measured, both seeds produce the same digest 37ce4a1d… — so 'every diffed file differs' would be red against a correct simulation."
  - "The halt test asserts the shipped configuration's line count and its 44 `# GRADE:` comments survive the override. The leaf assertions alone would pass against a re-serialisation of the parsed parameters, which works and strips every comment — and the comments carry the source grades tests/provenance.rs makes load-bearing."
  - "The exclusion test spawns the binary through std::process::Command rather than the assertion builder, because the builder does not surface the child's identifier and clause 4's reasoning needs it. The binary is the same artifact: CARGO_BIN_EXE_sim is set by cargo to the binary built for this test run, not a path assembled into the target directory."

patterns-established:
  - "A determinism comparison is guilty until its inputs are shown non-empty; route the reads through one helper so the clause cannot be forgotten."
  - "A guard whose false-positive rate has not been measured is not a guard. Measure it, and if it is a coin flip, say the number in the source and cover the ground differently."
  - "Scanners self-check against a fabricated leak in the same test body — a scanner that matches nothing passes over everything."

requirements-completed: [TICK-05, TICK-06, TICK-08, TICK-09, TICK-10]

coverage:
  - id: D1
    description: "Two runs at one seed produce byte-identical logs inside one process"
    requirement: TICK-09
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism same_seed_identical_in_process — three files, each read through read_nonempty, each digest-compared"
        status: pass
      - kind: other
        ref: "Mutation: one extra row appended to the second run → red on the digest comparison (052499a8… against 04a09662…). Mutation: both event streams zeroed → red on the non-empty clause instead, which is the clause ordering the test depends on. Both reverted."
        status: pass
    human_judgment: false
  - id: D2
    description: "Two invocations of the built binary at one seed write identical bytes — the claim an in-process comparison cannot make"
    requirement: TICK-09
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism two_processes_at_one_seed_write_identical_bytes"
        status: pass
      - kind: other
        ref: "Mutation: the second process run at seed 43 → red, and the failure printed events.jsonl hashing EQUAL at both seeds (37ce4a1d…), which is the measured basis for D3's scope note. Reverted."
        status: pass
    human_judgment: false
  - id: D3
    description: "A different seed produces a different ticks.csv, localised to the activation digest column"
    requirement: TICK-10
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism different_seed_differs — file digests differ; rng_draws is asserted EQUAL at both seeds and activation_digest asserted different"
        status: pass
      - kind: other
        ref: "Mutation: activation_digest blanked to 0 in src/phases.rs:150 → red, both seeds hashing to 9deecfb9c9fe5ff588004de9a56d6854e8303364f9a6a927f5494800e055cabd. Reverted; suite green."
        status: pass
    human_judgment: false
  - id: D4
    description: "The run-record exclusion is enforced rather than documented: same file set, the excluded file exists, everything else enumerated from the directory is non-empty and digest-equal, and the diffed count is asserted against the directory minus the exclusion list"
    requirement: TICK-05
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism the_exclusion_is_enforced_not_documented — 4 entries, 1 excluded, 3 diffed"
        status: pass
      - kind: other
        ref: "Mutation: main.rs stops writing the run record → red with `run_meta.json is excluded from the diff but was never written`. Mutation: ticks.csv added to EXCLUDED_FROM_DIFF → red with `only 2 files were diffed`. Both reverted."
        status: pass
    human_judgment: false
  - id: D5
    description: "No diffed file carries a path, a host name, a process identifier or a timestamp"
    requirement: TICK-06
    verification:
      - kind: integration
        ref: "the_exclusion_is_enforced_not_documented clause 4 — a closed vocabulary from schema_json plus Decision::ALL and Rule::ALL; no path separator and neither known path; no dddd-dd-dd or dd:dd:dd shape; and byte-equality across two processes with asserted-different identifiers"
        status: pass
      - kind: other
        ref: "Mutation: the word `buildhost` written into BOTH runs' provenance.csv (so the digests still agree) → red on the vocabulary clause. Mutation: `2026-08-31` written into both → red on the date shape. Both reverted."
        status: pass
      - kind: other
        ref: "Measured false-positive rate of the rejected digit-substring identifier search: 42,032 of 100,000 five-digit numbers, 49,422 of 1,000,000 six-digit, 46,915 of 10,000,000 seven-digit occur by coincidence in one real ticks.csv; pid_max = 32768"
        status: pass
    human_judgment: false
  - id: D6
    description: "ticks.csv is one header line plus one row per configured tick, terminated with a bare newline, with no empty field and every field an integer"
    requirement: TICK-03
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism the_run_directory_is_well_formed — header compared against ticks_header(), 3,650 rows × 9 fields, provenance header against provenance_header(), every events.jsonl line a tagged JSON object"
        status: pass
      - kind: other
        ref: "Mutation: every terminator rewritten as CRLF → red with `a carriage return reached the tick file`. Mutation: one field emptied → red with `row 0 has an empty total_money_cents field`. Both reverted."
        status: pass
    human_judgment: false
  - id: D7
    description: "3,650 empty ticks execute in the release binary with every invariant passing and a complete run directory produced"
    requirement: TICK-08
    verification:
      - kind: integration
        ref: "cargo test --locked --release --test determinism the_empty_decade_runs — the configuration is asserted to ask for 3,650; the exit code carries the invariant claim; the directory is compared against the set of files the library and binary declare; the record reports 3650 completed and `ok`"
        status: pass
      - kind: other
        ref: "Mutation: provenance.csv deleted from the run directory → red, printing the three names it found against the four declared. Reverted."
        status: pass
      - kind: other
        ref: "Release binary timed directly: 0.014s for the decade, 3,651 lines in ticks.csv"
        status: pass
    human_judgment: false
  - id: D8
    description: "The endowment records sum in cents to the tick series' money column — the origin row Phase 4's conservation replay is anchored to"
    requirement: TICK-04
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism endowment_events_sum_to_the_money_stock — 220 records, 2,000,000 cents, compared against total_money_cents read out of the tick file's first data row; neither side a literal"
        status: pass
      - kind: other
        ref: "Mutation: one endowment record's cash_cents reduced by a cent → red with `the 220 endowment records sum to 1999999 cents, while the tick series opens at 2000000`. Reverted."
        status: pass
    human_judgment: false
  - id: D9
    description: "The run record carries the effective seed, the configuration digest and the compiler string, and nothing the diff exclusion is not a licence for"
    requirement: TICK-05
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism run_meta_carries_the_three_fields — the recorded seed equals the seed passed; the recorded digest equals a digest this test takes of the same file; no key contains duration/elapsed/host/pid/process/path/dir/user; no string value carries a path separator"
        status: pass
      - kind: other
        ref: "Mutation: a duration_ms field added to RunMeta in src/main.rs → red with `the run record carries a duration_ms field`. Reverted."
        status: pass
    human_judgment: false
  - id: D10
    description: "The built binary halts non-zero at tick 0 with the liveness gate overridden through a configuration file, and its message carries no path"
    requirement: ROADMAP Phase 3 criterion 6
    verification:
      - kind: integration
        ref: "cargo test --locked --test determinism the_binary_halts_on_a_liveness_violation_at_tick_zero, debug and release — exactly one leaf asserted before substitution, exit code 1 specifically, stderr naming tick 0 and liveness, no path in it, run record showing 0 ticks and `violation`, a header-only tick file"
        status: pass
      - kind: other
        ref: "Mutation: the shipped leaf reworded to `liveness_enabled=false` → red, 0 against 1. Mutation: the shipped file written instead of the overridden one → red with assert_cmd's `Unexpected success, code=0` — the criterion-1-twice failure. Mutation: the run directory eprintln'd beside the violation → red on the no-path clause, quoting the leaked message. All reverted."
        status: pass
      - kind: other
        ref: "grep -cE '(env::set_var|SIM_[A-Z_]*=)' tests/determinism.rs → 0; grep -c 'liveness_enabled = false' tests/determinism.rs → 1"
        status: pass
    human_judgment: false

# Metrics
duration: ~10min
completed: 2026-08-31
status: complete
---

# Phase 3 Plan 05: Reproducibility Proved on Bytes Summary

**Nine tests that were each watched failing on the defect they name — including the one that measured 42% of five-digit numbers occurring by coincidence in a real `ticks.csv` and therefore refused the obvious spelling of the process-identifier check rather than shipping a coin flip.**

## Performance

- **Duration:** ~10 min (commit span 15:35:54 → 15:44:59 UTC, plus the whole-plan verification block)
- **Completed:** 2026-08-31
- **Tasks:** 3 of 3
- **Files created/modified:** 1 (`1,108 insertions(+), 0 deletions(-)`)
- **Test count:** 299 → **308** in debug (+9), 297 → **306** in release — the two-test gap is the pre-existing `#[cfg(debug_assertions)]` sub-stream re-entry pair in `src/rng.rs`, unchanged by this plan
- **Wall time of this file's own passes:** **1.85 s** debug, **0.06 s** release (13 decade-long runs across the nine tests; the release binary does a decade in 0.014 s, the debug binary in 0.553 s)

## Task Commits

1. **Task 1: The three reproducibility claims** — `5fe9a73` (test)
2. **Task 2: The exclusion enforced, and the decade that has to actually run** — `3d3d7ac` (test)
3. **Task 3: The process-level halt** — `0681979` (test)

## The exclusion, exactly

| Fact | Value |
|---|---:|
| Entries in a run directory | **4** |
| Excluded from the diff | **1** — `run_meta.json` |
| Files actually diffed | **3** — `ticks.csv`, `events.jsonl`, `provenance.csv` |
| Diffed count asserted as | `files.len() - EXCLUDED_FROM_DIFF.len()`, and `>= 3` |

The count is never a literal. A file a later phase adds is diffed automatically, or declared in `EXCLUDED_FROM_DIFF` deliberately; it cannot be skipped by omission from a hand-written list, which is the failure this clause exists to prevent.

The excluded file is asserted to **exist**. Excluding a file that was never written is a vacuous exclusion — it enforces nothing, and it is indistinguishable from one doing its job. Watched: with `write_run_meta` disabled in `src/main.rs` the test goes red with `run_meta.json is excluded from the diff but was never written`.

## The endowment anchor

```
220 endowment records → 2,000,000 cents
ticks.csv first data row, total_money_cents → 2,000,000
```

Two independently produced numbers, neither of them a literal: one parsed out of `events.jsonl`, the other read by column name out of `ticks.csv`. This is the origin row HARN-02's conservation replay is defined against, so it is checked here rather than assumed in Phase 4. Removing one cent from one record makes the test red with `the 220 endowment records sum to 1999999 cents, while the tick series opens at 2000000`.

## The halt, exactly

Exit code **1**, and standard error carries precisely one line:

```
tick 0: liveness — 0 transactions recorded, at least 1 required; no posting, which is the violation
```

No path, no host name, no process identifier, no wall clock — TICK-06 at the message level, which is the runtime half of lint guard 7h. The run record is still written and says `"ticks_completed": 0, "exit": "violation"`, and `ticks.csv` is present and header-only: a halted run leaves an openable file rather than a zero-byte one.

`03-RESEARCH.md` recorded this line with an `INVARIANT VIOLATION: ` prefix. The prefix is not in the shipped rendering and the test does not require it — it asserts on `tick 0` and on `liveness`, which is what criterion 6 actually claims.

The override moved **exactly one** leaf, asserted before substituting, and the file's **44** `# GRADE:` comments and its line count are asserted unchanged afterwards.

## The blank-the-digest mutation

Recorded in the manner Phase 2 recorded its mutation proofs, because `different_seed_differs` is the test whose value depends entirely on having been seen red:

| | Result |
|---|---|
| `src/phases.rs:150` → `activation_digest: 0` | `different_seed_differs` **FAILS** |
| Seed 42 digest of `ticks.csv` | `9deecfb9c9fe5ff588004de9a56d6854e8303364f9a6a927f5494800e055cabd` |
| Seed 43 digest of `ticks.csv` | `9deecfb9c9fe5ff588004de9a56d6854e8303364f9a6a927f5494800e055cabd` |
| Reverted, suite | green |

The two seeds hash to the same value. That is the vacuous-reproducibility trap the research measured against the mechanism ROADMAP criterion 3 originally prescribed: the generator is consumed — 218 draws every tick, at every seed — and nothing it touches reaches a diffed byte. A `rng_draws` count column proves draws occurred and says nothing about which draws. The `activation_digest` column is the repair, and this mutation is the standing proof the column is load-bearing rather than decorative.

The measurement is preserved a second way. When the cross-process test was mutated to run its second process at another seed, the failure printed both files' digests: `ticks.csv` differed and `events.jsonl` hashed **equal** (`37ce4a1d1a1928f3044b0372b13f2e204e4fe462d62efc718f6e8f1a3be4ce41`) at seeds 42 and 43. That is the measured reason `different_seed_differs` asserts on the tick file specifically — an assertion that every diffed file differs would be red against a correct simulation at this phase.

## Deviation: the process-identifier check

**The plan asked for a substring search of each diffed file's bytes for the running process's identifier. That check was refused, and something stronger was written instead.** This is the only departure from the plan's text.

**Measured on a real 3,650-tick `ticks.csv`** (202,974 bytes of comma-separated integers, including a 19-digit `activation_digest` per row):

| Identifier width | Distinct values occurring by coincidence | Rate |
|---|---:|---:|
| 5 digits | 42,032 of 100,000 | **42.0%** |
| 6 digits | 49,422 of 1,000,000 | 4.9% |
| 7 digits | 46,915 of 10,000,000 | 0.47% |

`/proc/sys/kernel/pid_max` on this host is **32768**, so every process identifier here is at most five digits. A substring search would therefore be **red against a correct simulation roughly two runs in five**, and what it would be measuring is digit coincidence, not information disclosure. That is precisely the class of defect this plan exists to close — an assertion whose stated claim is not what it measures — so writing it would have added a tenth instance rather than closing any.

The same ground is covered by four clauses that cannot false-positive, and each was watched firing:

1. **Anything alphabetic** — every run of ASCII letters in a diffed file must be a word the wire format declares. The vocabulary comes from `sim::log::schema_json()` (the generator, never the artifact under test) plus `Decision::ALL` and `Rule::ALL`. A host name, a user name, a path component or a month name is not in it. *Watched: `buildhost` written into **both** runs' `provenance.csv` — so the digests still agree and byte-equality is blind to it — goes red.*
2. **Anything path-shaped** — no `/` or `\` byte, and neither the temporary root nor the repository root as a substring. Both are distinctive strings known to the test.
3. **Anything timestamp-shaped** — no `dddd-dd-dd` and no `dd:dd:dd` byte shape. Neither is constructible from comma-separated integers, so a match is a finding rather than a coincidence. *Watched: `2026-08-31` written into both runs goes red.*
4. **Anything that varies per process** — the two runs are two spawned processes, with **asserted-different identifiers**, writing into differently named directories. Every diffed file is byte-equal across them, so a process identifier, a per-run directory name or a clock reading cannot have reached a diffed byte. The identifiers are asserted to differ so this reasoning is not vacuous.

The one residual gap is named in the source rather than papered over: a wall clock recorded as a bare integer in a numeric column would evade clauses 1–3 and could evade 4 if two runs fell in the same second. It is closed by the header assertion in `the_run_directory_is_well_formed` — the tick file's header must equal `ticks_header()`, and a clock would need a column that is not declared.

## A real defect the mutations caught

The first draft derived the closed vocabulary from `schema_json()` alone. Mutation F — a fabricated provenance row — went red on the word **`held`**, which is a legitimate `Rule` value. The schema declares the provenance table's `decision` and `rule` **columns**; the names those columns hold are declared by the enumerations instead. This phase writes zero provenance rows, so the omission cost nothing today and would have made the clause fire on the first correct row Phase 6 writes — and a guard that goes red on correct output is one the next reader loosens, which is how a clause like this stops catching a host name. `Decision::ALL` and `Rule::ALL` are now folded in, with a self-check asserting they arrived.

## What the nine tests are, and what each one is for

| Test | Claim | Requirement |
|---|---|---|
| `same_seed_identical_in_process` | The library reproduces | TICK-09 |
| `two_processes_at_one_seed_write_identical_bytes` | No global state, environment read or allocator-order effect survives a fork | TICK-09 |
| `different_seed_differs` | The seed reaches a written byte | TICK-10 |
| `the_exclusion_is_enforced_not_documented` | The exclusion means something, and nothing environmental is diffed | TICK-05, TICK-06 |
| `the_run_directory_is_well_formed` | One header, 3,650 integer rows, `\n`, no empty field | TICK-03 |
| `the_empty_decade_runs` | A decade runs clean and leaves a complete directory | TICK-08 |
| `endowment_events_sum_to_the_money_stock` | Phase 4's replay origin agrees with the series it explains | TICK-04 |
| `run_meta_carries_the_three_fields` | The record is complete, and quarantine is not a licence | TICK-05 |
| `the_binary_halts_on_a_liveness_violation_at_tick_zero` | The process halts, exit 1, naming tick 0, with no path | Criterion 6 |

## Division of labour with `tests/tracer_end_to_end.rs`

Deliberate overlap, recorded in both files' module docs. The tracer tests assert at the **column** level and have existed since Phase 1; a failure there says *which* column stopped depending on the seed. These assert at the **file-byte** level and across a **process boundary**; they cover every column including ones nobody thought to name, and they see the class of defect an in-process comparison is blind to by construction. Neither subsumes the other.

## Note on `debug_and_release_agree`

`03-VALIDATION.md` lists a cross-profile agreement row. It is **not** implemented here as a test that spawns a build of the other profile — invoking cargo from inside a cargo-run test deadlocks on the build lock. It is delivered by plan 03-06's golden-run test, which compares the current profile's output against one committed artifact and is run by CI in **both** profiles, so a disagreement between profiles makes one of the two passes red. Both profiles of this file are green today: 9 passed in each.

## Whole-plan verification

| Check | Result |
|---|---|
| `cargo test --locked --test determinism` | 9 passed |
| `cargo test --locked --release --test determinism` | 9 passed |
| `cargo test --locked --all-targets` | **308** passed |
| `cargo test --locked --release --all-targets` | **306** passed |
| `bash tests/lints.sh` | OK — 60 method bans fire, both compile-fail probes refused, eleven source guards silent on a clean tree |
| `bash tests/toolchain.sh` | OK |
| `bash tests/schema_drift_negative.sh` | OK — drift test observed failing on the perturbed schema and passing after the restore |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `git status --porcelain` after the suite | empty — no test left an artifact behind |

## For Phase 4

- `EXCLUDED_FROM_DIFF` is the one place to declare a new run-directory file as undiffed. Adding a file without touching it means the file gets diffed, which is the intended default.
- The standing check the research recommends handing over: `assert df.activation_digest.nunique() > 1` over a run. The measured value today is 3,650 distinct values in 3,650 rows; `rng_draws` is the constant 218, and its constancy is itself worth asserting — a tick whose draw count moved is a fixed-draw-sampling violation.
- The endowment anchor is checked here: 220 records, 2,000,000 cents, equal to `total_money_cents`. The conservation replay can rely on it rather than re-establish it.
- A header-only `provenance.csv` gives every column dtype `object`, not `int64`. The dtype assertion must be conditional on a non-empty frame, or read from `schema/schema.json`.

## Self-Check: PASSED

- `tests/determinism.rs` — present
- `.planning/phases/03-world-tick-pipeline-and-log-seam/03-05-SUMMARY.md` — present
- Commits `5fe9a73`, `3d3d7ac`, `0681979` — all present in the history
