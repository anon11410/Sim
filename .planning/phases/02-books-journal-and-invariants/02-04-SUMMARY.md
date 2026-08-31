---
phase: 02-books-journal-and-invariants
plan: 04
subsystem: ledger
tags: [books, headcount, invariants, non-negativity, zero-sum, check-order, posting]

# Dependency graph
requires:
  - phase: 02-books-journal-and-invariants
    plan: 03
    provides: "`Posting`'s two real units legs, `Books::produce`/`consume`/`exchange`, the three-entry `ALL_CHECKS`, `Violation`'s boxed optional posting, `BooksError::InitialInventoryOutOfRange`"
  - phase: 02-books-journal-and-invariants
    plan: 02
    provides: "`Books`, the private recorder, `CheckSet`/`CheckId`/`CheckFn`, the end-to-end halt proof"
  - phase: 01-primitives-and-the-determinism-spine
    provides: "`Account`/`FirmId`/`FirmSlot`/`HouseholdId` and their derived total order, `Money`, the release profile's overflow checks"
provides:
  - "`sim::books::Books::headcount_of` / `set_headcount` / `total_headcount` — the books' third quantity, keyed by firm slot, unsigned"
  - "`sim::books::Books::accounts` — the fixed-order account walk (households ascending, then firm slots ascending) the non-negativity check rests on"
  - "`sim::books::PostError::EmptyExchange` — an exchange with an empty cash or units leg, refused at the boundary"
  - "`sim::invariants::NegativeField` (Cash, Stock) with `Display`"
  - "`sim::invariants::ZeroSumDetail` — eight named shapes, each carrying the offending numbers or identities, with `Display`"
  - "`sim::invariants::Violation::Negative` (posting optional) and `Violation::ZeroSum` (posting mandatory)"
  - "`sim::invariants::CheckId::NonNegative` / `ZeroSum` and `CheckId::ALL` — the single source of truth for the sequence"
  - "`ALL_CHECKS` completed to five entries in the documented order, with `check_non_negative` and `check_zero_sum`"
affects: [02-05, 02-06, 02-07, phase-03-tick-pipeline, phase-05-production, phase-06-labour, phase-07-goods-market, phase-10-bankruptcy]

actuals:
  tokens: 16765
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "A quantity whose non-negativity is a fact of its type is documented as one, never expressed as a runtime loop: an unsigned payroll cannot go negative, so a loop over the payrolls would be a check that can never fire and would be indistinguishable in a test report from one that works"
    - "The walk order over accounts is an accessor on the books (`Books::accounts`), not an ad-hoc loop in the check: it puts the ordering contract next to the vectors it orders, and makes the order testable on its own"
    - "A malformed shape is refused at the operation boundary AND reported by the check: refusing means no run records one, reporting means one that somehow appears is named rather than passing. Applied to self-dealing (02-03) and now to an empty exchange"
    - "The completeness of a table is a compile error, not a promise: an exhaustive match over every identifier stops compiling when a variant is added, and the assertions around it then force the variant into both the constant and the table"
    - "A name in a table is asserted against a spelling DERIVED from its identifier, never against a second hand-written string — the derivation is what makes drift impossible rather than merely noticed"
    - "A check function whose negative direction has no public path is split into a per-item rule (`well_formed`) that a synthesised item can drive, plus a walk that a real journal drives"

key-files:
  created: []
  modified:
    - src/books.rs
    - src/invariants.rs
    - tests/invariant_halt.rs

key-decisions:
  - "The books own the headcount (research Pitfall 7 option b, Open Question 2, assumption A3 — the fork the CONTEXT did not settle). Three reasons, all recorded in the module docs: the books own every quantity an invariant reads, so a third column with a different owner makes that sentence false; it removes a cross-phase promise rather than recording one, because Phase 6 then has nowhere else to put it; and an unsigned count makes LEDG-06's third column a fact of the type rather than a loop over an empty structure that passes vacuously"
  - "`headcount_of` and `set_headcount` take a `FirmSlot`, not an `Account`. A household has no payroll, so an address-shaped accessor would carry an arm answering `None` for every household that has ever existed — a signature inviting a question with no answer. The slot makes 'only a firm has employees' a fact of the type"
  - "`total_headcount` widens to 64 bits. A run may hold `u16::MAX` slots each carrying a `u32` payroll, so the sum can exceed the element width and would abort under this project's overflow checks on an economy that is merely large"
  - "The docs state flatly that a headcount is NOT conserved value — no counterparty, no opening stock, no identity — which is why `set_headcount` does not contradict LEDG-01, and that it is the whole of Phase 2's headcount vocabulary with Phase 6 (LABR-01..08) building on top of it rather than beside it"
  - "The account walk lives on `Books` as `accounts()`, returning an iterator rather than a `Vec`: it runs once per tick for the whole run, and an allocation per tick for a fixed sequence would be a cost paid 3,650 times for nothing"
  - "`Violation::ZeroSum`'s posting is NOT optional while `Violation::Negative`'s is. That asymmetry is structural rather than stylistic: zero-sum is evaluated one posting at a time so the offender is always known, while a balance driven negative outside the posting path genuinely has none to blame"
  - "`check_zero_sum` is split into a journal walk and a per-posting rule (`well_formed`). No public operation can produce a malformed posting — that is the point of the rest of the phase — so without the split the rule could not be shown to discriminate at all until plan 02-05"
  - "Non-negativity's negative direction needs no fault injection: a configuration whose per-agent endowments still sum to the money stock, but endows one side of the population a negative amount, opens books that conserve exactly and hold negative balances. That is the research's 'driven-negative balance with the total intact' case, reached entirely through the public API, and it is what proves this check and `check_money` are independent"
  - "Zero-sum sits FOURTH, after non-negativity. A malformed posting will usually already have shown up as a broken conservation identity, and the identity is the more useful of the two reports; non-negativity is a finding about one account and belongs above it"

patterns-established:
  - "Each new check is taken from `ALL_CHECKS` by position in its own test module, with the identifier and the name asserted at the point of retrieval — so every test runs the function the tick loop runs and the position is asserted rather than assumed (extended from 02-03's `goods_check`)"
  - "Exactly one message-contract test per violation variant reads the rendered `Display`; every other assertion is whole-value equality (research Pitfall 11)"

requirements-completed: []

coverage:
  - id: D1
    description: "The books own the headcount as their third quantity, keyed by firm slot, with read, write and aggregate accessors — and its non-negativity is a documented fact of the unsigned type rather than an unreachable runtime loop (LEDG-06)"
    requirement: "LEDG-06"
    verification:
      - kind: unit
        ref: "src/books.rs#headcount::every_slot_opens_with_an_empty_payroll"
        status: pass
      - kind: unit
        ref: "src/books.rs#headcount::setting_a_count_then_reading_it_back_round_trips"
        status: pass
      - kind: unit
        ref: "src/books.rs#headcount::the_total_is_the_sum_of_the_individual_counts"
        status: pass
      - kind: unit
        ref: "src/books.rs#headcount::a_slot_outside_the_arena_reads_nothing_and_writes_nothing"
        status: pass
      - kind: unit
        ref: "src/books.rs#headcount::the_headcount_is_independent_of_the_two_conserved_quantities"
        status: pass
      - kind: other
        ref: "grep -c 'pub fn headcount_of' src/books.rs == 1; 'pub fn set_headcount' == 1; 'pub fn total_headcount' == 1; grep -cE 'LEDG-06' src/books.rs == 6"
        status: pass
    human_judgment: false
  - id: D2
    description: "No account holds a negative cash or stock balance at the end of a tick, and the check names the account, the column and the value — reported in a fixed walk order and independent of money conservation (LEDG-06)"
    requirement: "LEDG-06"
    verification:
      - kind: unit
        ref: "src/invariants.rs#non_negative::a_negative_balance_with_the_total_intact_is_reported_and_conservation_is_not"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#non_negative::households_are_walked_before_firm_slots_and_slots_in_ascending_order"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#non_negative::the_violation_names_the_first_posting_touching_the_account_or_says_there_is_none"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#non_negative::a_healthy_economy_holds_no_negative_quantity"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#non_negative::the_message_names_the_account_the_column_and_the_value"
        status: pass
      - kind: unit
        ref: "src/books.rs#tests::the_account_walk_is_households_ascending_then_firm_slots_ascending"
        status: pass
      - kind: other
        ref: "grep -cE 'binary_search|\\bmid\\b|\\bhi\\b' src/invariants.rs == 0 (localisation is a linear scan)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every posting is well formed for its kind, checked on one posting with no aggregate — and the check is meaningful only because a Posting carries two cash amounts and two unit amounts (LEDG-07)"
    requirement: "LEDG-07"
    verification:
      - kind: unit
        ref: "src/invariants.rs#zero_sum::each_malformed_shape_is_named_exactly"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#zero_sum::an_over_credited_exchange_is_expressible_as_data_and_is_caught"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#zero_sum::every_posting_a_real_run_records_is_well_formed"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#zero_sum::the_endowment_shape_the_constructor_records_is_well_formed"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#zero_sum::the_public_api_refuses_the_shapes_this_check_looks_for"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#zero_sum::the_violation_names_the_posting_and_what_disagreed"
        status: pass
      - kind: unit
        ref: "src/books.rs#goods::every_refused_exchange_moves_neither_cash_nor_units (three EmptyExchange cases)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The check table is complete and in its documented order, asserted from the table itself, and a check identifier cannot be added without an entry"
    requirement: "LEDG-06, LEDG-07"
    verification:
      - kind: unit
        ref: "src/invariants.rs#order::the_table_runs_the_documented_sequence"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#order::an_identifier_cannot_exist_without_a_table_entry"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#order::the_names_are_distinct_and_spell_their_identifiers"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#liveness::the_gate_decides_the_exact_sequence_of_active_checks"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#non_negative::the_check_set_reports_the_deficit_before_it_reports_liveness"
        status: pass
      - kind: integration
        ref: "tests/invariant_halt.rs#the_gate_removes_exactly_one_check_and_never_disables_the_phase"
        status: pass
    human_judgment: false
  - id: D5
    description: "Both modules stay float-free and profile-independent, and the whole suite is green in both profiles under the determinism lint wall"
    requirement: "LEDG-10"
    verification:
      - kind: integration
        ref: "tests/numeric_det.rs#confinement_of_the_float_domain"
        status: pass
      - kind: other
        ref: "cargo test --locked --all-targets; cargo test --locked --release --all-targets; bash tests/lints.sh; bash tests/toolchain.sh; cargo clippy --all-targets --all-features -- -D warnings; cargo fmt --check"
        status: pass
    human_judgment: false

# Metrics
duration: 15 min
completed: 2026-08-31
status: complete
---

# Phase 2 Plan 04: Non-Negativity, Zero-Sum and the Check-Order Contract Summary

**The invariant phase is complete and ordered: the books now own headcount as their third quantity with its non-negativity closed by an unsigned type rather than by an unreachable loop, no account can hold a negative cash or stock balance without the check naming the account, the column and the value in a fixed walk order, every posting is checked for the shape its kind requires one posting at a time, and the five-entry table's sequence is asserted from the table itself against a single source of truth that a new identifier cannot bypass without a compile error.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-08-31T09:40:40Z
- **Completed:** 2026-08-31T09:55:10Z
- **Tasks:** 2 of 2
- **Files modified:** 3

## Accomplishments

- **The books' third quantity, and the fork the research flagged for a planner, decided.** `firm_headcount: Vec<u32>` is keyed by firm slot exactly as that slot's cash and stock are, opened at zero by the constructor because no employment relation exists before Phase 6. `headcount_of`, `set_headcount` and `total_headcount` are the whole of Phase 2's headcount vocabulary. The module docs record the three reasons the books own it — one owner for every quantity an invariant reads, a cross-phase promise removed rather than recorded, and a type-level guarantee that cannot rot into a vacuous loop — and state flatly that a headcount is *not* conserved value, which is why a setter here does not contradict LEDG-01.
- **`check_non_negative` walks every account in a contracted order.** Households by ascending index, then firm slots by ascending slot, cash before stock, resting on the derived total order `src/ids.rs` already carries. Two accounts can be negative at once, so an arbitrary answer to "which one" would make the check's own negative test flaky in a way indistinguishable from a real failure. `Violation::Negative` names the account, the column (`NegativeField`) and the value.
- **Its negative direction needs no fault injection at all.** Endow every household 100 cents below zero and give the firms the difference: the endowment still sums to the configured stock, so `check_money` and `check_goods` both pass and only non-negativity fires. That is the research's "driven-negative balance with the total intact" case, reached entirely through the public API, and it is the test that proves the two checks are independent rather than two views of one number. The mirror configuration — every firm slot negative, every household clean — proves the walk reaches slot 0 only after finding all 200 households sound.
- **Localisation in both directions.** With no posting naming the offending account the violation says so in those terms; with one, it carries that posting. A synthetic posting in a halt message is a lie a future reader will chase, and a balance driven negative outside the posting path is exactly the reachable case that produces none.
- **`check_zero_sum` validates the shape of every posting kind, one posting at a time.** A transfer moves equal cash between two distinct accounts and no units; an exchange moves equal cash one way and equal units the other with neither leg empty; a production or consumption names one account on both legs, carries no cash and moves units in exactly one direction; an endowment carries no debit leg. `ZeroSumDetail` names exactly what disagreed, carrying the offending numbers or identities and never a formatted string.
- **The check is possible only because a `Posting` carries two cash amounts and two unit amounts,** and one test says so directly: an exchange debiting 500 and crediting 501 is *expressible as data* and is caught. With one amount of each it could not be written down, and the whole check would be a structural tautology.
- **The table is complete, ordered, and mechanically so.** `CheckId::ALL` is the single source of truth; `the_table_runs_the_documented_sequence` reads the identifiers out of `ALL_CHECKS` and compares against it, never against a second hand-written list. `an_identifier_cannot_exist_without_a_table_entry` pattern-matches every variant exhaustively, so a new `CheckId` stops the crate compiling until it has a position. `the_names_are_distinct_and_spell_their_identifiers` *derives* each snake-case name from its identifier rather than restating it — the derivation is what makes drift impossible rather than merely noticed.
- **The order is proved to bite, not just to exist.** `the_check_set_reports_the_deficit_before_it_reports_liveness` builds books that are both negative and silent, and asserts the whole `CheckSet` returns the deficit: position three beats position five, and the diagnostically useful finding wins.

## Task Commits

1. **Task 1: the books own the headcount, and its non-negativity is a type** — `1c2d0d9` (feat)
2. **Task 2: the last two invariants, and the order contract over the table** — `03cf3a2` (feat)

**Plan metadata:** the `docs(02-04)` commit that follows this file.

## Files Created/Modified

- `src/books.rs` (modified, +396) — the module docs' headcount decision, `firm_headcount`, three accessors, `Books::accounts`, `PostError::EmptyExchange` and its refusal in `exchange`, five tests under `books::headcount`, one walk-order test and three refusal cases under `books::goods`.
- `src/invariants.rs` (modified, +1,217) — `NegativeField`, `ZeroSumDetail` (eight variants) and both `Display` impls, `Violation::Negative` and `Violation::ZeroSum`, `CheckId::NonNegative`/`ZeroSum`/`ALL`, the five-entry `ALL_CHECKS`, `check_non_negative`, `check_zero_sum`, `well_formed`, `two_party`, `one_party`, `no_cash`, `negative`, `first_posting_naming`, and 15 tests across `invariants::order`, `invariants::non_negative` and `invariants::zero_sum`.
- `tests/invariant_halt.rs` (modified, +9 −2) — the active-check sequence assertions, now five entries with the gate on and four with it off.

## Decisions Made

Recorded in full in the `key-decisions` frontmatter. The three with the longest reach:

1. **The books own the headcount.** This was research assumption A3 and Open Question 2 — a genuine design fork the phase context did not settle, explicitly flagged for a planner rather than for whoever wrote the check first. Option (b) is taken, and Phase 6 now builds `LABR-01..08` on top of `set_headcount` rather than introducing a payroll of its own. Reversible today at the cost of moving one vector; costly once Phase 6's hiring rules are written against it.
2. **`Violation::ZeroSum` carries a mandatory posting; `Violation::Negative` carries an optional one.** The asymmetry is structural. Zero-sum is per-posting, so the offender is always known. Non-negativity reads a balance, and a balance can be driven negative by a write that no posting describes.
3. **A malformed shape is refused at the boundary *and* reported by the check.** `exchange` now refuses an empty leg exactly as it already refused self-dealing, so no run records one; `check_zero_sum` still reports one if a corruption produces it. The two are different guarantees and both are wanted.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `src/books.rs` was modified in Task 2, though the task's `<files>` lists only `src/invariants.rs`**

- **Found during:** Task 2
- **Issue:** `check_non_negative` must "walk every account in a fixed, documented order". No accessor enumerated accounts, and none could be written outside `books.rs`: the household count and — decisively — the per-slot firm *generations* needed to construct a live `Account::Firm` are private fields. Without them the check cannot address a single firm account, so the task's mandate and its file list are not jointly satisfiable as written.
- **Fix:** `Books::accounts()`, returning an iterator (not a `Vec` — it is walked once per tick for 3,650 ticks) in the contracted order, with the ordering rationale in its doc comment where the vectors it orders live. Covered by `books::tests::the_account_walk_is_households_ascending_then_firm_slots_ascending`, which asserts the sequence, that it agrees with the derived total order, and that every address it yields resolves.
- **Files modified:** `src/books.rs`
- **Committed in:** `03cf3a2`
- **Note:** the plan's own `files_modified` frontmatter lists both files, so this is a task-level scope note rather than a phase-level one.

**2. [Rule 2 — Missing critical] `exchange` accepted an empty leg, which is a liveness hole and a posting the new check would halt on**

- **Found during:** Task 2
- **Issue:** The plan's zero-sum rule for an exchange requires "both legs non-zero", but `Books::exchange` refused a negative amount and a negative count and said nothing about zero. Two consequences, and the second is the serious one. First, `exchange(buyer, seller, good, 0, Money::ZERO)` increments `transactions_this_tick` while moving nothing — the degenerate "a transaction happened" pass LEDG-08 exists to close. Second, with the check written as the plan specifies, a legitimate public call would have produced a posting that halts the run: a self-inflicted false halt, which is strictly worse than a weaker check.
- **Fix:** `PostError::EmptyExchange { units, amount_cents }`, refused in `exchange`'s compute step after the two sign checks and before self-dealing, on exactly the precedent 02-03 set for `SelfDealing` — refuse at the boundary so the journal never records one, and let the check report one that somehow appears. Three cases added to `every_refused_exchange_moves_neither_cash_nor_units` (both legs empty, cash empty, units empty), each asserting the value and then that nothing was written.
- **Files modified:** `src/books.rs`
- **Committed in:** `03cf3a2`

**3. [Rule 3 — Blocking] `tests/invariant_halt.rs` had to change again, and is outside `files_modified`**

- **Found during:** Task 2
- **Issue:** `the_gate_removes_exactly_one_check_and_never_disables_the_phase` asserts the exact active sequence. Inserting two checks at positions three and four — which the plan mandates — makes it fail, and the plan's own `<verify>` block requires `cargo test --locked --release --test invariant_halt` to pass. Anticipated by 02-03 (its Deviation 1, and ledger entry 7).
- **Fix:** Both assertions updated — the integration one and its unit-level twin `src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks` — to five entries with the gate on and four with it off. **No behavioural claim was weakened:** the loop still halts at tick 4 with `Violation::Liveness`, still runs all ten ticks with the gate off, and the gate still removes liveness and nothing else. Ledger entry 7 is thereby discharged.
- **Files modified:** `tests/invariant_halt.rs`, `src/invariants.rs`
- **Committed in:** `03cf3a2`

### Deliberate refinements

- **`ZeroSumDetail` ships eight variants, not the six in `<artifacts_produced>`.** Two shapes the action text requires cannot be expressed by the six listed. `SplitParties { debit, credit }` is the mirror of `SelfDealing` — a one-party kind (produce, consume, endow) naming two different accounts, which the action text's "names one account on both legs" clause demands and which only a corruption could produce. `EmptyExchange { cents, units }` is the "with both legs non-zero" clause; none of the six could name it, and dropping the clause instead would have left the liveness hole in Deviation 2 uncovered by the check. Recorded in `.planning/WINDOWS.md` so 02-05's message tests expect eight.
- **`check_zero_sum` is split into a journal walk and a per-posting rule.** `well_formed(&Posting) -> Result<(), ZeroSumDetail>` is what the negative tests drive, because no public operation can produce a malformed posting and a rule reachable only through books nobody can corrupt would go untested until 02-05. The walk is what the real journal drives.
- **`check_non_negative` uses let-chains** (`if let Some(cash) = … && cash.cents() < 0`) rather than nested `if let`, which is the form edition 2024 makes available and which clippy prefers.
- **Anchored grep forms used for the criteria that needed them,** per ledger entry 5. None of this plan's criteria used an unanchored `pub fn <name>` count that a longer sibling would also match, so no substitution was required — but `pub fn total_headcount`, `pub fn headcount_of` and `pub fn set_headcount` were each verified to print exactly `1`.

## Authentication Gates

None.

## Issues Encountered

None outstanding. Worth recording that the wave-3 warning was heeded and cost nothing: `Violation` gained two variants, both smaller than `GoodsConservation`, so the enum did not grow and `clippy::result_large_err` stayed quiet. `cargo clippy --all-targets --all-features -- -D warnings` was run explicitly rather than inferred from a green `cargo test`.

## Known Stubs

**None from this plan.**

Two forward obligations, both plan-owned and neither a stub in this plan's output:

| Obligation | Where | Owner |
|---|---|---|
| `NegativeField::Stock` has no negative test: a negative stock balance has no public path, because `Books::new` refuses a negative initial inventory (02-03) and every unit operation refuses a negative count. The arm is exercised only through `Violation::Negative`'s message-contract test | `src/invariants.rs` `check_non_negative` | Plan 02-05 (seeded corruptions) |
| `Violation::ZeroSum` is constructible but not reachable through the public API — `exchange` and `transfer` refuse every shape `well_formed` looks for, which is the point of the phase. The rule is exercised here through `well_formed` over synthesised postings | `src/invariants.rs` `check_zero_sum` | Plan 02-05 (seeded corruptions) |

## Threat Flags

None. No new network endpoint, auth path, file access or trust boundary. The plan's register is addressed as written: T-02-18 — the non-negativity check walks cash and stock, both signed and both genuinely reachable, and headcount's exclusion is a documented type-level fact rather than a loop that never fires; T-02-19 — every posting kind is checked, including the shapes that must carry no cash and the endowment that must carry no debit leg, and an imbalance is expressible as data because a posting carries two cash amounts and two unit amounts; T-02-20 — the walk order is fixed, documented and tested in both directions (households clean / firms clean); T-02-21 — `CheckId::ALL` is the single source of truth, the order test reads the table, and the exhaustive match makes a dropped check a compile error; T-02-22 — both detail enums carry only integers and identities, no owned string, path, host name, wall-clock reading or process id; T-02-SC — no package was installed, `Cargo.lock` is unchanged and both `--locked` builds are green.

## Self-Check: PASSED

- `src/books.rs`, `src/invariants.rs`, `tests/invariant_halt.rs` — all present on disk.
- Commits `1c2d0d9`, `03cf3a2` — both present in `git log --oneline --all`.
- `git diff --diff-filter=D --name-only HEAD~1 HEAD` — empty; no file was deleted by either commit.
- `git status --short` — clean after each task commit; no untracked files left behind.
- All task-level `<acceptance_criteria>` re-run and passing:
  - `cargo test --locked --release --lib -- books` → 24 passed
  - `cargo test --locked --release --lib -- books::headcount` → 5 passed (not `0 filtered out`)
  - `cargo test --locked --release --lib -- invariants` → 25 passed
  - `cargo test --locked --release --lib -- invariants::order` → 3 passed (≥ 3 required)
  - `cargo test --locked --release --lib -- invariants::non_negative` → 6 passed
  - `cargo test --locked --release --lib -- invariants::zero_sum` → 6 passed
  - `cargo test --locked --release --test invariant_halt` → 3 passed
  - `grep -c 'pub fn headcount_of' src/books.rs` → `1`; `pub fn set_headcount` → `1`; `pub fn total_headcount` → `1`
  - `grep -cE 'LEDG-06' src/books.rs` → `6` (≥ 1 required)
  - `grep -c 'CheckId::ALL' src/invariants.rs` → `8` (≥ 2 required)
  - `grep -cE 'binary_search|\bmid\b|\bhi\b' src/invariants.rs` → `0`
  - `grep -c 'to_string().contains' src/invariants.rs` → `0`
  - `grep -cE '\bf16\b|\bf32\b|\bf64\b|\bf128\b'` → `0` for both modules
  - `grep -cE 'debug_assert|debug_assertions'` → `0` for both modules
  - `grep -cE 'RefCell|Rc<|Arc<|Mutex|dyn |impl Fn|FnMut|FnOnce' src/books.rs` → `0`
  - `grep -vE '^[[:space:]]*//' src/books.rs | grep -cE 'pub fn [a-z_]+.*-> *&([a-z_]+ )?mut '` → `0`
- Plan-level `<verification>` re-run and passing:
  - `cargo test --locked --all-targets` → 10 suites ok, 0 failed (196 tests)
  - `cargo test --locked --release --all-targets` → 10 suites ok, 0 failed (194 tests; the two-test delta is the pre-existing debug-gated RNG re-entry pair)
  - `cargo test --locked --test numeric_det` → 5 passed
  - `cargo clippy --all-targets --all-features -- -D warnings` → clean
  - `cargo fmt --check` → clean
  - `bash tests/lints.sh` → OK (checks 1–4; all 60 resolvable method bans fire)
  - `bash tests/toolchain.sh` → OK
- `.planning/STATE.md` and `.planning/ROADMAP.md` untouched by this plan.
- `.planning/REQUIREMENTS.md` untouched: `requirements ready-ids` reports `0/2 ready` — plan 02-05 also declares LEDG-06 and LEDG-07, so neither may read `Complete` yet. Respected per the shared-artifact rule.

## Next

Plan **02-05** (seeded corruptions and the message contract) may begin. It inherits: the five-entry `ALL_CHECKS` in its final order; `Violation` with five variants, of which `Negative` boxes an *optional* posting and `ZeroSum` boxes a *mandatory* one; `ZeroSumDetail` with **eight** variants rather than the six the plan listed; `well_formed` as the per-posting rule its corruptions can drive directly; and the two active-sequence assertions (in `tests/invariant_halt.rs` and `src/invariants.rs`) that any further check would have to update.
