---
phase: 02-books-journal-and-invariants
plan: 02
subsystem: ledger
tags: [books, journal, posting, invariants, conservation, liveness, thiserror, serde]

# Dependency graph
requires:
  - phase: 01-primitives-and-the-determinism-spine
    provides: "`Money` with its split overflow API, `Account`/`HouseholdId`/`FirmId`/`FirmSlot`/`GoodId`, `Params` and `config::load`, `#![forbid(unsafe_code)]`, the release profile's overflow checks"
  - phase: 02-books-journal-and-invariants
    plan: 01
    provides: "`invariants.liveness_enabled` as a required config key with no serde default, and the five address `Display` impls a halt message interpolates"
provides:
  - "`sim::books::Books` — every cent in the simulation, behind private fields, with one constructor and one cash-mutation point"
  - "`sim::books::Books::transfer` — compute-then-commit, no callback of any kind, returns the amount actually moved"
  - "`sim::books::Posting` / `PostingKind` — the journal line, with two cash legs, two units legs and both running residuals, `Serialize` with addresses rendered through `Display`"
  - "`sim::books::PostError` / `BooksError` — a refused posting and a refused construction, as separate `thiserror` types"
  - "`sim::invariants::CheckSet` — the ordered check phase, built once from the parameters, returning `Result`"
  - "`sim::invariants::Violation` / `CheckId` / `CheckFn` / `ALL_CHECKS` / `MINIMUM_TRANSACTIONS_PER_TICK`"
  - "`tests/invariant_halt.rs` — the end-to-end proof that a tick recording no transaction aborts the loop with the gate on and does not with it off"
affects: [02-03, 02-04, 02-05, 02-06, 02-07, phase-03-tick-pipeline, phase-06-labour-market, phase-08-dividends, phase-10-bankruptcy]

actuals:
  tokens: 14300
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Compute-then-commit: every fallible step of a `&mut Books` method completes before the first write, and the commit step is assignments plus the recorder"
    - "A journal line carries BOTH legs of each amount (`debit_cents`/`credit_cents`, `units_out`/`units_in`), so a non-conserving posting is expressible as data and the per-posting check is not a tautology"
    - "Running residuals are stamped onto each posting incrementally, so localisation is a forward scan over already-computed numbers and never a replay"
    - "Two independent sources per conservation check — the balance vectors and the journal residual — neither derived from the other"
    - "A configuration gate is read once at construction and filtered into a `Vec` of active checks, so the per-tick path carries no branch on it"
    - "A wire shape is pinned in the module that owns it: `Posting`'s serde derive renders addresses through their `Display` form via `serialize_with`, leaving `src/ids.rs` uncommitted to a serde representation"

key-files:
  created:
    - src/books.rs
    - src/invariants.rs
    - tests/invariant_halt.rs
  modified:
    - src/lib.rs

key-decisions:
  - "`Posting` serialises addresses through `serialize_with` + `Display` (`\"household:12\"`, `\"firm:3:0\"`) rather than deriving `Serialize` on the `src/ids.rs` newtypes — the event stream stays greppable by agent, and Phase 3 inherits a decided wire shape instead of an accidental structural encoding"
  - "Balances key on `FirmSlot`, postings key on the full `Account` identity; `Books` carries its own `firm_generation` vector so a stale firm identity is a typed `UnknownAccount` miss and never a silent hit on the successor"
  - "`Books::new` clears the journal after endowing, so tick 0 begins empty — otherwise the liveness check could pass on the strength of the endowment alone, the exact degenerate pass LEDG-08 exists to close"
  - "The private recorder takes a whole `Posting` draft rather than nine parameters; `seq` and both residuals are placeholders in and are stamped on the way out"
  - "`MINIMUM_TRANSACTIONS_PER_TICK` is a module constant, not a config key: 'at least one' is the definition of the check, and a minimum whose zero value meant 'disabled' would be a hidden second switch"
  - "`Violation::MoneyConservation` carries `Option<Posting>` and renders the `None` case in its own terms — a write outside the posting path leaves every residual at zero and there genuinely is no offending posting to name"
  - "Localisation is `iter().find(|p| p.cash_residual_cents != 0)` with the cancelling-residual counterexample (broken #50, healed #120, broken #200) recorded in the doc comment, so the next reader cannot 'optimise' it into a search over halves"

patterns-established:
  - "LEDG-02's four legs: exclusive borrow, no callback on any mutable-borrow method, no shared-mutability wrapper, compute-then-commit — each stated in the module docs where a later author will read it"
  - "A refusal writes nothing: every `PostError` path is proved to leave the books byte-identical, asserted against a clone taken before the attempt"
  - "Violations are asserted by whole-value equality against a constructed `Violation`; exactly one test asserts the rendered message, and it tests a different claim"

requirements-completed: [LEDG-01, LEDG-02, LEDG-03, LEDG-04, LEDG-08, LEDG-09, LEDG-10]

coverage:
  - id: D1
    description: "`Books` owns every cent behind private fields with one constructor, and `transfer` is the only cash-mutation point — compute-then-commit, no callback, returning the amount actually moved"
    requirement: "LEDG-01, LEDG-02, LEDG-03"
    verification:
      - kind: unit
        ref: "src/books.rs#construction_endows_every_agent_and_conserves_the_configured_stock"
        status: pass
      - kind: unit
        ref: "src/books.rs#a_transfer_moves_the_amount_reports_it_and_conserves_the_total"
        status: pass
      - kind: unit
        ref: "src/books.rs#every_refusal_leaves_the_books_exactly_as_it_found_them"
        status: pass
      - kind: other
        ref: "grep -c 'impl Default for Books' src/books.rs == 0; grep -vE '^[[:space:]]*//' src/books.rs | grep -cE 'pub fn [a-z_]+.*-> *&([a-z_]+ )?mut ' == 0; grep -cE 'RefCell|Rc<|Arc<|Mutex|dyn |impl Fn|FnMut|FnOnce' src/books.rs == 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "Construction sets the conservation baseline from the configured stock, refuses an endowment that does not sum to it, and leaves tick 0 with an empty journal"
    requirement: "LEDG-04"
    verification:
      - kind: unit
        ref: "src/books.rs#an_endowment_that_does_not_sum_to_the_stock_is_refused_at_construction"
        status: pass
      - kind: unit
        ref: "src/books.rs#tick_zero_begins_with_an_empty_journal"
        status: pass
      - kind: unit
        ref: "src/books.rs#ending_a_tick_clears_the_journal_and_the_count_but_not_the_residual"
        status: pass
    human_judgment: false
  - id: D3
    description: "The invariant phase is a value that runs as a real step and returns a `Result`, built once from the parameters with no per-tick branch on the gate and no debug-only assertion anywhere on the path"
    requirement: "LEDG-10"
    verification:
      - kind: unit
        ref: "src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks"
        status: pass
      - kind: integration
        ref: "tests/invariant_halt.rs#the_gate_removes_exactly_one_check_and_never_disables_the_phase"
        status: pass
      - kind: other
        ref: "cargo test --locked --release --all-targets (release is the primary profile for this phase); grep -cE 'debug_assert|debug_assertions' src/invariants.rs == 0 and src/books.rs == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "With the liveness gate on, a library tick loop halts at exactly the tick that recorded no transaction and does not begin the next one; with the gate off the identical loop completes"
    requirement: "LEDG-08, LEDG-10"
    verification:
      - kind: integration
        ref: "tests/invariant_halt.rs#with_the_gate_on_the_loop_halts_at_exactly_the_tick_that_traded_nothing"
        status: pass
      - kind: integration
        ref: "tests/invariant_halt.rs#with_the_gate_off_the_identical_loop_runs_every_tick"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#a_tick_that_traded_nothing_fails_only_because_the_gate_is_on"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#a_tick_that_moved_a_cent_passes_with_the_gate_on"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#the_transaction_count_resets_each_tick_so_liveness_is_a_per_tick_property"
        status: pass
    human_judgment: false
  - id: D5
    description: "A violation names the tick, the agent and the offending posting; localisation is a forward linear scan for the first non-conserving posting, never a search over halves of the journal"
    requirement: "LEDG-09"
    verification:
      - kind: integration
        ref: "tests/invariant_halt.rs#with_the_gate_on_the_loop_halts_at_exactly_the_tick_that_traded_nothing (message-contract assertion)"
        status: pass
      - kind: unit
        ref: "src/books.rs#a_transfer_moves_the_amount_reports_it_and_conserves_the_total (Posting Display pinned by full-string equality)"
        status: pass
      - kind: other
        ref: "grep -cE 'binary_search|\\bmid\\b|\\bhi\\b' src/invariants.rs == 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "Neither new module names a floating-point type or a debug-only assertion, comments included, and both are clippy-clean under the determinism lint wall"
    requirement: "LEDG-10"
    verification:
      - kind: integration
        ref: "tests/numeric_det.rs#confinement_of_the_float_domain"
        status: pass
      - kind: other
        ref: "bash tests/lints.sh; bash tests/toolchain.sh; cargo clippy --all-targets --all-features -- -D warnings; cargo fmt --check"
        status: pass
    human_judgment: false

# Metrics
duration: 12 min
completed: 2026-08-31
status: complete
---

# Phase 2 Plan 02: Books, Journal and Invariants — Tracer Summary

**One capability now runs end to end through the config gate, the ledger, the ordered check phase and a caller's loop: a library tick loop that moves a cent per tick halts at exactly the tick that moved none, and completes when the gate is off.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-08-31T09:08:00Z (approx — first task commit at 09:16:47Z)
- **Completed:** 2026-08-31T09:20:51Z
- **Tasks:** 3 of 3
- **Files modified:** 4 (3 created, 1 modified)

## Accomplishments

- **`src/books.rs` — the ledger.** `Books` holds every cent behind private fields, with exactly one constructor and exactly one cash-mutation point. `transfer` is compute-then-commit (resolve, refuse a negative amount, refuse self-dealing, subtract through the named non-panicking money API, refuse an overdraft, add — all before the first write), takes no closure, function pointer or trait object, and returns the amount actually moved. `Posting` carries two cash legs, two units legs and both running residuals; `PostError` and `BooksError` are separate `thiserror` types because a refused posting and a refused construction are different findings.
- **`src/invariants.rs` — the check phase.** `CheckSet::from_params` is the one site in the crate that reads `invariants.liveness_enabled`; it filters `ALL_CHECKS` once, so `run` iterates a `Vec` with no configuration lookup and no branch on the gate. `check_money` compares two independent sources — the balance vectors against the opening stock, and the journal's running residual against zero — and localises with a forward linear scan carrying the cancelling-residual counterexample in its doc comment.
- **`tests/invariant_halt.rs` — the end-to-end proof.** Through the public API alone, with no fault injection: with the gate on the loop returns `Violation::Liveness { tick: 4, counted: 0, required: 1 }` by whole-value equality *and* is proved never to have begun tick 5; with the gate off the identical loop runs all ten ticks; and the active-identifier sequence proves the gate removes exactly one check rather than disabling the phase.
- **Both new modules are float-free and profile-independent.** Neither names a floating-point type anywhere including prose (`tests/numeric_det.rs` reads raw lines), and neither names a debug-only assertion or the debug-build configuration predicate — LEDG-10's claim is about what the release binary contains, and the release profile is green.

## Task Commits

Each task was committed atomically:

1. **Task 1: The ledger half of the tracer slice — `Books`, the journal, and its registration** — `f48f0b5` (feat)
2. **Task 2: Close the slice — the check phase, and the end-to-end proof that a tick trading nothing halts the loop** — `b4b5dc8` (feat)
3. **Task 3: Pin the gate's behaviour and the check-set construction as unit tests** — `f137b61` (test)

**Plan metadata:** see the `docs(02-02)` commit that follows this file.

## Files Created/Modified

- `src/books.rs` (created, 832 lines) — `Books`, `Posting`, `PostingKind`, `PostError`, `BooksError`, `Display` and `Serialize` for `Posting`, plus seven unit tests.
- `src/invariants.rs` (created, 386 lines) — `Violation`, `CheckId`, `CheckFn`, `ALL_CHECKS`, `CheckSet`, `check_money`, `check_liveness`, `first_breaking_posting`, `MINIMUM_TRANSACTIONS_PER_TICK`, plus four unit tests under `invariants::liveness`.
- `tests/invariant_halt.rs` (created, 136 lines) — three integration tests driving the whole path from `config/baseline.toml`.
- `src/lib.rs` (modified, +2 lines) — `pub mod books;` and `pub mod invariants;`, list still flat and alphabetical.

## Decisions Made

Recorded in full in the `key-decisions` frontmatter above. The three with the longest reach:

1. **`Posting`'s wire shape renders addresses through `Display`, not through a serde derive on `src/ids.rs`.** The plan asked for `#[derive(Serialize)]` on `Posting`, but `Account`/`GoodId` do not implement `Serialize` and adding it would have committed Phase 3's `events.jsonl` to an externally-tagged enum encoding (`{"Household":12}`) as a side effect. `#[serde(serialize_with = ...)]` on the three address fields keeps the derive exactly as planned, leaves `src/ids.rs` untouched, and gives Phase 3 a greppable `"debit":"firm:3:0"` instead. Pinned by a unit test.
2. **`Books` carries its own `firm_generation` vector.** Balances key on the slot (stable across a Phase 10 respawn), postings key on the full identity (so the journal records which occupant acted), and resolution compares the two — an identity held across a respawn is `PostError::UnknownAccount`, tested.
3. **The recorder takes a `Posting` draft rather than nine parameters.** Nine would have tripped `clippy::too_many_arguments` under `-D warnings`, and the draft form also puts each leg's name at its construction site.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[derive(Serialize)]` on `Posting` does not compile against `src/ids.rs`**

- **Found during:** Task 1
- **Issue:** The plan specifies `Posting` derives serde's `Serialize`. `Account` and `GoodId` (from `src/ids.rs`, Phase 1) derive no serde traits, so the derive fails to compile. The two obvious fixes both had costs: adding `Serialize` to the five `src/ids.rs` newtypes modifies a file outside this plan's `files_modified` *and* silently decides Phase 3's event-stream encoding of an address; hand-writing `Serialize` for `Posting` abandons the derive the plan asked for.
- **Fix:** Kept `#[derive(Serialize)]` on `Posting` exactly as planned and added `#[serde(serialize_with = ...)]` to `debit`, `credit` and `good`. Addresses serialise through their `Display` form (`"household:12"`, `"firm:3:0"`) and a good as its bare index. `src/ids.rs` is untouched and the ledger owns the wire shape of a posting, which is where the plan's own reversibility note puts the decision.
- **Files modified:** `src/books.rs`
- **Verification:** `src/books.rs#a_posting_serialises_with_rendered_addresses_and_integer_amounts` asserts the rendered keys; `cargo build --locked --release` and `cargo clippy --all-targets --all-features -- -D warnings` both clean.
- **Committed in:** `f48f0b5`

**2. [Rule 3 - Blocking] `Books::new` needed a bound on the firm count to avoid a narrowing cast**

- **Found during:** Task 1
- **Issue:** `params.sim.firms` is a `u32` and `FirmSlot` is a `u16`. Writing `index as u16` in the endowment loop would reintroduce exactly the silent aliasing that `FirmArena::with_occupants` was hardened against in Phase 1 (CR-03), and `BooksError` is specified to carry one variant, so the condition has no honest error to be reported as.
- **Fix:** Narrowed once at the top of `Books::new` with `u16::try_from(params.sim.firms).expect(...)`, using the same message shape and the same reasoning as the precedented panic in `src/ids.rs`. `config::load` already refuses a firm count past `MAX_FIRMS`, so the bound is restated rather than newly imposed, and no `as` cast that could truncate appears below it.
- **Files modified:** `src/books.rs`
- **Verification:** `cargo clippy --all-targets --all-features -- -D warnings`; `src/books.rs#construction_endows_every_agent_and_conserves_the_configured_stock` exercises the path against the shipped 20-firm configuration.
- **Committed in:** `f48f0b5`

**3. [Rule 1 - Bug] The plan's `cargo test --locked --lib books invariants` verify command is not valid `cargo` syntax**

- **Found during:** Task 2
- **Issue:** `cargo test` accepts one positional `TESTNAME`; a second is rejected with `error: unexpected argument 'invariants' found` and the command exits non-zero without running anything. Taken at face value, this `<automated>` check would have read as a permanent failure of task 2.
- **Fix:** Ran the equivalent that expresses the same intent, `cargo test --locked --lib -- books invariants`, which passes both patterns to libtest as filters. Result: 7 passed, 0 failed (task 2 point) and 11 passed after task 3.
- **Files modified:** none — this is a plan defect, not a source defect.
- **Verification:** Both filters observed to select their modules in the output; the superset `cargo test --locked --all-targets` and its release counterpart are also green.
- **Committed in:** n/a (no source change). Recorded in `.planning/WINDOWS.md` so plans 02-03 through 02-07 do not copy the broken form.

---

**Total deviations:** 3 auto-fixed (2 × Rule 3 blocking, 1 × Rule 1 bug).
**Impact on plan:** None on scope or architecture. Deviation 1 preserves the plan's stated derive while making a wire-shape choice the plan itself flags as costly from Phase 3 onward — worth recording for 02-03, which appends `Exchange`/`Produce`/`Consume` to `PostingKind`. Deviations 2 and 3 are mechanical.

## Issues Encountered

None. The slice compiled, the end-to-end check was green on its first run, and both build profiles agree.

## Known Stubs

One, deliberate, plan-owned, and named in the plan's own objective:

| Stub | File | Line | Reason |
|------|------|------|--------|
| `Posting::units_out` / `units_in` / `goods_residual_units` are zero on every posting this phase produces | `src/books.rs` | ~118-135, ~561 | The goods identity has no terms until goods postings exist. Plan **02-03** appends `PostingKind::Exchange`, `Produce` and `Consume` and gives these fields values; plan **02-04** adds the goods-conservation check that reads them. The fields are carried now rather than added later because `Posting` is the wire shape Phase 3 writes into its event stream, and widening it after snapshots exist is a trajectory-visible change. The recorder already advances the goods residual from them, so no code changes when the values become non-zero — only the inputs do. |

The middle element of each `ALL_CHECKS` triple (the check's stable name) is likewise unread in this plan; plan **02-05** reports seeded corruptions against it. Both are documented at their definitions.

This stub does not prevent this plan's goal: the tracer's capability is money conservation and liveness, and neither reads a goods term.

## Self-Check: PASSED

- `src/books.rs`, `src/invariants.rs`, `tests/invariant_halt.rs` — all present on disk.
- Commits `f48f0b5`, `b4b5dc8`, `f137b61` — all present in `git log --oneline --all`.
- `git diff --diff-filter=D --name-only f48f0b5~1 HEAD` — empty; no file was deleted.
- `git status --short` — clean after each task commit; no untracked files left behind.
- All task-level `<acceptance_criteria>` re-run and passing:
  - `cargo test --locked --lib -- books invariants` → 11 passed, 0 failed
  - `cargo test --locked --release --test invariant_halt` → 3 passed, 0 failed
  - `cargo test --locked --release --lib invariants::liveness` → 4 passed, 0 failed
  - `cargo build --locked --release` → 0
  - `grep -cE '\bf16\b|\bf32\b|\bf64\b|\bf128\b'` → `0` for both new modules
  - `grep -c 'impl Default for Books' src/books.rs` → `0`
  - `grep -vE '^[[:space:]]*//' src/books.rs | grep -cE 'pub fn [a-z_]+.*-> *&([a-z_]+ )?mut '` → `0`
  - `grep -cE 'binary_search|\bmid\b|\bhi\b' src/invariants.rs` → `0`
  - `grep -c 'to_string().contains' src/invariants.rs` → `0`
  - `grep -c 'pub mod books;' src/lib.rs` → `1`; `grep -c 'pub mod invariants;' src/lib.rs` → `1`
  - `grep -cE 'LEDG-0[1-9]|LEDG-10'` → `9` (books) and `4` (invariants)
- Plan-level `<verification>` re-run and passing:
  - `cargo test --locked --all-targets` → all suites ok (100 lib + integration suites)
  - `cargo test --locked --release --all-targets` → all suites ok (98 lib + integration suites; the two-test delta is the pre-existing debug-gated RNG re-entry pair)
  - `cargo clippy --all-targets --all-features -- -D warnings` → clean
  - `cargo fmt --check` → clean
  - `bash tests/lints.sh` → OK (checks 1-4, including "the clean tree passes the lint gate")
  - `bash tests/toolchain.sh` → OK
- `.planning/STATE.md` and `.planning/ROADMAP.md` untouched by this plan.

## Next

Plan **02-03** (goods postings: `Exchange`, `Produce`, `Consume`, per-account stock) may begin — the tracer's end-to-end check is green, which is the gate the plan's objective set on every expansion plan in this phase.
