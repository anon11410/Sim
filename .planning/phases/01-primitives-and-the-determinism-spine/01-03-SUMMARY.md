---
phase: 01-primitives-and-the-determinism-spine
plan: 03
subsystem: core-primitives
tags: [rust, money, integer-cents, checked-arithmetic, thiserror, proptest, determinism]

# Dependency graph
requires:
  - phase: 01-01
    provides: "The crate spine and the thin `Money` the tracer needed — private `i64` field, `from_cents`, `cents`, `ZERO` and the single checked `Add` impl, plus `[profile.release] overflow-checks = true` and the committed toolchain pin"
  - phase: 01-02
    provides: "REQUIREMENTS.md as the current contract after the CORE-03/CORE-10/CORE-11 amendments"
provides:
  - "`Money` complete: the panicking operator set (`Add`, `Sub`, `Neg`, `AddAssign`, `SubAssign`, `Sum` for both `Money` and `&Money`), each routing through an `i64::checked_*` primitive so it aborts in every build profile rather than only where `overflow-checks` is set"
  - "`MoneyOverflow`, a `thiserror` struct carrying `lhs`, `op` and `rhs`, so a failed operation can be quoted rather than reported as a bare word"
  - "The named non-panicking API `Money::checked_add`, `Money::checked_sub` and `Money::try_scale(num, den)` returning `Result<Money, MoneyOverflow>` — the surface `src/config.rs` calls so an absurd supplied `total_money_cents` becomes a named `ConfigError` instead of a process abort"
  - "`Money::split(n)`, the conserving division every dividend and pro-rata split in the model will use: the first `|amount % n|` recipients by ascending index each receive one extra cent, on both signs"
  - "`tests/money_props.rs` — four properties including the dedicated non-evenly-dividing case that a remainder-dropping implementation cannot pass"
  - "`.proptest-regressions/money_props.txt`, tracked, so a counterexample found later in CI is replayed forever rather than lost"
affects: [01-06 config parameter set, 02 ledger and transfer, 08 ownership and dividends, 09 firm planning]

actuals:
  tokens: 4600
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Split overflow API (D-07): operators panic, named methods return `Result`. Both ship; neither substitutes for the other."
    - "Conserving division (D-09): remainder distributed one cent at a time by ascending index, never discarded."
    - "Mutation-checked acceptance: each task's tests were proven load-bearing by breaking the implementation and observing the specific tests go red."

key-files:
  created:
    - tests/money_props.rs
    - .proptest-regressions/money_props.txt
  modified:
    - src/money.rs

key-decisions:
  - "`Money::try_scale` multiplies before dividing, so the ratio keeps full precision inside the integer domain with no intermediate rounding and no float; it truncates toward zero on both signs and reports a zero denominator as `Err`, not a panic."
  - "The split-conservation tests live in a sibling module `money::split_tests`, not nested under `money::tests`, because cargo's test filter is a substring match on the full test path — under a nested module the path would read `money::tests::split::…`, which does not contain the plan's verification string `money::split`."
  - "`MoneyOverflow` uses `op: \"*\"` for a `try_scale` multiplication overflow and `op: \"/\"` for a zero or unrepresentable denominator, extending the plan's explicitly specified `\"+\"` / `\"-\"` shape."
  - "The committed `.proptest-regressions/money_props.txt` carries proptest's header only. Counterexamples were produced during the mutation check, but they are counterexamples against a deliberately broken implementation and recording them would misrepresent the file as a record of real regressions."
  - "`failure_persistence` is pinned to `FileFailurePersistence::Direct(\".proptest-regressions/money_props.txt\")` rather than left on proptest's default source-parallel rule, so counterexamples land at the committed repository path by construction."

patterns-established:
  - "Pattern: every arithmetic path in `src/money.rs` names an `i64::checked_*` primitive. There is no bare `+`, `-` or `/` on the money field anywhere in the module, so correctness does not depend on a Cargo profile setting."
  - "Pattern: the float boundary is enforced by absence and asserted by grep — `src/money.rs` names no floating-point type at all, so a source scan (not a code review) is the check."
  - "Pattern: TDD gates are real gates — the failing test is committed first, and the passing implementation is a separate commit, so the history shows the test could fail."

requirements-completed: [CORE-01]

coverage:
  - id: D1
    description: "Every `Money` operator panics on overflow in the debug profile and the release profile alike, because each routes through an `i64::checked_*` primitive rather than relying on `[profile.release] overflow-checks`"
    requirement: CORE-01
    verification:
      - kind: unit
        ref: "cargo test --lib money:: (14 tests, incl. adding_one_cent_past_the_maximum_panics, subtracting_one_cent_below_the_minimum_panics, negating_the_minimum_panics, add_assign_past_the_maximum_panics, a_sum_that_would_overflow_panics)"
        status: pass
      - kind: unit
        ref: "cargo test --release --lib money:: (same 14 tests under the release profile)"
        status: pass
      - kind: other
        ref: "mutation: replacing the checked add with wrapping_add makes `cargo test --release --lib money::` exit 101 with 3 failures; reverted"
        status: pass
    human_judgment: false
  - id: D2
    description: "The named API `checked_add`, `checked_sub` and `try_scale` returns `Result<Money, MoneyOverflow>` and never panics, so config ingestion reports an absurd supplied amount as a named error"
    requirement: CORE-01
    verification:
      - kind: unit
        ref: "src/money.rs#money::tests::checked_add_at_the_maximum_returns_the_named_error (asserts the exact MoneyOverflow { lhs: i64::MAX, op: \"+\", rhs: 1 })"
        status: pass
      - kind: unit
        ref: "src/money.rs#money::tests::checked_sub_returns_ok_and_a_named_error_at_the_minimum"
        status: pass
      - kind: unit
        ref: "src/money.rs#money::tests::try_scale_truncates_toward_zero_and_reports_overflow"
        status: pass
    human_judgment: false
  - id: D3
    description: "`Money::split(n)` conserves every cent and distributes the remainder in a specified, stable order — the first `|amount % n|` recipients by ascending index, on both signs; `n == 0` panics rather than silently destroying the amount"
    requirement: CORE-01
    verification:
      - kind: unit
        ref: "cargo test --lib money::split (8 tests, incl. the_remainder_goes_to_the_first_recipients_by_ascending_index asserting [334, 333, 333] by value)"
        status: pass
      - kind: unit
        ref: "cargo test --release --lib money::split (same 8 tests under the release profile)"
        status: pass
      - kind: other
        ref: "mutation: replacing the body with vec![Money(base); n] makes `cargo test --lib money::split` exit 101 with 3 failures; reverted"
        status: pass
    human_judgment: false
  - id: D4
    description: "A property test over amounts that do NOT divide evenly proves the parts of `split` sum exactly back to the whole, so a remainder-discarding implementation fails rather than passing on round numbers (ROADMAP criterion 1)"
    requirement: CORE-01
    verification:
      - kind: integration
        ref: "tests/money_props.rs#split_parts_sum_to_the_whole_when_not_evenly_divisible (512 cases, prop_assume!(amount % n != 0))"
        status: pass
      - kind: integration
        ref: "tests/money_props.rs#split_part_spread_is_at_most_one_cent"
        status: pass
      - kind: integration
        ref: "tests/money_props.rs#add_then_subtract_round_trips"
        status: pass
      - kind: other
        ref: "mutation: a remainder-dropping split turns exactly this property red, and proptest wrote its counterexample to .proptest-regressions/money_props.txt, confirming the persistence path wiring; reverted"
        status: pass
    human_judgment: false
  - id: D5
    description: "The float boundary is intact at the money type — no conversion to or from a floating-point type, no float multiplication, no decimal `Display`"
    requirement: CORE-01
    verification:
      - kind: other
        ref: "grep -c 'f64' src/money.rs == 0 and grep -c 'as f' src/money.rs == 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "Committed proptest regression file, so a rare counterexample found later in CI is replayed on every future run"
    requirement: CORE-01
    verification:
      - kind: other
        ref: "git ls-files --error-unmatch .proptest-regressions/money_props.txt"
        status: pass
    human_judgment: false

duration: 5 min
completed: 2026-08-30
status: complete
---

# Phase 01 Plan 03: The Money Type, Completed Summary

**`Money` finished in the D-07 split shape — checked operators that abort in every build profile, a `Result`-returning named API for config ingestion, and a `split` whose remainder rule is specified, stable and property-tested against the non-evenly-dividing case that a cent-dropping implementation would otherwise pass.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-08-30T23:21:38Z
- **Completed:** 2026-08-30T23:27:00Z
- **Tasks:** 3
- **Files modified:** 3 (1 modified, 2 created)

## Accomplishments

- **Both halves of D-07 exist simultaneously.** The operator set (`Add`, `Sub`, `Neg`, `AddAssign`, `SubAssign`, `Sum` for `Money` and for `&Money`) routes through `i64::checked_*` and `.expect("… overflow …")`, so it panics under `cargo test` and under `cargo test --release` alike — the check lives in the code, not in the profile. Alongside it, `checked_add`, `checked_sub` and `try_scale` return `Result<Money, MoneyOverflow>` and never panic, which is the surface `src/config.rs` needs so a supplied `total_money_cents` of absurd magnitude surfaces as a named `ConfigError` rather than aborting the process (threat T-1-03).
- **`Sum` folds through the checked `Add`, never over a raw integer accumulator** (D-08) — the one path that would have wrapped silently. An empty iterator sums to `Money::ZERO`.
- **`Money::split(n)` cannot destroy a cent.** The remainder goes to the first `|amount % n|` recipients by ascending index; `split(1000, 3)` is asserted by value as `[334, 333, 333]`. Negative amounts are handled by preserving the exact sum rather than by taking an absolute value, so `split(-1000, 3)` is `[-334, -333, -333]`. `n == 0` asserts with a message naming the zero recipient count instead of returning an empty vector, which would silently destroy the whole amount.
- **The load-bearing property test exists and was proven load-bearing.** `split_parts_sum_to_the_whole_when_not_evenly_divisible` carries `prop_assume!(amount % n != 0)`, which is exactly what ROADMAP criterion 1 demands. Reverting `split` to `vec![Money(base); n]` turns it red; it is not a property that passes by accident on round numbers.
- **The float boundary is intact by absence.** `src/money.rs` names no floating-point type anywhere — not in code, not in a doc comment — so `grep -c 'f64'` returning 0 is a real check rather than a stylistic one (threat T-1-11).

## Task Commits

Each task was committed atomically; the two TDD tasks are a RED commit followed by a GREEN commit.

1. **Task 1: The two halves of the overflow contract** — `f281e1f` (test, RED) → `58d1ab2` (feat, GREEN)
2. **Task 2: `Money::split` with a specified, stable remainder rule** — `5975ae3` (test, RED) → `2512103` (feat, GREEN)
3. **Task 3: Property tests, including the case that catches a remainder-dropping split** — `94261e2` (test)

_Both RED commits genuinely fail: `f281e1f` produces 20 compile errors for the not-yet-existing API, `5975ae3` produces 4 for the missing `split`. The failing state is in the history, so the gate is auditable rather than asserted._

## Files Created/Modified

- `src/money.rs` — the complete `Money` newtype. Adds `MoneyOverflow`, the named `Result` API, the five remaining operator impls plus both `Sum` impls, and `split`. Carries 22 unit tests in two modules: `money::tests` (14) and `money::split_tests` (8).
- `tests/money_props.rs` — four proptest properties reaching the type through `use sim::money::{Money, MoneyOverflow}`, which is also part of CORE-08's proof that integration tests under `tests/` can reach all code. Case count pinned at 512 and `failure_persistence` pinned to the committed path, so neither run time nor counterexample location depends on the environment.
- `.proptest-regressions/money_props.txt` — tracked, carrying proptest's own header comment.

## Decisions Made

1. **`try_scale` multiplies before dividing.** Keeping the full product in the integer domain means the ratio suffers no intermediate rounding, and `i64` division already truncates toward zero on both signs. `try_scale(1000, 3, 4) == 750` and `try_scale(-1000, 3, 4) == -750`.
2. **`MoneyOverflow`'s `op` field extends to `"*"` and `"/"`.** The plan specified the exact error value only for `checked_add` (`"+"`). `try_scale` reports a multiplication overflow as `"*"` and a zero or unrepresentable denominator as `"/"`, keeping the "quote the arithmetic that failed" property across the whole named API.
3. **Split tests live in `money::split_tests`, a sibling of `money::tests`.** The plan's verification command is `cargo test --lib money::split`, and cargo's filter is a substring match on the full test path. Nesting the tests under `money::tests` would produce paths like `money::tests::split::…`, which does not contain `money::split`, so the command would have selected zero tests and the `fails_when` condition ("running 0 tests") would have tripped. A comment on the module records why the name is what it is.
4. **`failure_persistence` is set to `Direct(".proptest-regressions/money_props.txt")`.** Proptest's default `SourceParallel` rule resolves relative to the source file, which for an integration test does not land at the repository-root path the plan requires. Pinning it makes the location a property of the file. Verified during the mutation check: proptest wrote its counterexample to exactly that path.
5. **The committed regressions file carries the header only.** No counterexample exists against the real implementation. The two counterexamples produced during the mutation check were found against a deliberately broken `split`, and committing them would present the file as a record of real regressions that never happened.

## Deviations from Plan

None — plan executed exactly as written. All three tasks, every line of both behavior blocks, and every acceptance criterion including the two mutation checks were executed as specified. The items above are decisions within the plan's stated latitude, not departures from it.

**Total deviations:** 0
**Impact on plan:** None.

## Issues Encountered

1. **A `+` mutation would not have proven what the acceptance criterion intends.** Task 1's criterion says "replacing any one `.expect(` with an unchecked operator makes `cargo test --release --lib money::` exit non-zero". Because this repository sets `[profile.release] overflow-checks = true` (CORE-02, D-10), a plain `self.0 + other.0` still panics in release — with the message `attempt to add with overflow`, which contains the substring `overflow` and therefore still satisfies `#[should_panic(expected = "overflow")]`. The mutation was run with `wrapping_add` instead, which is the honest test of the criterion's intent and produced exit 101 with three failures. **This is itself the argument for D-07's operator half:** the second belt is not redundant, it is the one that holds when `overflow-checks` is absent, and only a truly unchecked operation can distinguish the two.
2. **Task 1's first implementation pass included `split`, which was removed before commit.** Writing it there would have made Task 2's RED gate vacuous — the tests would have passed on first run and the gate would have proven nothing. `split` was stripped from `58d1ab2` and reintroduced in `2512103` after `5975ae3` recorded the genuine failure.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **CORE-01 is satisfied and demonstrable**, and this is the last of the three Phase 1 plans declaring it (01-01, 01-02, 01-03), so it is now markable complete.
- **Phase 2's ledger has its contract.** `transfer()` is written against `checked_sub`/`checked_add` for the cash-mutation point and against `split` for pro-rata distribution; LEDG-03's "callers subtract the amount actually transferred" is expressible because `split` returns the exact parts rather than a nominal share.
- **Phase 8's dividends inherit the ascending-index remainder rule.** It is documented on the function as the reason not to refactor it, because changing it later would alter every committed run's trajectory and force every golden log and snapshot to be regenerated.
- **Plan 01-06 (config parameter set)** should call `try_scale`/`checked_*` rather than the operators wherever an operator-supplied value is involved — that is the entire reason the named API exists.
- No blockers. `cargo test` and `cargo test --release` are both fully green across all five targets (22 lib, 4 money_props, 5 tracer).

## Self-Check: PASSED

- `src/money.rs` — FOUND
- `tests/money_props.rs` — FOUND
- `.proptest-regressions/money_props.txt` — FOUND and tracked by git
- Commits `f281e1f`, `58d1ab2`, `5975ae3`, `2512103`, `94261e2` — all FOUND
- Plan `<verification>` re-run at close: `cargo test` → 22 + 4 + 5 pass; `cargo test --release` → 22 + 4 + 5 pass; `git ls-files .proptest-regressions/money_props.txt` → prints the path; `grep -c 'f64' src/money.rs` → 0.

---
*Phase: 01-primitives-and-the-determinism-spine*
*Completed: 2026-08-30*
