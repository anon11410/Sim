---
phase: 02-books-journal-and-invariants
fixed_at: 2026-08-31T11:26:00Z
review_path: .planning/phases/02-books-journal-and-invariants/02-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 2: Code Review Fix Report

**Fixed at:** 2026-08-31
**Source review:** `.planning/phases/02-books-journal-and-invariants/02-REVIEW.md`
**Iteration:** 1
**Branch:** `gsd/phase-02-books-journal-and-invariants` (no worktree; sequential on the main tree)

**Summary**

- Findings in scope: 9 (1 blocker + 8 warnings)
- Fixed: 9
- Skipped: 0
- Out of scope but picked up because it is free: IN-03
- Two *sub-clauses* of the review's fix guidance were declined with reason and
  are recorded below and in `.planning/WINDOWS.md`. Neither leaves the finding
  itself open.

**Verification.** After every commit, in both build profiles:

    cargo test --locked --all-targets
    cargo test --locked --release --all-targets
    bash tests/lints.sh
    bash tests/toolchain.sh
    cargo clippy --all-targets --all-features -- -D warnings
    cargo fmt --check
    git diff --exit-code -- Cargo.toml Cargo.lock

All green at every commit. **No dependency change.** `tests/lints.sh` now runs
seven checks with ten source guards in check 7. Verification ran in the main
checkout, not an isolated worktree.

---

## Fixed Issues

### CR-01: a zero-cent transfer counted as a transaction

**Files:** `src/books.rs`, `src/invariants.rs`, `tests/ledger_props.rs`,
`.proptest-regressions/ledger_props.txt`
**Commit:** `613da60`

Closed at the boundary AND at the check, on exactly the terms `exchange` was
already closed on: `PostError::EmptyTransfer` refused in `transfer`'s compute
step, and `ZeroSumDetail::EmptyTransfer` so a posting that somehow reaches the
journal is named rather than passing.

Both test blind spots closed:

- `tests/ledger_props.rs::every_counted_transaction_moved_money` asserts the
  *counting rule* directly — every `Transfer` or `Exchange` posting carries a
  non-zero cash leg, and `transactions_this_tick()` equals the number of such
  postings. That is a per-posting claim, so it holds mid-tick and can be
  asserted with `liveness_enabled = false`, which is why turning the gate on
  (which would break the other properties for a legitimate reason the file
  documents) was not the answer.
- the transfer refusal table gains the zero case and the zero-and-self-dealing
  case, where its `exchange` sibling already enumerated three.

**Mutation-verified.** Deleting the refusal fails four independent assertions:

    books::tests::every_refusal_leaves_the_books_exactly_as_it_found_them
      panicked at src/books.rs:1916
    invariants::liveness::a_tick_whose_only_transfer_moved_nothing_still_fails_liveness
      panicked at src/invariants.rs:1009
    ledger_props::every_counted_transaction_moved_money
      posting 0 counts towards the liveness minimum but moved 0 cents out and
      0 cents in, after operation 0 (Transfer { .., cents: 0 })
    ledger_props::total_money_is_conserved_under_any_operation_sequence
      the check set reported Err(ZeroSum { .., detail: EmptyTransfer {
      debit_cents: 0, credit_cents: 0 } })

The last is the check half firing on its own, which is what proves the hole is
shut at both ends. The counterexample proptest shrank during that run is
committed, so the case is replayed on every future run.

**Behaviour note carried to `WINDOWS.md`:** the empty-leg clause runs before the
self-dealing clause (matching `exchange`), so `transfer(a, a, 0)` now reports
`EmptyTransfer` where it reported `SelfDealing`.

---

### WR-02: the residual property that asserted `0 == 0`

**Files:** `src/books.rs`, `tests/ledger_props.rs`
**Commit:** `4b13eaf`

The claim with teeth is not that the two sources AGREE — on the honest path they
agree at zero, and on `corrupt_recorded_cash` they agree because that corruption
writes both. It is that they can DISAGREE: the journal residual is what the
postings say, not a restatement of the balances.
`books::tests::the_two_residual_sources_move_apart_when_only_one_of_them_is_told`
appends postings whose legs disagree while touching no balance and no stock, and
asserts the posting-derived quantity moves by exactly what the legs say while
the balance-derived one does not move at all. Both quantities are covered (an
over-credited transfer, +1 cent; an exchange whose unit legs disagree, +2 units),
and it closes with `corrupt_recorded_cash` to pin the complementary direction.

**Mutation-verified** against the exact single-source collapse the property's own
doc comment named:

    let cash_delta = self.total_money().cents()
        - self.opening_stock.cents()
        - self.cash_residual_cents;

    src/books.rs the_two_residual_sources_move_apart_when_only_one_of_them_is_told
      assertion `left == right` failed: the postings say a cent was created,
      and the residual is read off the legs of the posting rather than off the
      balances
        left: 0
       right: 1

    tests/ledger_props.rs: 8 passed; 0 failed

That split is the finding, executed. The property's doc comment now records that
it is structurally `0 == 0` and names where the version with teeth lives.

---

### WR-03: check 4b's regex anchored on the first argument

**File:** `tests/lints.sh`
**Commit:** `38b2808`

| line | old | new |
|---|---|---|
| `#![allow(clippy::disallowed_types)]` | yes | yes |
| `#[allow(dead_code, clippy::disallowed_methods)]` | **NO** | yes |
| `#[cfg_attr(test, allow(clippy::disallowed_methods))]` | **NO** | yes |
| `#[expect(warnings)]` | yes | yes |

Given `assert_fires`/`assert_ignores` proofs, which required lifting those two
helpers above their new call site — their original position, below check 4b, is
recorded in section 7's comment as the reason 4b never got a proof.

**Verified end to end**, not only against the fixture. Injecting each missed
spelling into `src/numeric.rs`:

    FAIL: a file carries a lint exemption for a determinism ban — found:
      src/numeric.rs:75:#[allow(dead_code, clippy::disallowed_methods)]
    FAIL: a file carries a lint exemption for a determinism ban — found:
      src/numeric.rs:75:#[cfg_attr(test, allow(clippy::disallowed_methods))]

---

### WR-01: the saturating endowment gate

**File:** `src/books.rs`
**Commit:** `f23a3e5`

The money side now does what the goods side already did: computes the endowment
in closed form with `checked_mul`/`checked_add` and reports a failure to
represent it as `BooksError::EndowmentOutOfRange`. The closed form is checked
rather than the running sum because an intermediate can overflow while the final
total fits — `households * liquidity` alone is the reviewer's scenario. The
residual gate is kept as the recorder's independent statement, and its
`unwrap_or(i64::MAX)` (a fabricated total in an operator-facing message) is gone.

**Mutation-verified twice** — restoring `saturating_add` in the closed form, and
accepting the overflow instead of refusing it. Both fail
`a_mixed_sign_endowment_that_saturates_the_running_sum_is_refused_not_accepted`,
while `a_mixed_sign_endowment_that_does_sum_to_the_stock_still_opens_the_books`
passes throughout — which is what says the refusal is on representability and
not on sign.

---

### WR-04: check 6 named one of four corruption methods

**Files:** `tests/lint-probes/books_cfg_test_probe.rs.txt`, `tests/lints.sh`
**Commit:** `1936494`

Probe calls all four, one per line; `cargo build --tests` now emits four E0599
diagnostics. Guard 7i counts every `corrupt_*` declaration against those inside a
`#[cfg(test)]` block — the case the probe structurally cannot cover, because it
cannot name a fifth method nobody has written. Guard 7j pins the probe's call
count to the declaration count.

**Both watched firing on the real tree.** Adding a fifth `pub fn corrupt_stock`
to the non-test `impl Books`: checks 1–6 all pass, check 6 prints its success
line, and then guard 7i fails. That is the finding executed.

---

### WR-05: guard 7h did not search where half the message is rendered

**Files:** `tests/lints.sh`, `clippy.toml`
**Commit:** `b3dd5e4`

`src/books.rs`'s production half is now in guard 7h's search set, because every
`Violation` carrying a posting renders it through `impl Display for Posting`,
which lives there. The reviewer's failure scenario, executed: adding
`std::process::id()` to that impl produces **zero** clippy diagnostics and passes
the runtime TICK-06 message test (which rejects only `/` and `\`), and now fails
guard 7h.

**Sub-clause declined:** adding `std::process::id` to `clippy.toml`'s
`disallowed-methods`. It breaks check 1 — `error: use of a disallowed method
std::process::id --> tests/config_strict.rs:275`, and
`tests/tracer_end_to_end.rs:21` the same, both building a unique temporary path —
and check 4b forbids the `#[allow(...)]` that would silence it. Same class of
exclusion `clippy.toml` already documents for `RefCell` and `Arc`, recorded in
the same place and in the same terms. No `BANNEDCALL` line added, so check 3
still reads 60 against 60. **The finding is still closed:** the source grep is
not a weaker substitute here, it is the only instrument that works, as the
injection above demonstrated.

---

### WR-06: guard 7d's two-file scope

**File:** `tests/lints.sh`
**Commit:** `8fb0667`

Widened over every tracked `src/*.rs` with `src/rng.rs` carved out by name,
exactly as guard 7c carves it out for `RefCell`. The two-file clause is kept and
still searches raw files; the wider clause strips line comments, so
`src/numeric.rs` may keep explaining in prose that its finiteness check is
deliberately *not* a `debug_assert!`.

**Watched firing on the exact Phase 3 shape:**

    FAIL: guard 7d: the debug-only assertion vocabulary appears under src/ in
    [src/rng.rs src/world.rs ] — expected exactly src/rng.rs

**Sub-clause declined:** recording the obligation in `.planning/ROADMAP.md`.
This pass is barred from editing that file. The obligation is **discharged
rather than deferred** — the widened scope catches a Phase 3 `src/world.rs` on
the commit that adds it, with no promise for a future reader to keep. This is
the stronger of the two instruments, so nothing is lost.

---

### WR-07: "already shaped" for a goods table

**Files:** `src/invariants.rs`, `src/books.rs`
**Commit:** `95bf8aa`

Both comments now list the work Phase 5 inherits — per-good stock vectors,
accessor bodies that use their argument, per-good `produced`/`consumed`, a
per-good residual — and each accessor that ignores its argument says so at its
own definition. `check_goods` reads the residual **inside** the loop through
`Books::goods_residual_units_for(good)`; in v1 that is the same number every time
round, so it is not a behaviour change, but it is where the broadcast is
prevented. `goods_residual_units` stays as the whole-books quantity.

---

### WR-08: `UnitLegsDiffer` contradicting its own numbers

**File:** `src/invariants.rs`
**Commit:** `54f6df2`

`ZeroSumDetail::UnitsInTheWrongDirection { kind, units_out, units_in }`, returned
from the `Produce` and `Consume` arms. The `Display` match on kind is exhaustive
rather than defaulted. `UnitLegsDiffer` now means only what it says: an exchange
whose two legs must be equal and are not.

The malformed-shape table gains the `Consume` mirror, which it did not cover at
all, and a loop asserting on the **rendered** form that no shape whose two legs
are equal is described as a disagreement — the assertion with teeth here, since
the finding was that the value was right and the prose was wrong.

**Mutation-verified.** Restoring the old wording:

    invariants::zero_sum::each_malformed_shape_is_named_exactly FAILED
    a shape whose two legs are equal must not be reported as a disagreement:
    the unit legs disagree: 4 units left but 4 arrived

---

### IN-03 (out of scope, taken because free)

**File:** `src/books.rs`
**Commit:** `e79a11a`

Comment only. States why `record` saturates where `produce`/`consume` use bare
arithmetic that aborts: a unit count and a balance are quantities the model
claims exist; the residuals are diagnostic quantities a broken-invariant report
is made of. Taken because they are the same lines WR-01 and WR-02 are about.

---

## Info findings NOT addressed

Out of scope for this pass and left open deliberately: **IN-01** (`check_non_negative`
silently skips an unresolvable address — unreachable today, worth a loud
`unreachable!`), **IN-02** (`produce`/`consume` accept zero units and record a
no-op posting — not a liveness bypass, but a no-op line in a byte-compared
Phase 3 log), **IN-04** (`Params::validate` bounds no endowment key — WR-01 closed
the sharp form at the constructor; the config-layer bound is still worth adding),
**IN-05** (`well_formed`'s `Endow` rule permits a negative credit and cash-plus-units
on one posting — unreachable until a runtime endowment exists), **IN-06** (check 4a
has no `assert_fires` and misses a rustfmt-wrapped alias; the TICK-06 message test
rejects only `/` and `\`). IN-06(a) is now cheap — `assert_fires`/`assert_ignores`
sit above check 4 after the WR-03 commit.

---

_Fixed: 2026-08-31_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
