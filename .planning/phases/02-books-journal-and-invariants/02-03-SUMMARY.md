---
phase: 02-books-journal-and-invariants
plan: 03
subsystem: ledger
tags: [books, goods, inventory, posting, invariants, conservation, exchange]

# Dependency graph
requires:
  - phase: 02-books-journal-and-invariants
    plan: 02
    provides: "`Books` with its private cash vectors, the compute-then-commit shape, the private recorder and its incremental residuals, `Posting`'s two units legs, `CheckSet`/`ALL_CHECKS`/`Violation`, and the end-to-end halt proof"
  - phase: 01-primitives-and-the-determinism-spine
    provides: "`GoodId`, `Account`/`FirmId`/`FirmSlot`, `Money` and its non-panicking half, `firm.initial_inventory_units`, the release profile's overflow checks"
provides:
  - "`sim::books::Books::produce` / `consume` / `exchange` — the only three unit-mutation points, each compute-then-commit and each returning the quantity actually moved"
  - "`sim::books::Books::goods` / `stock_of` / `total_stock` / `produced` / `consumed` / `goods_residual_units` — the goods accessors, already account-and-good-shaped for Phase 5"
  - "`sim::books::PostingKind::Exchange` / `Produce` / `Consume` — appended, so the Phase 3 wire shape is additive"
  - "`sim::books::PostError::ShortStock` / `UnknownGood` / `NegativeUnits`"
  - "`sim::books::BooksError::InitialInventoryOutOfRange` / `InventoryDoesNotBalance`"
  - "`sim::invariants::CheckId::GoodsConservation` and `Violation::GoodsConservation` — the goods identity as a real check, second in `ALL_CHECKS`"
  - "`Posting::units_out` / `units_in` / `goods_residual_units` now carry real values — the 02-02 stub is closed"
affects: [02-04, 02-05, 02-06, 02-07, phase-03-tick-pipeline, phase-05-production, phase-07-goods-market, phase-10-bankruptcy]

actuals:
  tokens: 15100
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "One posting for a two-sided operation: `exchange` moves cash one way and units the other in a single journal line, so the swap cannot be half-applied and LEDG-07 stays a per-posting property"
    - "Two independently maintained sources per conservation check, extended to goods: the `produced`/`consumed` totals are advanced from the operations' ARGUMENTS, the running residual from the POSTINGS' LEGS, and the check compares them"
    - "The recorder derives each posting's net effect on the identity from that posting's own kind and legs, never from the totals the operations maintain — which is what keeps the two sources independent"
    - "An endowment that raises a conserved quantity must also advance its baseline in the same step: initial inventory raises Σstock, so it is counted into `produced` in the same loop iteration"
    - "One resolution serves both quantities: `AccountSlot` (renamed from `CashSlot`) indexes cash and stock identically, so a firm's money and its inventory cannot drift onto different keys across a Phase 10 respawn"
    - "A refusal boundary the identity cannot police is closed at construction: a negative inventory endowment balances perfectly (negative `produced` against negative stock), so it is refused rather than checked"

key-files:
  created: []
  modified:
    - src/books.rs
    - src/invariants.rs
    - tests/invariant_halt.rs

key-decisions:
  - "The units leg points OPPOSITE to the cash leg, and the field docs now say so: for an `Exchange` the buyer is the debit account because it pays, so `units_out` leaves the CREDIT account and `units_in` arrives at the DEBIT one. That is exactly LEDG-07's 'same pair of accounts in opposite directions', and 02-04's zero-sum check reads these two fields"
  - "The recorder computes `produced_added − consumed_added + units_out − units_in` from the posting's KIND and LEGS. It is zero for every well-formed posting of every kind, and non-zero exactly when a posting's legs contradict the totals its kind claims to move — a half-applied exchange, or a produce that credits units it does not count"
  - "`Endow` with `units_in > 0` counts as production in the recorder, which is what makes the tick-0 identity hold; the constructor advances the `produced` field in the same loop iteration that writes the stock"
  - "The three per-good totals (`total_stock`, `produced`, `consumed`) return zero for a good the books do not carry, and that is a fact rather than a fallback — every mutation refuses an unknown good before touching a vector, so no unit of one can exist. `stock_of` returns `None` instead, because it names a specific account"
  - "`CheckId` is declared in RUN order (Money, Goods, Liveness) rather than appended, so its derived `Ord` agrees with `ALL_CHECKS`. Unlike `PostingKind` it is not a wire shape and carries no append-only obligation, and an `Ord` disagreeing with the run order is a trap for the first test that sorts one"
  - "`check_goods` and its localiser are near-copies of the cash pair and are deliberately NOT factored into one generic scan over a residual selector: the two residuals are different quantities, and the saving would be one line at the cost of a message that cannot say which one it found"
  - "`Violation` now carries `Option<Box<Posting>>`. With a second posting-bearing variant the enum passes 128 bytes, at which `clippy::result_large_err` under `-D warnings` refuses to compile every `Result` in the crate that propagates a violation. Boxing preserves ownership — the property the by-value decision was actually about — at one allocation on a path that has already decided to abort"
  - "A negative `firm.initial_inventory_units` is refused at construction. The configuration layer bounds the money stock but not this key, and the identity cannot catch it: a negative endowment gives negative `produced` against negative stock and balances perfectly"

patterns-established:
  - "Unit arithmetic in a compute step uses bare integer operators, not a checked API: units are not `Money` and have no non-panicking half, and both profiles enable overflow checks, so an unrepresentable count aborts before the first write rather than wrapping into a plausible negative inventory (T-02-17)"
  - "Every goods refusal is asserted against a clone of the books taken before the attempt — 'it returned an error' and 'it wrote nothing' are two different claims"
  - "A cross-phase claim is demonstrated by construction, not stated: the two Phase 7 consumption models are evaluated by ONE function pointer taken from `ALL_CHECKS` and read through the SAME three accessors"

requirements-completed: []

coverage:
  - id: D1
    description: "The books own every goods unit, addressed identically to cash, with `produce`, `consume` and `exchange` as the only unit-mutation points — each compute-then-commit, each returning the quantity actually moved, and every refusal writing nothing"
    requirement: "LEDG-03, LEDG-05"
    verification:
      - kind: unit
        ref: "src/books.rs#production_raises_both_the_stock_and_the_produced_total"
        status: pass
      - kind: unit
        ref: "src/books.rs#consumption_lowers_the_stock_raises_the_consumed_total_and_posts"
        status: pass
      - kind: unit
        ref: "src/books.rs#consuming_beyond_the_stock_is_refused_and_writes_nothing"
        status: pass
      - kind: unit
        ref: "src/books.rs#every_refused_exchange_moves_neither_cash_nor_units"
        status: pass
      - kind: unit
        ref: "src/books.rs#an_unknown_good_is_refused_rather_than_indexed"
        status: pass
      - kind: other
        ref: "grep -cE 'pub fn exchange\\(|pub fn produce\\(|pub fn consume\\(' src/books.rs == 3; grep -vE '^[[:space:]]*//' src/books.rs | grep -cE 'pub fn [a-z_]+.*-> *&([a-z_]+ )?mut ' == 0; grep -cE 'RefCell|Rc<|Arc<|Mutex|dyn |impl Fn|FnMut|FnOnce' src/books.rs == 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "A cash-for-units swap is ONE posting that moves both and reports both, so it cannot be half-applied (T-02-14), and it counts as a transaction while production and consumption do not (LEDG-08)"
    requirement: "LEDG-03"
    verification:
      - kind: unit
        ref: "src/books.rs#a_completed_exchange_moves_both_and_reports_both"
        status: pass
      - kind: unit
        ref: "src/books.rs#the_transaction_count_rises_for_an_exchange_and_not_for_a_production"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#a_production_only_tick_passes_goods_conservation_and_fails_liveness"
        status: pass
    human_judgment: false
  - id: D3
    description: "The goods identity is checked every tick against two independently maintained sources — the identity recomputed from the fields against the running residual accumulated from the posting legs — and it holds at tick 0 because the initial inventory is counted into `produced`"
    requirement: "LEDG-05"
    verification:
      - kind: unit
        ref: "src/books.rs#construction_endows_inventory_and_counts_it_into_produced"
        status: pass
      - kind: unit
        ref: "src/books.rs#a_negative_initial_inventory_is_refused_at_construction"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#the_identity_holds_at_tick_zero_before_anything_has_happened"
        status: pass
      - kind: other
        ref: "grep -c 'GoodsConservation' src/invariants.rs == 7 (identifier, violation variant, table entry, message, tests)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The identity keeps one shape under BOTH Phase 7 consumption models — no formula, field or check differs between a unit consumed in the same tick and one held across a boundary"
    requirement: "LEDG-05"
    verification:
      - kind: unit
        ref: "src/invariants.rs#immediate_consumption_holds_the_identity_at_every_step"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#held_stock_across_a_tick_boundary_holds_the_same_identity"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#both_consumption_models_use_the_same_check_and_the_same_accessors"
        status: pass
    human_judgment: false
  - id: D5
    description: "Goods conservation is second in `ALL_CHECKS`, immediately after money conservation, and localisation is a forward linear scan over the goods residual kept separate from the cash one"
    requirement: "LEDG-05"
    verification:
      - kind: unit
        ref: "src/invariants.rs#localisation_names_the_first_break_and_not_a_later_one"
        status: pass
      - kind: unit
        ref: "src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks"
        status: pass
      - kind: integration
        ref: "tests/invariant_halt.rs#the_gate_removes_exactly_one_check_and_never_disables_the_phase"
        status: pass
      - kind: other
        ref: "grep -cE 'binary_search|\\bmid\\b|\\bhi\\b' src/invariants.rs == 0"
        status: pass
    human_judgment: false
  - id: D6
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
duration: 12 min
completed: 2026-08-31
status: complete
---

# Phase 2 Plan 03: Books, Journal and Invariants — Goods Summary

**The books now own every unit as well as every cent, on the same addressing and the same posting discipline: `produce`, `consume` and `exchange` are the only unit-mutation points, a cash-for-units swap is one posting that cannot be half-applied, and `produced − consumed − Σstock == 0` is checked every tick against two independently maintained sources — proved to hold under both Phase 7 consumption models with no difference in formula, field or check.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-08-31T09:26:00Z (approx — first task commit at 09:33:40Z)
- **Completed:** 2026-08-31T09:38:20Z
- **Tasks:** 2 of 2
- **Files modified:** 3

## Accomplishments

- **`src/books.rs` — the goods half of the ledger.** `Books` gained `household_stock` and `firm_stock` (the same two-vector shape as cash, indexed by the same resolved slot) plus running `produced` and `consumed` totals. `produce`, `consume` and `exchange` are each compute-then-commit — good, sign, account, cash and stock all settled before the first write, and a commit step of assignments plus the recorder — and each returns the quantity actually moved. `exchange` returns both, as `(Money, i64)`.
- **A swap is ONE posting.** `exchange` records a single journal line carrying both legs of the cash and both legs of the units. Two postings could be half-applied; one cannot. That is threat T-02-14 closed, and it is what lets plan 02-04 check LEDG-07 on a single posting rather than on an aggregate.
- **The 02-02 stub is closed.** `Posting::units_out`, `units_in` and `goods_residual_units` carry real values on every goods posting. The recorder needed no structural change — only its inputs did, exactly as 02-02 predicted.
- **The identity holds at tick 0.** Every firm slot is endowed with `firm.initial_inventory_units`, recorded as a second `Endow` posting carrying the units arriving, **and counted into `produced` in the same loop iteration**. Without that count the identity would fail on tick 0 by exactly the endowment — the defect 02-RESEARCH named as the most likely one in this phase. Both the constructor's doc comment and a dedicated test pin it.
- **`src/invariants.rs` — the goods identity as a real check.** `check_goods` compares the identity recomputed from the fields against the running residual accumulated from the posting legs, for every good the books carry, and reports a failure if either side is non-zero. It is **second** in `ALL_CHECKS`, immediately after money conservation, and localises with its own forward linear scan over the goods residual.
- **The "one shape" claim is demonstrated, not stated.** `both_consumption_models_use_the_same_check_and_the_same_accessors` builds an immediate-consumption economy and a held-stock economy, asserts they genuinely differ (`stock_of(buyer) == Some(0)` versus `Some(2)`), and then evaluates both through **one function pointer taken from `ALL_CHECKS`** and **the same three accessors**. Phase 7 (MKT-06) can settle its open question without touching a formula, a field or a check here.
- **The two liveness-adjacent checks cannot be mistaken for each other.** A production-only tick passes goods conservation — every unit is accounted for — and fails liveness, because nothing traded.

## Task Commits

Each task was committed atomically:

1. **Task 1: The books own every unit — production, consumption and the cash-for-units swap** — `475c538` (feat)
2. **Task 2: The goods identity as a real check, proved under both Phase 7 consumption models** — `f4c802c` (feat)

**Plan metadata:** see the `docs(02-03)` commit that follows this file.

## Files Created/Modified

- `src/books.rs` (modified, 832 → 1,762 lines) — three operations, six accessors, three `PostError` variants, two `BooksError` variants, three `PostingKind` variants, the recorder's identity delta, the constructor's inventory endowment, and 11 unit tests under `books::goods`.
- `src/invariants.rs` (modified, 386 → 743 lines) — `CheckId::GoodsConservation`, `Violation::GoodsConservation`, `check_goods`, `first_breaking_goods_posting`, the three-entry `ALL_CHECKS`, and 6 unit tests under `invariants::goods`.
- `tests/invariant_halt.rs` (modified, +10 −2) — the active-check sequence assertion, now three entries with the gate on and two with it off.

## Decisions Made

Recorded in full in the `key-decisions` frontmatter. The three with the longest reach:

1. **The units leg points opposite to the cash leg.** For an `Exchange` the buyer is the debit account because it pays, so `units_out` leaves the *credit* account and `units_in` arrives at the *debit* one. 02-02 left those field docs written as "left `debit`" / "arrived at `credit`", which was harmless while both were always zero and would have been wrong the moment 02-04 read them. They now state the convention explicitly, and it is the one 02-RESEARCH's Pattern 4 specifies (`units` moved credit → debit for a sale).
2. **`Violation` carries the posting boxed.** Forced by `clippy::result_large_err` — see Deviation 2. Plans 02-04 and 02-05 construct `Violation` values and must write `Some(Box::new(posting))`.
3. **The three per-good totals return zero for an unknown good; `stock_of` returns `None`.** The asymmetry is deliberate and documented: a total over a good the books do not carry is genuinely zero, because every mutation refuses an unknown good before touching a vector; but `stock_of` names a *specific account*, and reporting a plausible zero about one is a lie rather than a fact.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `tests/invariant_halt.rs` had to change, though it is outside `files_modified`**

- **Found during:** Task 2
- **Issue:** `the_gate_removes_exactly_one_check_and_never_disables_the_phase` asserts the exact active sequence `[MoneyConservation, Liveness]`. Inserting goods conservation at position two — which the plan mandates — makes that assertion fail. The plan's own `<verify>` block requires `cargo test --locked --release --test invariant_halt` to pass, so the two instructions are only jointly satisfiable by updating the test.
- **Fix:** Updated both sequence assertions (the integration one and its unit-level twin in `src/invariants.rs`) to the three-entry order, with the gate-off case asserting `[MoneyConservation, GoodsConservation]` and an added message naming what the gate is supposed to remove. The *behavioural* claims of the file are untouched: the loop still halts at tick 4 with `Violation::Liveness`, and still runs all ten ticks with the gate off.
- **Files modified:** `tests/invariant_halt.rs`, `src/invariants.rs`
- **Verification:** `cargo test --locked --release --test invariant_halt` → 3 passed.
- **Committed in:** `f4c802c`
- **Note for 02-04:** it inserts two more checks and must update these same two assertions.

**2. [Rule 3 - Blocking] `Violation` crossed `clippy::result_large_err`'s threshold and stopped compiling**

- **Found during:** Task 2
- **Issue:** `GoodsConservation` carries five `i64`s, a `GoodId` and an optional `Posting`. With a second posting-bearing variant the enum reached 128 bytes, at which `clippy::result_large_err` — implied by the project's `-D warnings` wall — refuses to compile `Result<(), Violation>`. Every check, `CheckSet::run`, and the integration test's `run_loop` return exactly that type, so the crate did not build under clippy at all. `cargo test` was green throughout, which is precisely why this is worth recording: the lint wall, not the test suite, is what caught it.
- **Fix:** `Option<Posting>` → `Option<Box<Posting>>` on both variants, with `render_posting` taking the boxed form (`Display` reaches through the `Box`) and both call sites doing `.map(Box::new)`. This is the remedy the lint's own help text names. It preserves the property the by-value decision was actually about — the violation *owns* the posting rather than indexing a buffer `end_of_tick` is about to clear — at the cost of one allocation on a path that has already decided to abort the run. The alternative, `#[allow(clippy::result_large_err)]`, would have had to be repeated in `tests/invariant_halt.rs` and in every future consumer including Phase 3's `main.rs`; the box fixes it once, at the definition.
- **Files modified:** `src/invariants.rs`
- **Verification:** `cargo clippy --all-targets --all-features -- -D warnings` clean; `bash tests/lints.sh` OK (its check 1 is what re-runs the wall); all 173 release tests pass.
- **Committed in:** `f4c802c`

**3. [Rule 2 - Missing critical] A negative `firm.initial_inventory_units` was accepted and the identity could not catch it**

- **Found during:** Task 1
- **Issue:** `Params::validate` bounds `money.total_money_cents` but imposes no bound on `firm.initial_inventory_units`. A negative value would open the books with every firm holding negative inventory — and the goods identity would still balance, because `produced` would be equally negative. No check in this phase or the next can detect it; the plan's own threat register (T-02-17) says negative counts are refused "at the operation boundary", and construction is such a boundary.
- **Fix:** `Books::new` computes the total endowment with `checked_mul` and refuses both a negative per-firm value and a total outside the integer range, via a new `BooksError::InitialInventoryOutOfRange { units_per_firm, firms }`. That bound is also what lets the endowment loop use bare integer arithmetic safely. This is one more `BooksError` variant than the plan's `<artifacts_produced>` lists (which names only a goods-mismatch variant, shipped as `InventoryDoesNotBalance`).
- **Files modified:** `src/books.rs`
- **Verification:** `src/books.rs#a_negative_initial_inventory_is_refused_at_construction` asserts the exact error value.
- **Committed in:** `475c538`

**4. [Rule 2 - Missing critical] `exchange` refuses self-dealing**

- **Found during:** Task 1
- **Issue:** The plan's compute step for `exchange` lists a negative amount, a negative count, an overdraft and short stock, but not a buyer and seller that are the same account. `transfer` already refuses that shape with `PostError::SelfDealing`, on the stated ground that a two-party posting naming one account on both legs is not well formed and that 02-04's zero-sum check reports exactly it.
- **Fix:** `exchange` refuses `buyer == seller` with the existing `PostError::SelfDealing`. No new variant.
- **Files modified:** `src/books.rs`
- **Verification:** covered in `src/books.rs#every_refused_exchange_moves_neither_cash_nor_units`, which asserts the value and then that nothing was written.
- **Committed in:** `475c538`

### Plan Defects (no source change)

**5. [Rule 1 - Bug] Task 1's `grep -c 'pub fn produce'` acceptance criterion is unsatisfiable**

- **Found during:** Task 1 verification
- **Issue:** The criterion requires `grep -c 'pub fn produce' src/books.rs` and `grep -c 'pub fn consume' src/books.rs` each to print `1`. Both print `2`, because each pattern is a **substring of the accessor the same plan mandates** in `<artifacts_produced>` — `Books::produced` and `Books::consumed`. The criterion and the artifact list cannot both be satisfied as written.
- **Fix:** Evaluated the anchored form that expresses the same intent, `grep -cE 'pub fn produce\(' src/books.rs` → `1`, and likewise for `consume` and `exchange`. No source change: this is a plan defect. Recorded in `.planning/WINDOWS.md` so plans 02-04 through 02-07 do not copy the pattern.
- **Files modified:** none
- **Committed in:** n/a

### Deliberate refinements

- **`CashSlot` renamed to `AccountSlot`** (private type). It now indexes stock as well as cash, from one resolution — which is what keeps a firm's inventory keyed on the same slot as its money across a Phase 10 respawn (T-02-16). Leaving it named for one of the two quantities it addresses would have been actively misleading.
- **`CheckId` declares `GoodsConservation` in run position rather than appended.** The plan says "append"; the enum derives `Ord` and is not a wire shape, so declaring it in run order makes the derived ordering agree with `ALL_CHECKS` instead of quietly contradicting it. `ALL_CHECKS` itself was *inserted* into exactly as instructed. `PostingKind`, which **is** a wire shape, was strictly appended.
- **`first_breaking_posting` renamed to `first_breaking_cash_posting`** so the pair is symmetric with `first_breaking_goods_posting`, and neither name can be read as "the" residual.

---

**Total deviations:** 4 auto-fixed (2 × Rule 3 blocking, 2 × Rule 2 missing-critical), plus 1 plan defect with no source change.
**Impact on plan:** None on scope or architecture. Deviation 2 changes a public type shape that plans 02-04 and 02-05 construct, and is the one a reader of this summary most needs. Deviations 1, 3 and 4 close gaps the plan left implicit; deviation 5 is a criterion defect.

## Authentication Gates

None.

## Issues Encountered

None outstanding. The `result_large_err` failure was the only surprise, and `cargo test` being green while `cargo clippy` refused to compile is the interesting part of it — the lint wall is load-bearing here, not hygiene.

## Known Stubs

**None from this plan.** The stub 02-02 declared is now closed: `Posting::units_out`, `units_in` and `goods_residual_units` carry real values on every goods posting, and `.planning/WINDOWS.md` entry 4 has been marked fixed.

Two forward obligations, both plan-owned and neither a stub in this plan's output:

| Obligation | Where | Owner |
|---|---|---|
| The goods residual is one books-wide quantity, not per-good. With one good in v1 that is exact; `check_goods` already loops over `Books::goods()`, so the widening is a change of the container's dimension | `src/books.rs` `goods_residual_units`, `src/invariants.rs` `check_goods` | Phase 5 (PROD-01) |
| `Violation::GoodsConservation` is constructible but not yet reachable through the public API — no path can produce a non-zero residual, which is the point of the phase. Its failure machinery is exercised here only through `first_breaking_goods_posting` over a hand-built journal | `src/invariants.rs` | Plan 02-05 (seeded corruptions) |

## Threat Flags

None. No new network endpoint, auth path, file access or trust boundary. The plan's register (T-02-13 … T-02-17, T-02-SC) is addressed as written: units move only through postings; `exchange` is one posting with an infallible commit; the check compares two separately maintained sources over a loop body that is entered every tick; stock is keyed on the firm slot behind the same generational resolution as cash; unit counts are signed 64-bit under overflow checks with negative counts refused at every boundary including construction; and no package was installed (`--locked` builds are green and `Cargo.lock` is unchanged).

## Self-Check: PASSED

- `src/books.rs`, `src/invariants.rs`, `tests/invariant_halt.rs` — all present on disk.
- Commits `475c538`, `f4c802c` — both present in `git log --oneline --all`.
- `git diff --diff-filter=D --name-only 475c538~1 HEAD` — empty; no file was deleted.
- `git status --short` — clean after each task commit; no untracked files left behind.
- All task-level `<acceptance_criteria>` re-run and passing:
  - `cargo test --locked --release --lib -- books` → 18 passed, 0 failed
  - `cargo test --locked --release --lib -- books::goods` → 11 passed (not `0 filtered out`)
  - `cargo test --locked --release --lib -- invariants` → 10 passed
  - `cargo test --locked --release --lib -- invariants::goods` → 6 passed (≥ 3 required)
  - `cargo test --locked --release --test invariant_halt` → 3 passed
  - `grep -cE 'pub fn exchange\(' src/books.rs` → `1`; same for `produce\(` and `consume\(` (see Deviation 5 on the unanchored form)
  - `grep -vE '^[[:space:]]*//' src/books.rs | grep -cE 'pub fn [a-z_]+.*-> *&([a-z_]+ )?mut '` → `0`
  - `grep -cE '\bf16\b|\bf32\b|\bf64\b|\bf128\b'` → `0` for both modules
  - `grep -cE 'binary_search|\bmid\b|\bhi\b' src/invariants.rs` → `0`
  - `grep -cE 'debug_assert|debug_assertions'` → `0` for both modules
  - `grep -cE 'RefCell|Rc<|Arc<|Mutex|dyn |impl Fn|FnMut|FnOnce' src/books.rs` → `0`
  - `grep -c 'GoodsConservation' src/invariants.rs` → `7` (≥ 3 required)
  - `grep -c 'to_string().contains' src/invariants.rs` → `0`
- Plan-level `<verification>` re-run and passing:
  - `cargo test --locked --all-targets` → 10 suites ok, 0 failed (175 tests)
  - `cargo test --locked --release --all-targets` → 10 suites ok, 0 failed (173 tests; the two-test delta is the pre-existing debug-gated RNG re-entry pair)
  - `cargo test --locked --test numeric_det` → 5 passed
  - `cargo clippy --all-targets --all-features -- -D warnings` → clean
  - `cargo fmt --check` → clean
  - `bash tests/lints.sh` → OK (checks 1–4)
  - `bash tests/toolchain.sh` → OK
- `.planning/STATE.md` and `.planning/ROADMAP.md` untouched by this plan.
- `.planning/REQUIREMENTS.md` untouched: `requirements.ready-ids` reports `0/2 ready` — plan 02-07 also declares LEDG-05 and LEDG-03 is likewise still owed, so neither may read `Complete` yet. Respected per the shared-artifact rule.

## Next

Plan **02-04** (zero-sum and non-negativity: LEDG-06, LEDG-07) may begin. It reads `Posting::units_out`/`units_in`, which now carry real values, and inserts two further checks at positions three and four — updating the two active-sequence assertions named in Deviation 1, and constructing `Violation` values with the posting boxed per Deviation 2.
