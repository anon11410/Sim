---
phase: "3"
slug: "world-tick-pipeline-and-log-seam"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-08-31"
---

# Phase 3 — Validation Strategy

> Derived from `03-RESEARCH.md` § Validation Architecture, which built this entire phase end
> to end on a copy of the repo and measured every claim below rather than reasoning about it.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `libtest`, rustc 1.94.1; `proptest` 1.11.0; **new:** `assert_cmd` 2.2.2 + `tempfile` 3.27.0 for process-level tests |
| **Config file** | `Cargo.toml` `[dev-dependencies]`; `.proptest-regressions/` committed |
| **Quick run command** | `cargo test --locked --lib -- phases` (one filter per run) |
| **Full suite command** | `cargo test --locked --all-targets && cargo test --locked --release --all-targets && bash tests/lints.sh && bash tests/toolchain.sh && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` |
| **Measured runtime** | debug 2.9 s · release 18.3 s cold · lints 4.6 s · toolchain 0.08 s · clippy 0.26 s warm |

**This phase adds two dev-dependencies** — the first since Phase 1. `assert_cmd` and
`tempfile` are needed for TICK-09's cross-process half and criterion 6's binary-level halt,
neither of which a library test can reach. CI runs `--locked`, so the lockfile update lands in
the same commit. No new test runner, no new framework, no new CI step.

**The release run is not a duplicate.** TICK-08's 3,650-tick decade and the
`debug_and_release_agree` claim are both about the release binary.

---

## Sampling Rate

- **After every task commit:** `cargo test --locked --lib -- phases` (sub-second). **Corrected:** the
  three-word form `--lib phases world log` is the recorded invalid-syntax defect — `cargo test`
  takes one positional TESTNAME and exits non-zero with `unexpected argument` *without running
  anything*. Caught by the Phase 3 planner reading this file.
- **After every plan wave:** both profiles, `--all-targets`
- **Before `/gsd-verify-work`:** the full six-step suite, matching CI exactly
- **Max feedback latency:** ~5 s warm

---

## Per-Requirement Verification Map

| Req ID | Behavior | Test Type | Automated Command | File | Status |
|---|---|---|---|---|---|
| TICK-01 | `PHASES` runs the exact 9-name sequence; a tenth phase cannot be added without placing it | unit (in-module, `ALL_CHECKS` pattern) | `cargo test --locked --lib phases::order` | ❌ W0 | ⬜ pending |
| TICK-01 | Each phase completes for all agents before the next | unit | `cargo test --locked --lib phases::order::each_phase_is_a_full_loop` (structural — assert `PhaseFn` takes `&mut Ctx` and no per-agent step exists) | ❌ W0 | ⬜ pending |
| TICK-02 | Generated schema equals the committed file | integration | `cargo test --locked --test log_schema schema_matches_the_committed_file` | ❌ W0 | ⬜ pending |
| TICK-02 | Drift test **fires** — mutation | integration (negative) | `bash tests/schema_drift_negative.sh` (perturb, expect fail, revert under `trap`) | ❌ W0 | ⬜ pending |
| TICK-03 | `ticks.csv` header, column order and integer-only dtypes | integration | `cargo test --locked --test log_schema ticks_csv_is_flat_and_integer_only` | ❌ W0 | ⬜ pending |
| TICK-03 | 3,650 rows, `\n` terminator, no CRLF, no empty field | integration | `cargo test --locked --test determinism the_run_directory_is_well_formed` | ❌ W0 | ⬜ pending |
| TICK-04 | Every `Event` variant round-trips and appears in the schema | unit | `cargo test --locked --lib log::events` | ❌ W0 | ⬜ pending |
| TICK-04 | Endowment events sum to `total_money_cents` | integration | `cargo test --locked --test determinism endowment_events_sum_to_the_money_stock` | ❌ W0 | ⬜ pending |
| TICK-05 | `run_meta.json` carries seed, config hash, rustc | integration | `cargo test --locked --test determinism run_meta_carries_the_three_fields` | ❌ W0 | ⬜ pending |
| TICK-06 | No path, hostname, PID or timestamp in any diffed file | integration | `cargo test --locked --test determinism the_exclusion_is_enforced_not_documented` | ❌ W0 | ⬜ pending |
| TICK-06 | Halt message carries no environment (source half) | shell guard | `bash tests/lints.sh` (guard 7h, already exists; extend its file set to `src/log.rs`, `src/phases.rs`, `src/world.rs`) | ⚠️ extend | ⬜ pending |
| TICK-07 | `provenance.csv` exists with the exact 7-column header and 0 rows | integration | `cargo test --locked --test log_schema provenance_has_a_header_even_with_no_rows` | ❌ W0 | ⬜ pending |
| TICK-08 | 3,650 empty ticks execute; invariants pass; run directory complete | integration | `cargo test --locked --release --test determinism the_empty_decade_runs` | ❌ W0 | ⬜ pending |
| TICK-09 | Same seed → identical, in-process | integration | `cargo test --locked --test determinism same_seed_identical_in_process` | ❌ W0 | ⬜ pending |
| TICK-09 | Same seed → identical, cross-process | integration (`assert_cmd`) | `cargo test --locked --test determinism two_processes_at_one_seed_write_identical_bytes` | ❌ W0 | ⬜ pending |
| TICK-09 | Debug and release bytes agree | integration | `cargo test --locked --release --test determinism debug_and_release_agree` (or a CI step) | ❌ W0 | ⬜ pending |
| TICK-10 | **Different seed → different `ticks.csv`** | integration | `cargo test --locked --test determinism different_seed_differs` | ❌ W0 | ⬜ pending |
| TICK-10 | The counter-check has teeth — mutation | manual-then-recorded | Blank the `activation_digest` column, confirm `different_seed_differs` **fails**, revert. **Already performed in this research pass** — record the result in the plan's SUMMARY as Phase 2 did. | n/a | ⬜ pending |
| Criterion 6 | Binary exits non-zero, stderr names tick 0 | integration (`assert_cmd`) | `cargo test --locked --test determinism the_binary_halts_on_a_liveness_violation_at_tick_zero` | ❌ W0 | ⬜ pending |
| Criterion 7 | `Household` / `Firm` carry no balance, expose no `set_cash` | shell guard (3 clauses) | `bash tests/lints.sh` (guard `7f-agents`) | ❌ W0 | ⬜ pending |
| Criterion 7 | The guard **fires** on all three hazard shapes | shell guard fixtures | built into the guard via `assert_fires` / `assert_ignores` — **already mutation-proved in this research pass** | ❌ W0 | ⬜ pending |
| Golden | 50-tick run reproduces byte-identically | integration | `cargo test --locked --test determinism the_golden_run_reproduces` | ❌ W0 | ⬜ pending |
*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## The Honest Unit / Property / Integration Split

Recorded deliberately so the plan-checker does not read the absence of property tests as a gap.

- **Unit** (`--lib`, no filesystem): the `PHASES` order triple, `Event`/`TickRow` wire shapes
  via `VecSink`, `schema_json()` self-consistency, `order_digest` sensitivity.
- **Integration** (`tests/`, writing to a `tempfile::tempdir`): everything about *files* —
  TICK-03, -05, -06, -08, -09, -10 and criterion 6. These need a real run directory, and two
  of them need a real process.
- **Property (`proptest`): exactly one thing earns it** — `order_digest` mapping distinct
  permutations to distinct digests. **Do not add proptest cases for the log types.** Everything
  else here is a fixed table or a fixed file shape, where a generated input domain would
  produce inputs the model never emits. The phase's risk lives in the file *bytes*, not in an
  input domain.
- **Shell guard** (`tests/lints.sh`): criterion 7's `7f-agents`, and guard 7h's file-set
  extension.

---

## Two Checks That Must Be Mutation-Proved, Not Merely Green

Phase 2 produced five defects of one shape — an assertion whose stated claim is not what it
measures — and its 242-test suite was green through every one. Two checks in this phase have
that exact hazard, and **the research already proved both by execution**:

1. **TICK-10, the reproducibility counter-check.** The mechanism the ROADMAP and the first
   draft of `03-CONTEXT.md` prescribed — an activation-order shuffle plus a per-tick
   `rng_draws` column — was built and run at seeds 42 and 43 over 3,650 ticks: `cmp` returned
   **byte-identical**. The draw count is a constant; it proves draws happened and says nothing
   about the seed. Adding an `activation_digest` column (sha256 of the tick's permutation)
   flipped the same test to differing at tick 0. **The plan must amend ROADMAP criterion 3**,
   in the inline-rationale shape plan 02-01 used for the localisation clause, and must record
   the blank-the-digest mutation in its SUMMARY.
2. **TICK-02's schema drift test.** A drift test that compares a generated file to itself
   passes forever. `tests/schema_drift_negative.sh` perturbs the committed schema, expects
   failure, and reverts under a `trap`.

Related, and the reason the empty run is not self-verifying: **empty artifacts pass every
test.** `provenance.csv` and `events.jsonl` both write at 0 bytes, and a cross-process hash
comparison over two empty files compares the sha256 of the empty string with itself. Closed by
an eager CSV header (`has_headers(false)` — the obvious spelling emits the header twice) and by
emitting the opening endowment as events from `books.accounts()`, which is also the origin row
Phase 4's conservation replay needs: 220 rows summing to exactly 2,000,000 cents.

---

## Wave 0 Requirements

- [ ] `Cargo.toml` + `Cargo.lock` — add `csv`, `serde_json`, `assert_cmd`, `tempfile`
- [ ] `src/world.rs` — `World`, `Household`, `Firm` (criterion 7's subject)
- [ ] `src/phases.rs` — `Ctx`, `PhaseId`, `PHASES`, `tick`, `run`, in-module `order` tests
- [ ] `src/log.rs` — `Sink`, `TickRow`, `Event`, `ProvenanceRow`, `RunWriter`, `schema_json`
- [ ] `build.rs` — `SIM_RUSTC_VERSION`
- [ ] `src/main.rs` — rewritten CLI, `--dump-schema`
- [ ] `schema/schema.json` — generated and committed
- [ ] `tests/determinism.rs`, `tests/log_schema.rs`, `tests/golden/` (50-tick run)
- [ ] `tests/lints.sh` — guard `7f-agents`, guard 7h file-set extension, **and the check-count
      prose at all FOUR sites that currently say "ten source guards"**: the preamble comment
      (line 28), the check-7 section header (line 377, capitalised `Ten`), the check-7 summary
      line (783) and the final `OK:` line (785). Three are lowercase and one is not, so the
      case-sensitive greps in plan 03-02 must be paired with case-insensitive siblings — expected
      counts after the edit are `3` lowercase and `4` case-insensitive, not `2`
- [ ] `tests/tracer_end_to_end.rs` — **port `runs_end_to_end` and `different_seed_changes_the_draw`**;
      the research measured that rewriting `main.rs` silently drops these two tests. Keep the
      overflow tests.

---

## Manual-Only Verifications

All phase behaviors have automated verification. This is pure infrastructure — no UI, no
network, no external service, no user-observable behaviour.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
