---
phase: 02-books-journal-and-invariants
plan: 07
subsystem: testing
tags: [proptest, property-testing, ledger, conservation, invariants, rust]

requires:
  - phase: 02-books-journal-and-invariants
    provides: "Books::transfer/produce/consume/exchange, the running cash and goods residuals, Books::accounts, and the accessors these properties read (plans 02-02, 02-03, 02-04)"
  - phase: 02-books-journal-and-invariants
    provides: "the five-check ALL_CHECKS table and CheckSet::from_params, whose verdict the conservation property cross-checks against its own direct comparison (plans 02-04, 02-05)"
  - phase: 01-foundations
    provides: "the proptest conventions in tests/money_props.rs — explicit case count, FileFailurePersistence::Direct at a committed path, and deliberately edge-weighted strategies"
provides:
  - "tests/ledger_props.rs — six properties over arbitrary sequences of public ledger operations, plus a guard test pinning the carried-goods table"
  - "a generator for valid-and-adversarial operation sequences whose refusal regions and success region are both weighted by measurement rather than by hope"
  - "the two-source agreement property: the posting-derived residuals asserted against the balance-derived quantities, so the independence the whole phase rests on is stated rather than assumed"
  - ".proptest-regressions/ledger_props.txt — committed, so a counterexample found once is replayed forever"
affects: [phase-03-tick-loop-and-logging, phase-05-production, phase-06-labour, phase-07-goods-market, phase-08-dividends]

actuals:
  tokens: 9200
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "mutation-checking a new property before trusting it, and recording in the property's own doc comment what it was measured to catch and what it structurally cannot"
    - "instrumenting a generator to count how often each branch actually fires, then weighting the strategy from the measurement"

key-files:
  created:
    - tests/ledger_props.rs
    - .proptest-regressions/ledger_props.txt
  modified: []

key-decisions:
  - "Liveness is disabled in the property parameter set: it is a claim about a whole tick, and these properties assert after every operation, where a produce-only prefix is a legitimate mid-tick state rather than a violation."
  - "A deliberate plausible-trade arm was added to the operation strategy after instrumentation measured exchange succeeding on only 5% of draws; the success branch of exchange_returns_match_deltas was close to untested."
  - "The goods identity and the two-source agreement are asserted from the accessors, never by calling check_goods, so weakening the check under test cannot make the property pass."
  - "Unit counts are bounded away from i64::MAX on purpose: Books::produce aborts there by design (T-02-17) rather than refusing, and an abort is not a conservation failure."
  - "The four proptest seeds written while mutation-testing were discarded rather than committed — they were counterexamples against deliberately broken code and shrink to a trivial no-op."

patterns-established:
  - "Property doc comments record the mutation that was tried and its outcome, so the next reader knows the property's teeth were measured rather than asserted."
  - "A property that structurally cannot fail from its own test level says so in its doc comment and is filed in WINDOWS.md, rather than being left to look like coverage."

requirements-completed: [LEDG-03, LEDG-04, LEDG-05]

coverage:
  - id: D1
    description: "No sequence of public ledger operations, however adversarial, changes the total money in the books (LEDG-04)"
    requirement: LEDG-04
    verification:
      - kind: integration
        ref: "tests/ledger_props.rs#total_money_is_conserved_under_any_operation_sequence"
        status: pass
    human_judgment: false
  - id: D2
    description: "The amount transfer returns equals the amount the books actually moved, and a refused transfer leaves balances, journal, transaction count and both residuals untouched (LEDG-02, LEDG-03)"
    requirement: LEDG-03
    verification:
      - kind: integration
        ref: "tests/ledger_props.rs#transfer_return_matches_delta"
        status: pass
    human_judgment: false
  - id: D3
    description: "The pair exchange returns matches all four observed balance and stock deltas, and a refused exchange changes nothing (LEDG-02, LEDG-03)"
    requirement: LEDG-03
    verification:
      - kind: integration
        ref: "tests/ledger_props.rs#exchange_returns_match_deltas"
        status: pass
    human_judgment: false
  - id: D4
    description: "produced minus consumed minus held stock is zero for every carried good after every operation in any random sequence (LEDG-05)"
    requirement: LEDG-05
    verification:
      - kind: integration
        ref: "tests/ledger_props.rs#goods_identity_holds"
        status: pass
    human_judgment: false
  - id: D5
    description: "The residuals accumulated from the posting legs agree with the same quantities recomputed from the balances after every operation — the two-source independence asserted directly (LEDG-04, LEDG-05)"
    requirement: LEDG-04
    verification:
      - kind: integration
        ref: "tests/ledger_props.rs#posting_residuals_agree_with_the_balance_derived_quantities"
        status: pass
      - kind: other
        ref: "mutation check: adding +1 to record()'s Transfer cash_delta fails this property and the conservation property, and nothing else"
        status: pass
    human_judgment: false
  - id: D6
    description: "Ending a tick empties the journal and zeroes the transaction count while leaving every balance and stock untouched"
    verification:
      - kind: integration
        ref: "tests/ledger_props.rs#ending_a_tick_leaves_the_residuals_and_the_balances_untouched"
        status: pass
      - kind: other
        ref: "mutation check: deleting journal.clear() from end_of_tick fails this property and no other in the file"
        status: pass
    human_judgment: false
  - id: D7
    description: "A counterexample found once is replayed forever — failure persistence points at a committed repository path"
    verification:
      - kind: other
        ref: "git ls-files --error-unmatch .proptest-regressions/ledger_props.txt"
        status: pass
    human_judgment: false

duration: 14min
completed: 2026-08-31
status: complete
---

# Phase 02 Plan 07: Property Tests Summary

**Six properties over arbitrary sequences of public ledger operations — conservation, both return-agreement claims, the goods identity, two-source agreement and the tick boundary — each mutation-checked, with the one clause that structurally cannot fail from an integration test documented as such rather than counted as coverage.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-08-31T10:08Z
- **Completed:** 2026-08-31T10:22Z
- **Tasks:** 2
- **Files created:** 2

## Accomplishments

- **The two-source agreement is now asserted rather than assumed.** The residuals `record()` accumulates from the posting legs are compared directly against the quantities recomputed from the balance and stock vectors, after every operation of every generated sequence. This is the property the rest of the phase rests on: `check_money` and `check_goods` are non-vacuous only because their two inputs are independent, and a change that derived one from the other would leave both checks passing forever. Mutation-checked — perturbing `record()`'s cash delta on `Transfer` postings alone fails this property and the conservation property, while `transfer_return_matches_delta` correctly stays green, because the balances are still right. That is precisely the "ledger perfect, derived total leaking" shape LEDG-03 exists to catch.
- **The generator's success region was weighted from measurement, not intuition.** Instrumenting it revealed `exchange` succeeding on 50 draws against 875 refusals — about one in twenty. An exchange needs six conditions at once and households open holding no stock, so a uniformly drawn seller usually cannot sell; the success branch of `exchange_returns_match_deltas` was close to untested. A deliberate household-buys-from-firm arm brings it to 391 against 850, and as a side effect puts stock into household hands so a household's `consume` and a resale can happen at all.
- **All five refusal regions the plan names are reached deliberately** — zero amounts, over-balance amounts, negative amounts, over-stock unit counts, and self-directed operations named on both legs by construction — plus the two unknown-address regions, including the subtle one: a live firm slot named at a generation that does not occupy it, which is Phase 10's respawn shape.
- **The refusal-atomicity claim is now exhaustive over inputs.** Every refused `transfer` and `exchange` is checked to have left both balances, both stocks, the journal length, the transaction count and both running residuals exactly as they were. Plan 02-06 covers the panic path; between them no path writes half an operation.
- **A coverage gap was found and recorded rather than papered over** — see Issues Encountered.

## Task Commits

1. **Task 1: Strategies and the two money properties** — `0cb2f09` (test)
2. **Task 2: Goods identity, two-source agreement, tick boundary** — `04630ed` (test)

## Files Created

- `tests/ledger_props.rs` (804 lines) — the parameter helper, four strategies, the operation strategy, six properties and one guard test.
- `.proptest-regressions/ledger_props.txt` — committed with its header, so proptest appends to a tracked file rather than creating an untracked one.

## Decisions Made

**Liveness is off in the property parameter set.** LEDG-08 is a claim about a whole tick — that money changed hands before the tick closed. These properties assert after *every operation*, and a sequence that has so far only produced units is a legitimate mid-tick state. Leaving the gate on would have made most sequences fail for a reason unrelated to what is under test. The gate is proved end to end in `tests/invariant_halt.rs`, at the tick level where it means something.

**The goods identity is read from the accessors, never from `check_goods`.** A property that only calls the check under test proves the check is self-consistent. Weakening `check_goods` to `Ok(())` leaves a check-calling property green on a ledger losing units; it does not leave this one green.

**Unit counts stop well short of `i64::MAX`.** `Books::produce` adds the count to a stock with bare integer arithmetic under this project's overflow checks, so a huge count *aborts* rather than being refused — T-02-17 working as designed, before any write. Generating one would test the panic rather than the algebra.

**The ordinary cent band is `1..5_000`, off the round numbers.** The configured liquidities are 5 000 and 50 000 cents, so no draw coincides with a whole balance and a rule that happens to work on exactly one endowment gets no help. Two upper arms straddle the largest balance so the overdraft boundary is crossed from both sides.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical coverage] `exchange`'s returns were not covered by any property**

- **Found during:** Task 1
- **Issue:** The plan specifies `transfer_return_matches_delta` and nothing for `exchange`. But `exchange` returns a **pair**, and `src/books.rs` says in as many words that a caller must use both — an accumulator bumped by the intended unit count while the cash leg is taken from the return value leaks on one side only. LEDG-03 applies to it identically, and leaving it uncovered would have left the two-legged operation, the harder one, with the weaker guarantee.
- **Fix:** Added `exchange_returns_match_deltas`, asserting both returned quantities against four observed deltas (buyer cash, seller cash, buyer stock, seller stock) on success, and full non-mutation on refusal.
- **Files modified:** `tests/ledger_props.rs`
- **Verification:** `cargo test --locked --release --test ledger_props exchange_returns_match_deltas` — 1 passed.
- **Committed in:** `0cb2f09`

**2. [Rule 2 - Missing critical coverage] The generator reached `exchange`'s success path once in twenty draws**

- **Found during:** Task 1
- **Issue:** Instrumenting the generator measured 50 successful exchanges against 875 refusals. A property whose interesting branch fires that rarely is close to not being tested, and the rate would have fallen silently as the economy grew. The plan's discipline is explicit that a generator which never reaches a region proves only that the other one works — the same reasoning applies to the success region, not just the refusal regions.
- **Fix:** Added an explicit plausible-trade arm — a household buying from a firm at counts and amounts both sides can meet. Re-instrumented: 391 successes against 850 refusals.
- **Files modified:** `tests/ledger_props.rs`
- **Verification:** Generator instrumented before and after via a throwaway probe binary (not committed); both branch counts recorded in the strategy's doc comment.
- **Committed in:** `0cb2f09`

---

**Total deviations:** 2 auto-fixed (2 × Rule 2)
**Impact on plan:** Both strengthen the requirement the plan already owns (LEDG-03) rather than widening scope. No new files, no new dependency, no source change.

## Issues Encountered

**The tick-boundary property's residual clause cannot fail from an integration test.** The plan asks the tick-boundary property to assert that ending a tick leaves both running residuals untouched, reasoning that "a tick-boundary bug that resets a residual would leave every other property green and quietly disable conservation from tick one onward". The property is written and it passes — but mutation-checking it showed it does not catch that bug. Adding `self.cash_residual_cents = 0;` to `Books::end_of_tick` leaves it green.

The reason is structural, not a defect in the property. On the honest path the books conserve, so the cash residual is *already* zero at every boundary; setting it to zero changes nothing observable. Making it observable requires a seeded non-zero residual, which requires the `pub(crate)` corruption vocabulary — and an integration test under `tests/` cannot reach that, which is the same property that makes every other assertion in this file a statement about what a real caller can do.

Handled three ways rather than quietly:

1. The property's own doc comment states what was tried, that it stayed green, and why — so nobody reads the clause as coverage it is not.
2. Recorded in `.planning/WINDOWS.md` as an `unmet-truth`, naming plan 02-06 (which owns the fault-injection unit tests) as the place the version with teeth belongs.
3. The rest of the property was mutation-confirmed to have real teeth: deleting `journal.clear()` from `end_of_tick` fails this property **and no other in the file**, because every other property runs inside a single tick. That is the property earning its place.

**Four proptest seeds were written during mutation checking and discarded.** Running the properties against deliberately broken `books.rs` caused proptest to persist counterexamples to the tracked regression file. They are counterexamples against code that no longer exists and shrink to a trivial no-op self-transfer. Restored the file to its committed header with `git checkout -- .proptest-regressions/ledger_props.txt`. `src/books.rs` was restored from a backup taken before the first mutation and verified byte-identical with `git diff --stat src/books.rs` (empty).

**The ROADMAP side effect recorded as WINDOWS entry 11 did not occur.** `.planning/STATE.md` and `.planning/ROADMAP.md` are untouched — no state-advancing command was run, per the wave shared-artifact rule while 02-06 is outstanding.

## Verification

Full suite, both profiles, before returning:

| Command | Result |
|---|---|
| `cargo test --locked --all-targets` | 158 + 14 + 14 + 7 + 8 + 6 + 5 + 4 + 4 + 3 passed, 0 failed |
| `cargo test --locked --release --all-targets` | 156 + 14 + 14 + 7 + 8 + 6 + 5 + 4 + 4 + 3 passed, 0 failed |
| `cargo test --locked --release --test ledger_props` | 7 passed (6 properties + 1 guard) |
| `cargo test --locked --release --test ledger_props transfer_return_matches_delta` | 1 passed, 6 filtered out (not 0 matching) |
| `cargo test --locked --release --test ledger_props goods_identity_holds` | 1 passed, 6 filtered out (not 0 matching) |
| `bash tests/lints.sh` | OK — all 60 resolvable method bans fire |
| `bash tests/toolchain.sh` | OK |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no dependency added |
| `git ls-files --error-unmatch .proptest-regressions/ledger_props.txt` | tracked |

## User Setup Required

None.

## Next Phase Readiness

Plan 02-06 (wave 7) is unblocked. Two things are handed to it:

- **The panic half of atomicity.** This plan covers every *refused* input exhaustively; 02-06 owns the panic path. Together they close LEDG-02 over both failure modes.
- **One item of its own.** The residual-survives-the-tick-boundary assertion needs a seeded non-zero residual and therefore belongs in 02-06's unit tests, where the corruption vocabulary is in scope. Filed in `.planning/WINDOWS.md`.

Note for 02-06 and any later plan adding a check: WINDOWS entries 7 and 9 still apply — the active-check sequence assertions in `tests/invariant_halt.rs` and `src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks` must be updated together. This plan added no check and did not touch either.

For Phase 5 onward: `the_books_carry_exactly_the_good_these_strategies_name` fails the moment PROD-01 widens the goods table, which is deliberate — it is what stops the uncarried-good refusal region from silently emptying. The `goods_identity_holds` and agreement properties already loop over `books.goods()` and widen by themselves.

---
*Phase: 02-books-journal-and-invariants*
*Completed: 2026-08-31*

## Self-Check: PASSED

All created files exist on disk; both task commits (`0cb2f09`, `04630ed`) are present in git;
all six property symbols resolve in `tests/ledger_props.rs`; `.planning/STATE.md` and
`.planning/ROADMAP.md` are untouched.
