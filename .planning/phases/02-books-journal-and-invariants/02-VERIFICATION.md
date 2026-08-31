---
phase: 02-books-journal-and-invariants
verified: 2026-08-31T11:38:04Z
status: passed
score: 4/4 must-haves verified
# was gaps_found 3/4; the single gap was closed by e0ee1b4 and re-verified independently — see Gap Closure below
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "The negative test passes for every check: an invariant never observed to fire has never been shown to work"
    status: resolved
    resolved_by: e0ee1b4
    resolved_at: 2026-08-31
    reason: >-
      Four of the five checks are mutation-proven to fire on a seeded fault. The fifth —
      `check_goods` (goods conservation, LEDG-05) — has never been observed to return `Err`.
      Replacing its entire body with `if true { return Ok(()); }` leaves the FULL suite green:
      239 tests across 12 binaries, 0 failures. Every one of the four call sites that reach the
      goods check in the codebase asserts `Ok(())`; `Violation::GoodsConservation` is
      constructed only by the production code itself and by a hand-built fixture in the
      `message` module that never runs the check. This is the fifth instance of the defect
      shape 02-REVIEW found four times: an assertion whose stated claim is not what it measures.
    artifacts:
      - path: "src/invariants.rs:581"
        issue: >-
          `check_goods` is dead weight as far as the suite is concerned — it can be deleted
          without a red test. Verifier probe confirms the function itself is CORRECT (it fires
          with the right violation and the right localised posting on a seeded goods leak), so
          this is a coverage gap, not a correctness defect.
      - path: "src/invariants.rs:1190,1305"
        issue: >-
          `invariants::goods` asserts `goods_check()(&books, ..) == Ok(())` and nothing else.
          `localisation_names_the_first_break_and_not_a_later_one` calls
          `first_breaking_goods_posting` directly on a hand-built array, bypassing `check_goods`.
      - path: "src/invariants.rs:2303,2340"
        issue: >-
          `invariants::negative` calls `check_for(CheckId::GoodsConservation)` twice, both
          asserting `Ok(())` as a control for another check. Never as the subject.
      - path: ".planning/phases/02-books-journal-and-invariants/02-05-PLAN.md:21"
        issue: >-
          Plan 02-05 scoped exactly four violation classes (LEDG-04, LEDG-06, LEDG-07, LEDG-10).
          Goods conservation was never in scope for a negative test, and nothing in
          `.planning/WINDOWS.md` records the omission — so it is an undetected gap rather than
          a knowingly-deferred one.
    missing:
      - >-
        A seeded goods-conservation negative test in `src/invariants.rs::goods` driving
        `goods_check()` to `Err(Violation::GoodsConservation { .. })` by whole-value equality,
        using the `pub(crate)` corruption vocabulary already in scope in that module. The
        verifier's throwaway probe (below) passes as written and is ~25 lines.
      - >-
        Cover BOTH arms of the check independently: the journal-residual arm (reached by
        `corrupt_appended_posting` with disagreeing unit legs — touches no stock, so
        `delta_units == 0` and only `journal_residual_units` is non-zero) and the
        balance-derived `produced − consumed − Σstock` arm. Neutering the whole function
        proves neither arm is exercised today.
      - >-
        A control assertion in the same test that money conservation and non-negativity still
        return `Ok(())` on the same books, matching the discipline
        `a_conserving_move_that_drives_an_account_negative_is_not_a_conservation_failure`
        already applies.
deferred:
  - truth: "`Household` and `Firm` carry no balance fields and expose no `set_cash`"
    addressed_in: "Phase 3"
    evidence: >-
      ROADMAP Phase 3 success criterion 7: "The balance-field obligation inherited from Phase 2
      criterion 1. `Household` and `Firm` first exist in this phase … Guard 7f in
      `tests/lints.sh` is extended to name those two types in the same commit that introduces
      them." The agent types do not exist in Phase 2 (`src/config.rs`'s `Household`/`Firm` are
      parameter sections, not agents), so the claim is not yet checkable.
  - truth: "A release binary with a seeded violation still halts (process level: non-zero exit, message on stderr)"
    addressed_in: "Phase 3"
    evidence: >-
      ROADMAP Phase 3 success criterion 6: "The process-level half of Phase 2 criterion 2. The
      built binary … exits non-zero with a stderr line naming tick 0. Phase 2 can only prove the
      halt at the library level … because the `const PHASES` table and the binary's tick loop
      are this phase's." The library-level release halt IS proved here (see criterion 4).
  - truth: "The liveness gate is on by default from Phase 6"
    addressed_in: "Phase 6"
    evidence: >-
      ROADMAP Phase 6 success criterion 7: "The liveness gate flips on here.
      `invariants.liveness_enabled` is `true` in `config/baseline.toml` by the end of this
      phase, flipped in the commit that first makes wages move money."
human_verification: []
---

# Phase 2: Books, Journal and Invariants — Verification Report

**Phase Goal:** A single ledger owns every cent and every goods unit, and the invariant checks
halt the run on the tick a violation occurs, naming the offending posting.
**Verified:** 2026-08-31T11:38:04Z
**Status:** gaps_found
**Re-verification:** No — initial verification
**Method:** Goal-backward, adversarial. Every criterion resting on a test was subjected to
**source mutation in an isolated copy** (`git archive HEAD` into a scratch tree; the working
repo was verified clean before and after and was never modified). A green test was accepted as
evidence only where a named mutation makes it red.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | No type outside `books` can move value; `transfer()` is the only cash-mutation point, and it is atomic | ✓ VERIFIED | Atomicity proved by mutant discrimination (`NaiveBooks` ends at −400 having unwound; real books end at 100 having returned). Guard 7f proved to fire on a real injection. Agent-type half formally deferred to Phase 3 criterion 7. |
| 2 | **The negative test passes for every check** — four seeded leaks halt the run naming tick, agent and posting, localised by a linear scan | ✗ FAILED | The four *named* faults are all mutation-proven. **`check_goods` (the fifth check) is never observed to fire** — its entire body can be replaced by `Ok(())` with 239/239 tests green. |
| 3 | The liveness invariant halts a build in which a tick records zero transactions; config-gated off, on from Phase 6 | ✓ VERIFIED | Halt at exactly `SILENT_TICK` with a control run and a `reached` witness proving the loop stopped. Both liveness *bypasses* (zero-cent transfer, empty exchange) mutation-proven closed. |
| 4 | Invariants run in release as a real pipeline phase returning `Result`; no `debug_assert!` on the invariant path; a release binary with a seeded violation still halts | ✓ VERIFIED | Zero `debug_assert!` in `src/`. Full suite green under `--release`, including all seven `invariants::negative::` tests. `debug_assertions`-off in release proved empirically. Process-level half deferred to Phase 3 criterion 6. |

**Score:** 3/4 truths verified (0 present, behavior-unverified)

---

## Criterion 1 — Only `books` moves value, and `transfer()` is atomic

**Verdict: ✓ VERIFIED** (with one half formally deferred, see Deferred Items)

### The falsifiable form, as delivered

The ROADMAP's phrasing *"a test observing the books mid-transaction is impossible to write"* is
unfalsifiable and was superseded by `02-RESEARCH.md`. Plan 02-06 delivered the falsifiable
replacement, and it is genuine:

`tests/ledger_atomicity.rs` drives seven refusals through a `panic::catch_unwind` +
`AssertUnwindSafe` harness asserting four separate claims each (did not unwind / returned the
right refusal / every captured quantity unchanged / journal did not grow). The `Snapshot` struct
compares ten quantities as a whole, so a quantity added in a later phase is caught the moment it
is added rather than silently omitted.

The two assertions that make the file non-vacuous:

- `the_naive_ordering_unwinds_and_corrupts_its_total_under_the_same_harness` — the `NaiveBooks`
  mutant (decrement → check → increment) driven through the *identical* harness ends at **−400**
  against an opening 100, having unwound. Its failure message states the consequence outright:
  "the harness cannot discriminate between the two designs, so every atomicity assertion above
  is vacuous."
- `a_transfer_that_can_complete_still_commits` — the control. Without it, a `transfer` that
  refused *everything* would satisfy all seven refusal tests.

**Compute-then-commit is real in the source.** `transfer` (`src/books.rs:833`), `produce`
(`:908`), `consume` (`:959`) and `exchange` (`:1025`) each carry an explicit
`--- compute: every fallible step, before any write ---` / `--- commit: assignments only ---`
split, and every commit step is assignments plus one `record` call, none of which can fail.

### The lint gate is not a paper guarantee

`bash tests/lints.sh` → exit 0, seven checks, ten source guards. Each guard's grep pattern is
**self-tested against an inline hazard fixture with an exact expected match count**
(`assert_fires`), so a pattern typo is a hard failure rather than a silent always-pass.

I did not take that on trust. **Adversarial end-to-end injection:** appending
`pub fn set_cash(_v: i64) {}` to `src/numeric.rs` (before its test module, so the tree still
passes clippy) produces:

```
FAIL: guard 7f: a cash setter is declared under src/. The books own the quantity; there is
nothing for an agent to set (LEDG-01). … found: src/numeric.rs:147:pub fn set_cash(_v: i64) {}
```

The two compile-fail probes also execute rather than being asserted: check 5 refuses a shared
borrow held across a mutation with **E0502**; check 6 refuses the fault-injection vocabulary
from `tests/` with **E0599** — which is the compiler, not a review, enforcing that the
`#[cfg(test)] pub(crate)` corruption methods are unreachable from any consumer of the crate.

### Mutations that make this criterion's tests fail

| Mutation | Result |
|---|---|
| Delete the `EmptyTransfer` refusal from `Books::transfer` | 4 failures: `books::tests::every_refusal_leaves_the_books_exactly_as_it_found_them`, `invariants::liveness::a_tick_whose_only_transfer_moved_nothing_still_fails_liveness`, `ledger_props::every_counted_transaction_moved_money`, `ledger_props::total_money_is_conserved_under_any_operation_sequence` |
| Delete the `EmptyExchange` refusal from `Books::exchange` | 4 failures: `books::goods::every_refused_exchange_moves_neither_cash_nor_units`, `invariants::zero_sum::the_public_api_refuses_the_shapes_this_check_looks_for`, and the same two properties |
| Inject a `set_cash` under `src/` outside the ledger | `tests/lints.sh` guard 7f fails, naming file and line |

---

## Criterion 2 — The negative test passes for every check

**Verdict: ✗ FAILED — 🛑 BLOCKER**

### What passes (and is mutation-proven)

All four *named* seeded faults exist as real negative tests, asserted by **whole-value
equality** against a constructed `Violation` — never a message substring — and each is driven
through `ALL_CHECKS` by identifier (`check_for(CheckId::…)`), so a check removed from the
production table fails the test rather than continuing to pass in isolation.

| Named fault | Test | Neutering the check it targets |
|---|---|---|
| A dropped cent | `negative::a_dropped_cent_recorded_as_a_posting_is_reported_as_a_leak_and_localised` | `check_money` → `Ok(())` ⇒ **6 failures** |
| An over-credited sale | `negative::an_over_credited_posting_is_a_leak_and_the_same_books_also_break_zero_sum` | (same, plus `check_zero_sum` → **2 failures**) |
| A driven-negative balance | `negative::a_conserving_move_that_drives_an_account_negative_is_not_a_conservation_failure` | `check_non_negative` → `Ok(())` ⇒ **5 failures** |
| A non-zero-sum trade | `negative::a_synthesised_posting_breaks_only_the_structural_check` | `check_zero_sum` → `Ok(())` ⇒ **2 failures** |

The halt-and-stop claim is split into its two parts rather than conflated:
`a_seeded_leak_aborts_the_tick_loop_at_the_tick_it_occurred` asserts both the exact violation
value **and** `reached == Some(CORRUPTED_TICK)` — "it returned an error" and "it stopped" are
different claims. `the_identical_loop_with_no_seeded_leak_runs_every_tick` is the control, so a
loop that halted for an unrelated reason fails rather than looking like a working negative test.

**Localisation by linear scan is real and mutation-proven.** Reversing the scan in
`first_breaking_cash_posting` (`.rev().find(…)` — the bisection signature) fails
`localise::the_first_break_is_reported_even_when_a_later_posting_heals_the_residual` and
`localise::the_monotone_case_reports_the_only_break`. The first reproduces the researched
counterexample exactly — broken at #50, healed at #120, broken again at #200 — and asserts the
residual really does cancel (`1 → 0 → 7`) before asserting the scan answers 50, so the test
would prove nothing a monotone journal already proves if the cancellation were absent. The
superseded "bisect" spelling appears nowhere in `src/invariants.rs`.

### What fails

**`check_goods` — goods conservation, LEDG-05 — has never been observed to fire.**

Mutation, in the isolated copy:

```rust
fn check_goods(books: &Books, tick: u32) -> Result<(), Violation> {
    if true { return Ok(()); }        // <- entire check neutered
    for &good in books.goods() { … }
}
```

`cargo test --locked --all-targets` result: **every one of the 12 test binaries green,
0 failures** (163 + 0 + 14 + 14 + 4 + 3 + 10 + 8 + 4 + 5 + 6 + 8 = 239 tests).

Corroborated by exhaustive grep — every call site that reaches the goods check asserts success:

```
src/invariants.rs:1195:  assert_eq!(goods_check()(&books, 0), Ok(()));
src/invariants.rs:1305:  assert_eq!(goods_check()(&books, 3), Ok(()));
src/invariants.rs:2303:  assert_eq!(check_for(CheckId::GoodsConservation)(&books, 9), Ok(()));
src/invariants.rs:2340:  assert_eq!(check_for(CheckId::GoodsConservation)(&books, 11), Ok(()));
```

`Violation::GoodsConservation` is constructed at only three places: `src/invariants.rs:604`
(the production code itself) and `:2744` / `:2763` (a hand-built fixture in the `message`
module, which renders it without ever running the check). `invariants::goods::
localisation_names_the_first_break_and_not_a_later_one` calls `first_breaking_goods_posting`
directly on a hand-built `[Posting; 4]`, so it exercises the scan helper but not the check.

**The check itself is correct — this is a coverage gap, not a correctness defect.** I inserted a
throwaway probe into an unmutated copy and it passes as written:

```rust
let posting = books.corrupt_appended_posting(Posting {
    kind: PostingKind::Exchange, debit: buyer(), credit: seller(),
    debit_cents: 400, credit_cents: 400, good: FOOD,
    units_out: 3, units_in: 1, ..
});
assert_eq!(books.goods_residual_units(), 2);
assert_eq!(goods_check()(&books, 5), Err(Violation::GoodsConservation {
    tick: 5, good: FOOD, delta_units: 0, journal_residual_units: 2,
    posting: Some(Box::new(posting)), ..
}));
// → test invariants::goods::VERIFIER_PROBE_… ok
```

So the closure is ~25 lines in the `invariants::goods` module, using vocabulary already in
scope there.

**Why this is a blocker rather than a nit.** Criterion 2's own standard is the sentence *"An
invariant never observed to fire has never been shown to work."* Goods conservation is one of
the two conservation identities the phase goal names ("every cent **and every goods unit**"),
it currently has no regression protection whatsoever, and Phase 5 is scheduled to change the
accessors underneath it (`Books::total_stock`, `produced`, `consumed` and
`goods_residual_units_for` all take a `GoodId` and today ignore it past the `carries` check —
documented on `src/books.rs`'s `GOODS` array). A refactor that silently breaks the check will
be invisible to CI on the exact commit it lands.

**This is the fifth instance of the defect shape 02-REVIEW found four times** — an assertion
whose stated claim is not what it actually measures — and unlike the other four it is not
recorded anywhere. Plan 02-05 scoped exactly four violation classes and `.planning/WINDOWS.md`
has no entry for the omission, so it was never noticed rather than knowingly deferred.

---

## Criterion 3 — The liveness invariant

**Verdict: ✓ VERIFIED**

`tests/invariant_halt.rs::with_the_gate_on_the_loop_halts_at_exactly_the_tick_that_traded_nothing`
asserts by whole value `Err(Violation::Liveness { tick: 4, counted: 0, required: 1 })`, checks
the rendered message names `tick 4`, and asserts `reached == Some(SILENT_TICK)` — the loop
stopped, it did not swallow and continue.
`with_the_gate_off_the_identical_loop_runs_every_tick` is the control, and the gate is the only
difference between the two. `the_gate_removes_exactly_one_check_and_never_disables_the_phase`
asserts the full active **sequence** in both states, not a length (a length assertion passes
when two checks are swapped).

Both parameters are loaded through the real `config::load` path and only
`invariants.liveness_enabled` is set afterwards, which proves the gate is read from the
parameters rather than from a constant compiled into the check set.

**The shipped value is correct.** `config/baseline.toml:174` → `liveness_enabled = false`, with
a provenance annotation (`GRADE: PROJECT | SOURCE: ROADMAP Phase 2 criterion 3`) and a matching
`config/PROVENANCE.md:105` row. The three-file agreement is policed by
`provenance::no_annotation_is_orphaned` and `every_config_key_has_a_provenance_row`, so a
half-flip in Phase 6 is a red test rather than silent drift.

**Guard 7e** enforces that the key is named under `src/` in exactly `src/config.rs` (declared)
and `src/invariants.rs` (read), and read **exactly once** in the production half of the latter —
verified: `CheckSet::from_params` at `src/invariants.rs:484` is the sole read.

### Both liveness bypasses are closed at the boundary AND at the check

The review found that an empty `exchange` and a zero-cent `transfer` each incremented the
transaction count while moving nothing. `Books::record` still counts any `Transfer` or
`Exchange` posting unconditionally — the guard is at the public API, in the compute step:
`PostError::EmptyTransfer` (`src/books.rs:849`) and `PostError::EmptyExchange` (`:1039`). The
check side is closed too: `ZeroSumDetail::EmptyTransfer` and `::EmptyExchange` name a posting
that somehow reaches the journal, and `every_counted_transaction_moved_money` asserts per
posting that `debit_cents > 0 && credit_cents > 0` for every counted kind.

### Mutations that make this criterion's tests fail

| Mutation | Result |
|---|---|
| `check_liveness` returns `Ok(())` unconditionally | **5 failures**: `liveness::a_tick_that_traded_nothing_fails_only_because_the_gate_is_on`, `liveness::a_tick_whose_only_transfer_moved_nothing_still_fails_liveness`, `liveness::the_transaction_count_resets_each_tick_so_liveness_is_a_per_tick_property`, `goods::a_production_only_tick_passes_goods_conservation_and_fails_liveness`, and `invariant_halt::with_the_gate_on_…` |
| Reopen either liveness bypass | see Criterion 1's mutation table |

---

## Criterion 4 — Release builds, `Result`, no `debug_assert!`

**Verdict: ✓ VERIFIED** (process-level half deferred to Phase 3 criterion 6)

**A real pipeline phase returning `Result`.** `pub type CheckFn = fn(&Books, u32) -> Result<(),
Violation>` (`src/invariants.rs:433`); `ALL_CHECKS` is a `const [(CheckId, &str, CheckFn); 5]`;
`CheckSet::run` returns `Result<(), Violation>` and propagates with `?`. Not a trait object, not
a closure, no per-tick config lookup, no branch on the gate.

**The `grep` clause, run:**

```
$ grep -rn "debug_assert" src/
src/numeric.rs:126:  /// If `x` is not finite. The check is **unconditional**, not a `debug_assert!`.
src/numeric.rs:258:  // These were all `debug_assert!`, so in a release build …
src/rng.rs:506:      // Both were `debug_assert!` and so compiled out of the release build, where …
```

Three doc/comment mentions, **zero call sites**. The only `#[cfg(debug_assertions)]` in `src/`
is Phase 1's RNG sub-stream re-entry guard in `src/rng.rs`, which guard 7d whitelists by name
with an exact expected-match-count self-test and a failure message that states the reason
(`overflow-checks = true` does not enable `debug_assertions`, so a release run would otherwise
be silently unchecked).

**Release execution, run:**

```
$ cargo test --locked --release --all-targets     # exit 0
```

All 12 binaries green, including all seven `invariants::negative::` tests and all three
`tests/invariant_halt.rs` tests — so a seeded violation halts in a release-compiled artifact,
not merely in debug.

**Proof that `debug_assertions` really is off in that run** (rather than assumed): diffing the
debug and release test rosters shows exactly one test present in debug and absent in release —
`rng::tests::a_different_purpose_at_the_same_tick_and_agent_is_not_a_re_entry`, the one gated on
`#[cfg(debug_assertions)]`. Every invariant test is present in both.

`bash tests/toolchain.sh` → exit 0, asserting among other things that the release profile
enables `overflow-checks`, confirmed at `Cargo.toml [profile.release]`.

The remaining half — the built `sim` binary exiting non-zero with the message on stderr — is
not reachable in Phase 2 because `main.rs` has no tick loop by design (`const PHASES` is
Phase 3's). ROADMAP Phase 3 criterion 6 owns it explicitly and cross-references this phase.

---

## Deferred Items

Items not met here but explicitly addressed in later milestone phases.

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | `Household` / `Firm` carry no balance fields and expose no `set_cash` | Phase 3 | Criterion 7: the types first exist in Phase 3; guard 7f is extended "in the same commit that introduces them". `src/config.rs`'s `Household`/`Firm` are parameter sections, not agents. |
| 2 | Process-level halt: built binary exits non-zero with a stderr line naming the tick | Phase 3 | Criterion 6: "Phase 2 can only prove the halt at the library level … because the `const PHASES` table and the binary's tick loop are this phase's." |
| 3 | Liveness gate on by default | Phase 6 | Criterion 7: "The liveness gate flips on here … flipped in the commit that first makes wages move money." |

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/books.rs` (3,016 ln) | `Books`, journal, `transfer`/`produce`/`consume`/`exchange`, `pub(crate)` corruption vocabulary | ✓ VERIFIED | Compute-then-commit throughout; corruption methods `#[cfg(test)]` + `pub(crate)`, proved unreachable from `tests/` by an executed E0599 probe |
| `src/invariants.rs` (2,968 ln) | Five checks, `CheckSet`, `Violation`, three linear-scan localisers | ⚠️ PARTIAL | All five checks present, ordered and wired; `check_goods` has no negative test (see gap) |
| `src/ids.rs` (434 ln) | `Account`, `FirmId`, generational `FirmArena`, `Display` per address | ✓ VERIFIED | `Display` renders `household:12` / `firm:3:0`; postings serialise through it |
| `src/lib.rs` | Module surface | ✓ VERIFIED | `books`, `invariants` public; `#![forbid(unsafe_code)]` |
| `tests/invariant_halt.rs` (151 ln) | Library-level halt through the public API | ✓ VERIFIED | Halt + control + `reached` witness + active-sequence assertion |
| `tests/ledger_atomicity.rs` (447 ln) | Panic-atomicity with a discriminating mutant | ✓ VERIFIED | `NaiveBooks` fails the identical harness at −400; commit control present |
| `tests/ledger_props.rs` (902 ln) | Eight properties | ✓ VERIFIED (honestly bounded) | Two properties documented as structurally `0 == 0` from an integration test, with the teeth-bearing unit test named — both claims independently confirmed by mutation (below) |
| `tests/lints.sh` (785 ln) | 7 checks, 10 source guards, 2 compile-fail probes | ✓ VERIFIED | exit 0; each guard pattern self-tested against a hazard fixture; guard 7f additionally proved by real injection |
| `tests/lint-probes/` | 4 probe fixtures | ✓ VERIFIED | E0502 and E0599 probes execute in checks 5 and 6 |
| `config/baseline.toml` | `liveness_enabled = false` + provenance annotation | ✓ VERIFIED | Line 174; matching `config/PROVENANCE.md:105` row |

---

## Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `Books::transfer/exchange/produce/consume` | `Books::record` | private recorder, sole journal writer | ✓ WIRED | Every operation and every corruption method routes through it — no test can hand-fake a residual |
| `Books::record` | `cash_residual_cents` / `goods_residual_units` | derived from posting **legs**, not from balances | ✓ WIRED | Independence mutation-proven (see below) |
| `CheckSet::from_params` | `params.invariants.liveness_enabled` | single read, filter at construction | ✓ WIRED | Guard 7e enforces exactly one read site |
| `CheckSet::run` | `ALL_CHECKS` | ordered `fn` pointer table, `?` on first violation | ✓ WIRED | `order` module asserts the table against `CheckId::ALL` element-for-element |
| `check_money` / `check_goods` | `first_breaking_{cash,goods}_posting` | forward linear scan over the tick journal | ✓ WIRED | Cash side mutation-proven; goods side reached only by a direct helper test, never through `check_goods` |
| `Violation` | `Posting: Display` | boxed `Option<Posting>` rendered into the halt message | ✓ WIRED | `message` module asserts every variant; `None` branch says "no offending posting … outside the posting path" rather than inventing one |

---

## Data-Flow Trace (Level 4)

The two conservation checks depend entirely on their two sources being genuinely independent —
a single-source check compares a number against itself and passes forever (threat T-02-15).
This was traced and mutation-tested rather than read.

| Quantity | Source A (posting-derived) | Source B (balance-derived) | Independent? |
|---|---|---|---|
| Cash residual | `record`: `credit_cents − debit_cents`, accumulated from posting legs | `total_money() − opening_stock()`, summed from balance vectors | ✓ **Proven** |
| Goods residual | `record`: `units_in/out` per posting kind | `produced − consumed − Σstock`, maintained from operation *arguments* | ✓ **Proven** |

**The mutation that proves it.** Replacing `record`'s cash-residual arithmetic with the
single-source collapse:

```rust
let cash_delta = self.total_money().cents() - self.opening_stock.cents() - self.cash_residual_cents;
```

fails exactly one test — `books::tests::the_two_residual_sources_move_apart_when_only_one_of_
them_is_told` — and leaves all 8 properties in `tests/ledger_props.rs` green. **This is precisely
what that test's own doc comment claims**, down to the wording "that one unit test is the only
thing that fails."

Likewise, adding `self.cash_residual_cents = 0;` to `end_of_tick` fails only
`books::tests::ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched` and
leaves `ledger_props::ending_a_tick_leaves_the_residuals_and_the_balances_untouched` green —
again exactly as both files document.

**Assessment.** The two weak properties from 02-REVIEW were not papered over. They were left in
place, their blindness written into their doc comments in the terms the reviewer used, and each
was paired with a `#[cfg(test)]` unit test that has real teeth because it can reach the
`pub(crate)` corruption vocabulary an integration test cannot. Both claims verified
independently by this verifier. This is the standard the goods-conservation gap fails to meet.

---

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full suite, debug | `cargo test --locked --all-targets` | exit 0 — 239 tests, 0 failures across 12 binaries | ✓ PASS |
| Full suite, release | `cargo test --locked --release --all-targets` | exit 0 — 237 tests, 0 failures (1 test is `cfg(debug_assertions)`-only) | ✓ PASS |
| Lint / source-guard gate | `bash tests/lints.sh` | exit 0 — 7 checks, 60 method bans fire, 10 source guards, 2 compile-fail probes | ✓ PASS |
| Toolchain & profile gate | `bash tests/toolchain.sh` | exit 0 — release profile checks overflow, no codegen override, no OS-entropy crate | ✓ PASS |
| Clippy, all targets | `cargo clippy --all-targets --all-features -- -D warnings` | clean | ✓ PASS |
| Formatting | `cargo fmt --check` | clean | ✓ PASS |
| No skipped tests | `grep -rn "#\[ignore\]" src/ tests/` | none | ✓ PASS |
| Proptest regressions committed | `git ls-files .proptest-regressions/` | `ledger_props.txt`, `money_props.txt` | ✓ PASS |

### Mutation Matrix (verifier-run, isolated copy)

| # | Mutation | Tests made red | Verdict |
|---|---|---|---|
| M1 | `end_of_tick` zeroes the cash residual | 1 unit (`ending_a_tick_leaves_a_seeded_non_zero_residual…`); properties stay green | Caught, exactly as documented |
| M2 | `first_breaking_cash_posting` scans backwards (bisection signature) | 2 (`localise::*`) | Caught |
| M3 | `check_liveness` always `Ok(())` | 5 (4 unit + `invariant_halt`) | Caught |
| M4 | Delete `EmptyTransfer` refusal | 4 | Caught |
| M5 | Single-source collapse of `record`'s cash residual | 1 unit; properties stay green | Caught, exactly as documented |
| M6 | Delete `EmptyExchange` refusal | 4 | Caught |
| M7 | `check_money` always `Ok(())` | 6 | Caught |
| M8 | `check_non_negative` always `Ok(())` | 5 | Caught |
| M9 | `check_zero_sum` always `Ok(())` | 2 | Caught |
| **M10** | **`check_goods` always `Ok(())`** | **0 — full suite green** | 🛑 **NOT CAUGHT** |
| M11 | Inject `set_cash` under `src/` outside the ledger | `tests/lints.sh` guard 7f | Caught |

---

## Requirements Coverage

| Requirement | Description | Status | Evidence |
|---|---|---|---|
| LEDG-01 | Central `Books` owns every cent and goods unit; agents hold no balance | ✓ SATISFIED | Guards 7f/7g + E0502/E0599 probes; agent-type half deferred to Phase 3 criterion 7 |
| LEDG-02 | `transfer()` the only cash-mutation point, atomic | ✓ SATISFIED | `tests/ledger_atomicity.rs` with a discriminating mutant; guards 7a/7b/7c |
| LEDG-03 | `Money::split` distributes remainder deterministically; callers subtract what was actually transferred | ✓ SATISFIED | `money_props.rs` (4 properties); `transfer`/`exchange` return the moved quantity and `ledger_props::{transfer_return_matches_delta, exchange_returns_match_deltas}` verify return == delta. See Info I-2 |
| LEDG-04 | Money conservation checked every tick in release, exactly | ✓ SATISFIED | `check_money` runs first in `ALL_CHECKS`; mutation-proven (M7); passes under `--release` |
| LEDG-05 | Goods conservation checked every tick: produced − consumed = inventory | ⚠️ **PARTIAL** | The check is present, ordered second, loops over `books.goods()` so its body is entered every tick, and a verifier probe confirms it fires correctly. **But no test observes it fire (M10), so it has no regression protection** |
| LEDG-06 | Non-negativity across cash, inventory and headcount | ✓ SATISFIED | `check_non_negative`; mutation-proven (M8). Headcount arm honestly documented as type-level unrepresentable (`u32`) rather than written as an unreachable loop |
| LEDG-07 | Zero-sum trade checked per posting | ✓ SATISFIED | `check_zero_sum` + `well_formed`; 10 `ZeroSumDetail` shapes each with a message test, exhaustive by construction; mutation-proven (M9) |
| LEDG-08 | Liveness: transactions-per-tick > 0 | ✓ SATISFIED | Criterion 3; both bypasses closed and mutation-proven |
| LEDG-09 | Halt naming tick, agent and offending transaction, localised by linear scan | ✓ SATISFIED | Linear scan mutation-proven (M2) on the cancelling and monotone cases; the honest `None` branch is asserted rather than faked. Superseded "bisect" spelling absent from `src/invariants.rs` |
| LEDG-10 | Invariants a real pipeline phase returning `Result`, never `debug_assert!`; negative test proves a seeded leak halts | ✓ SATISFIED | Criterion 4; `a_seeded_leak_aborts_the_tick_loop_at_the_tick_it_occurred` asserts both the value and that the loop stopped |

**Orphaned requirements:** none. All ten LEDG IDs appear in the phase plans.

---

## Decision Coverage

Six decisions in `02-CONTEXT.md`'s `<decisions>` block; all six honored in shipped code:
agents own no value (guards 7f/7g); `transfer()` the sole atomic cash-mutation point
(`ledger_atomicity.rs`); invariants as a `Result`-returning pipeline phase, never `debug_assert!`
(guard 7d, zero call sites); per-tick journal buffer cleared by `end_of_tick`; the
**linear-scan correction** superseding "bisect" (mutation-proven, and the superseded spelling is
a hard grep failure); liveness config-gated off (guard 7e, `baseline.toml:174`).

**Honored: 6/6.** Non-blocking gate; no drift found.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| — | — | `TBD` / `FIXME` / `XXX` | — | **None.** Zero unreferenced debt markers in any file this phase touched |
| — | — | `TODO` / `HACK` / `PLACEHOLDER` | — | **None** |
| — | — | `#[ignore]` / skipped tests | — | **None** |
| — | — | Circular test fixtures | — | **None.** Expected values are hand-written literals or derived from the corruption's own arguments, never captured from the system under test |
| `src/invariants.rs` | 581 | Check with no negative test | 🛑 Blocker | See Criterion 2 gap |
| ROADMAP Phase 2 criterion 1 | — | Stale unfalsifiable phrasing | ℹ️ Info | I-1 below |
| `src/books.rs` | 833, 1025 | No `#[must_use]` on the moved-quantity return | ℹ️ Info | I-2 below |

### Info items (not gaps)

**I-1 — stale ROADMAP phrasing.** Criterion 1 still reads *"a test observing the books
mid-transaction is impossible to write"*, which `02-RESEARCH.md` superseded and plan 02-06
replaced with the falsifiable `catch_unwind` + `NaiveBooks` form. The delivered work is
strictly better than the criterion; only the criterion text is stale. Worth amending in
ROADMAP.md the way LEDG-09's localisation clause already was.

**I-2 — LEDG-03's caller obligation is documented, not enforced.** `transfer` and `exchange`
return the quantity actually moved and their doc comments say "a caller must use it", but
neither the methods nor `Money` carry `#[must_use]` beyond what `Result` gives for free —
`books.transfer(a, b, m).unwrap();` discards the returned `Money` without a warning. Harmless
in Phase 2 (there are no economic callers yet), but the leak this guards against — an
accumulator bumped by the *intended* rather than the *actual* amount — first becomes reachable
with Phase 6's partial payroll and Phase 8's dividends. A `#[must_use]` on the two methods
would make the obligation mechanical before the call sites exist.

---

## Test Quality Audit

| Test file | Linked reqs | Active | Skipped | Circular | Assertion level | Verdict |
|---|---|---|---|---|---|---|
| `tests/invariant_halt.rs` | LEDG-08/09/10 | 3 | 0 | No | Behavioral (halt + stop + control) | ✓ Strong |
| `tests/ledger_atomicity.rs` | LEDG-02 | 10 | 0 | No | Behavioral + mutant discrimination | ✓ Strong |
| `tests/ledger_props.rs` | LEDG-03/04/05/07/08 | 8 | 0 | No | Value (property) | ✓ Strong, with two known-blind properties honestly documented and separately discharged |
| `src/invariants.rs::negative` | LEDG-04/06/07/10 | 7 | 0 | No | Value (whole-`Violation` equality) | ✓ Strong |
| `src/invariants.rs::localise` | LEDG-09 | 2 | 0 | No | Value | ✓ Strong |
| `src/invariants.rs::message` | LEDG-09 | — | 0 | No | Substring, scoped to the rendering module only | ✓ Appropriate |
| `src/invariants.rs::goods` | **LEDG-05** | 5 | 0 | No | **Positive-only (`Ok(())`)** | 🛑 **Insufficient — no negative direction** |
| `tests/lints.sh` | LEDG-01/02/10 | 7 checks / 10 guards | 0 | No | Behavioral (each guard watched firing on a fixture) | ✓ Strong |

**Disabled tests on requirements:** 0.
**Circular patterns detected:** 0.
**Insufficient assertions:** 1 → LEDG-05 (goods conservation), escalated to BLOCKER because it
is the *only* coverage that requirement has.

---

## Human Verification

N/A — infrastructure/foundation phase with no user-facing elements. Every criterion is
verifiable programmatically and was verified by execution and by source mutation. No truth was
left ⚠️ PRESENT_BEHAVIOR_UNVERIFIED: each behavior-dependent claim (halt-and-stop, tick-boundary
residual survival, two-source independence, localisation across a cancelling residual, panic
atomicity) has a named mutation that makes its test red, listed in the Mutation Matrix.

---

## Gaps Summary

**One gap, and it is the same defect shape this phase already found four times.**

The phase is otherwise unusually strong. Every claim I tried to falsify held: the atomicity
harness genuinely discriminates against its mutant, the linear scan genuinely beats a bisection
on the cancelling journal, the liveness halt genuinely stops the loop rather than swallowing the
error, the two conservation sources are genuinely independent, and the two properties 02-REVIEW
exposed as vacuous were left in place with their blindness written into their own doc comments
and separately discharged by unit tests I independently confirmed have teeth. Nine of my ten
source mutations were caught, several by multiple independent tests.

The tenth was not. **`check_goods` — one of the two conservation identities the phase goal
names — can be deleted outright with the entire 239-test suite green.** It has never been
observed to fire, which is the exact condition criterion 2 declares disqualifying: *"An
invariant never observed to fire has never been shown to work."* Every call site that reaches
it asserts `Ok(())`; the one test with "localisation" in its name bypasses the check and calls
the scan helper directly; and the `message` module renders a hand-built `GoodsConservation`
value that the check never produced.

The check is *correct* — a verifier probe drives it to the right violation with the right
localised posting — so this is a coverage gap, not a broken invariant, and the closure is about
25 lines in a module where the corruption vocabulary is already in scope. But it must be closed
here rather than absorbed into Phase 5: Phase 5 is scheduled to change every accessor
`check_goods` reads (`total_stock`, `produced`, `consumed`, `goods_residual_units_for` all take
a `GoodId` and today ignore it), and a refactor that silently breaks the check would land with
a green CI on the exact commit that broke it.

Plan 02-05 scoped exactly four violation classes and `.planning/WINDOWS.md` records nothing
about the fifth, so this was never noticed rather than knowingly deferred — which is worth
noting on its own, because it means the phase's own "every check" self-audit did not run.

**Not gaps:** the `Household`/`Firm` balance-field claim and the process-level release halt are
both explicitly owned by ROADMAP Phase 3 criteria 7 and 6 respectively, and the liveness-gate
flip by Phase 6 criterion 7. Those cross-phase obligations were deliberately recorded as
roadmap criteria rather than left in comments, and they are.

---

_Verified: 2026-08-31T11:38:04Z_
_Verifier: Claude (gsd-verifier)_
_Mutation testing performed in an isolated `git archive HEAD` copy under the session scratchpad;
the working tree was confirmed clean (`git status --porcelain` empty) before and after._

---

## Gap Closure (2026-08-31, commit `e0ee1b4`)

The single blocking gap — `check_goods` never observed to fire — is closed.

**What was added.** A fifth `#[cfg(test)] pub(crate)` corruption on `Books`,
`corrupt_silent_stock`, being the goods analogue of `corrupt_silent_cash`. It was needed
because none of the four existing corruptions could reach the check's *balance-derived* arm
without also moving the journal arm, and a corruption that moves both cannot tell the two
apart. Two negative tests in `invariants::goods`, one per arm, each asserting the whole
`Violation` value — variant, tick, `produced`/`consumed`/`stock`, both residuals, and the
localised posting — with money-conservation, non-negativity and zero-sum asserted `Ok(())` on
the same books as controls.

**Mutation evidence, per arm.** The per-arm results are the stronger claim: neither arm can be
neutered alone without a red test, so this is not one test covering the function twice.

| Mutation in `check_goods` | Result |
|---|---|
| `if true { return Ok(()); }` | 2 failed, 164 passed — both new tests |
| `let journal_residual_units = 0;` | 1 failed — only the exchange test |
| `let delta_units = 0;` | 1 failed — only the conjured-units test |

**Independently re-verified by the orchestrator**, not accepted on report. The first mutation
was re-applied to a clean `git archive HEAD` copy outside the working tree:

```
failures:
    invariants::goods::an_exchange_whose_unit_legs_disagree_is_a_goods_leak_and_is_localised
    invariants::goods::units_conjured_outside_the_posting_path_break_the_identity_and_name_no_posting
test result: FAILED. 164 passed; 2 failed
```

Before the closure the identical mutation left all 239 tests green. The working tree was not
modified and the isolated copy was deleted afterwards.

**A regression the suite could not see.** `tests/lints.sh` guard 7j failed on the first
attempt — the E0599 compile-fail probe pins its call count to the corruption-method
declaration count, and a fifth method no probe line named could later escape the
`#[cfg(test)]` gate with check 6 still printing success. Fixed by adding the call to
`tests/lint-probes/books_cfg_test_probe.rs.txt`. Invisible to `cargo test`; caught only by the
lint wall.

**Ledger.** `.planning/WINDOWS.md` entry 23 records the omission — plan 02-05 scoped exactly
four violation classes, so the "every check" self-audit never ran. Entry 24 records the closure
and the two obligations it creates: guard 7j's probe-count contract for any sixth corruption
method, and Phase 5's rewrite of the four per-good accessors, which these two tests now guard.

Suite at closure: 242 tests, green in both profiles, clippy and fmt clean, no dependency change.
