---
phase: 02-books-journal-and-invariants
reviewed: 2026-08-31T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - src/books.rs
  - src/invariants.rs
  - src/ids.rs
  - src/config.rs
  - src/lib.rs
  - tests/invariant_halt.rs
  - tests/ledger_atomicity.rs
  - tests/ledger_props.rs
  - tests/lints.sh
  - tests/lint-probes/books_borrow_probe.rs.txt
  - tests/lint-probes/books_cfg_test_probe.rs.txt
  - clippy.toml
  - config/baseline.toml
  - config/PROVENANCE.md
findings:
  critical: 1
  warning: 8
  info: 6
  total: 15
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-08-31
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

The ledger's compute-then-commit discipline holds. I traced every write in
`transfer`, `produce`, `consume` and `exchange`: in all four, every fallible
step (sign checks, self-dealing, `resolve`, `checked_sub`/`checked_add`,
overdraft, `ShortStock`, `UnknownGood`, and the bare-integer stock/produced
additions that abort under `overflow-checks`) precedes the first
`write_cash`/`write_stock`, and the commit block contains only assignments plus
one `record` call. No `?`, no early return and no panic sits between two writes.
The localisation scans are forward `Iterator::find` calls — not bisections — and
the direction and early-exit are correct. `Books` is `Clone`-only, has no
callback surface, no interior mutability, and `#[forbid(unsafe_code)]` closes
the raw-pointer route. Distinct `Account` values always map to distinct
`AccountSlot`s, so no operation can alias one slot onto both legs.

The defects are elsewhere. One is a real liveness bypass of exactly the shape
the phase already found once and closed for `exchange` but left open for
`transfer`. The rest are checks and guards whose stated claim is broader than
what they actually measure: a construction-time conservation gate computed with
saturating arithmetic, a "two independent sources" residual clause that no
public-API sequence can move off zero (and the property test that consequently
asserts `0 == 0`), and four grep/probe guards in `tests/lints.sh` whose search
set or pattern does not cover the hazard they name — one of which I verified
does not match the most idiomatic spelling of the thing it forbids.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: A zero-cent `transfer` counts as a transaction, so the liveness check passes on a tick where no money changed hands

**File:** `src/books.rs:704-751` (`Books::transfer`), `src/books.rs:1227-1229`
(`Books::record`), `src/invariants.rs:677-691` (`well_formed`, `Transfer` arm)

**Issue:**
`Books::exchange` explicitly refuses an empty leg, and `PostError::EmptyExchange`
says why in as many words (`src/books.rs:324-337`):

> it would still count towards the liveness minimum (LEDG-08) — the degenerate
> "a transaction happened" pass that check exists to close.

`Books::transfer` has no such guard. It rejects `amount.cents() < 0` and
`from == to`, and then accepts `Money::from_cents(0)`. The commit step runs,
`record` is reached with `kind: PostingKind::Transfer`, and
`transactions_this_tick` is incremented (`src/books.rs:1227`). `check_liveness`
then sees `counted >= 1` and returns `Ok(())` for a tick in which not one cent
moved — the exact degenerate pass LEDG-08 exists to close, and the exact
condition `src/books.rs:1161-1166` claims the counting rule prevents:

> That counting rule is what makes LEDG-08 mean "money changed hands" rather
> than "something happened".

The structural check does not catch it either. `well_formed`'s `Transfer` arm
checks `two_party`, then units-on-a-cash-only-posting, then
`debit_cents != credit_cents`. A posting with `debit_cents == credit_cents == 0`
satisfies all three, so there is no `ZeroSumDetail` for an empty transfer the way
there is `ZeroSumDetail::EmptyExchange` for an empty exchange. For `exchange`
both the operation boundary *and* the check close the hole; for `transfer`
neither does.

**Failure scenario:**
The gate ships `false` today, but `config/baseline.toml`'s own comment records
that ROADMAP Phase 6 turns it on "in the commit that first makes wages move
money". Phase 6 introduces partial payroll payment — the case
`src/books.rs:96-101` is written about. A firm with no cash paying a wage of
zero, or a dividend of zero under `dividend_buffer_ppm`, calls
`transfer(firm, household, Money::ZERO)`. It returns `Ok(Money::ZERO)`,
increments the transaction count, and a tick in which the whole economy paid
nothing and traded nothing passes liveness. The check reports green forever
after, and the "money conserves because nothing ever moved" state it exists to
detect becomes unreachable by it.

There is also a property-test blind spot that hides this: `any_cents()` in
`tests/ledger_props.rs:206` draws `Just(0i64)` with weight 3 and `any_op()` gives
`Op::Transfer` weight 6, so zero-cent transfers are generated on nearly every
case — but `small_params()` sets `liveness_enabled = false`
(`tests/ledger_props.rs:95`), so nothing observes the count. And
`every_refusal_leaves_the_books_exactly_as_it_found_them`
(`src/books.rs:1832-1876`) enumerates five refusals and does not include a
zero-amount transfer, while its `exchange` sibling
(`src/books.rs:2228-2305`) enumerates three separate `EmptyExchange` cases.

**Fix:** refuse an empty transfer at the boundary on the same terms as an empty
exchange, and give the structural check a shape to report if one ever reaches the
journal.

```rust
// src/books.rs — PostError
/// A transfer of nothing. Refused for the same reason
/// [`PostError::EmptyExchange`] is: it moves no money and would still count
/// towards the liveness minimum (LEDG-08).
#[error("a transfer of {amount_cents} cents moves nothing")]
EmptyTransfer { amount_cents: i64 },

// src/books.rs — Books::transfer, in the compute step, after the sign check
if amount.cents() == 0 {
    return Err(PostError::EmptyTransfer { amount_cents: 0 });
}

// src/invariants.rs — ZeroSumDetail
/// A transfer with no cash on either leg. Nothing changed hands, and it
/// would count towards the liveness minimum.
EmptyTransfer { debit_cents: i64, credit_cents: i64 },

// src/invariants.rs — well_formed, PostingKind::Transfer arm, after the
// debit_cents != credit_cents clause
if posting.debit_cents == 0 {
    return Err(ZeroSumDetail::EmptyTransfer {
        debit_cents: posting.debit_cents,
        credit_cents: posting.credit_cents,
    });
}
```

Then add the case to the refusal table in
`src/books.rs::every_refusal_leaves_the_books_exactly_as_it_found_them` and to
`detail_position`/`every_detail` in `invariants::message` (both are exhaustive
matches, so the compiler will name the lines).

## Warnings

### WR-01: The construction-time conservation gate is computed with saturating arithmetic, and nothing bounds the cash endowment

**File:** `src/books.rs:1220` (`record`), `src/books.rs:658-667`
(`Books::new`), `src/config.rs:139-290` (`Params::validate`)

**Issue:** `Books::new`'s only proof that "the books do not begin the run
already broken" on the money side is `books.cash_residual_cents != 0`. That
residual is accumulated in `record` with
`self.cash_residual_cents.saturating_add(cash_delta)`. Saturation is silent: once
the running sum clamps at `i64::MAX`, subsequent additions are discarded and the
gate is reading a number that is no longer the endowment.

The goods side of the same constructor does this correctly —
`i64::from(firm_slots).checked_mul(units_per_firm)` with a dedicated
`BooksError::InitialInventoryOutOfRange` (`src/books.rs:570-576`). There is no
equivalent bound on the cash endowment, and `Params::validate` bounds
`money.total_money_cents` but neither `household.initial_liquidity_cents` nor
`firm.initial_liquidity_cents`. `Money::from_cents` is infallible by design.

**Failure scenario:** an operator file with a very large positive household
liquidity and a compensating negative firm liquidity (a mixed-sign endowment is a
supported shape — `invariants::non_negative::households_endowed_negative` builds
exactly one). The true endowment sum overflows `i64` during the household loop,
`saturating_add` clamps at `i64::MAX`, the negative firm legs bring the clamped
value back to exactly zero, and `Books::new` returns `Ok`. The first
`check_money` then calls `books.total_money()`, whose `Sum for Money` impl folds
with the *panicking* `Add` (`src/money.rs:222`) — the process aborts with
`Money overflow on add` from inside the invariant phase, which
`src/invariants.rs:477-479` states must never itself be the thing that fails.

**Fix:** accumulate the construction residual with `checked_add` and report the
overflow as a typed `BooksError`, and/or bound the two liquidity keys in
`Params::validate` the way every other consumer-imposed bound is:

```rust
// src/books.rs — Books::new, replacing the implicit reliance on record's
// saturating_add for the endowment gate
let endowed_cents = i64::from(params.sim.households)
    .checked_mul(params.household.initial_liquidity_cents)
    .and_then(|h| {
        i64::from(firm_slots)
            .checked_mul(params.firm.initial_liquidity_cents)
            .and_then(|f| h.checked_add(f))
    })
    .ok_or(BooksError::EndowmentDoesNotSumToStock {
        endowed_cents: i64::MAX,
        opening_cents,
    })?;
if endowed_cents != opening_cents {
    return Err(BooksError::EndowmentDoesNotSumToStock { endowed_cents, opening_cents });
}
```

### WR-02: The journal-residual clause of `check_money`/`check_goods` cannot move off zero from any public path, and the property that claims to prove two-source independence is therefore `0 == 0`

**File:** `src/books.rs:1195-1221` (`record`), `src/invariants.rs:473-494`,
`src/invariants.rs:516-547`, `tests/ledger_props.rs:664-704`

**Issue:** `record` derives `cash_delta` as
`draft.credit_cents.saturating_sub(draft.debit_cents)`. Every public operation
constructs the posting with **one** value on both cash legs — `transfer` writes
`debit_cents: amount.cents(), credit_cents: amount.cents()`
(`src/books.rs:742-743`), `exchange` the same (`src/books.rs:956-957`),
`produce`/`consume` write `0/0`. `goods_delta` has the same shape: `produce`
supplies `units_in: units` and separately advances `produced` from the same
`units` argument, so `produced_added - consumed_added + units_out - units_in`
is structurally zero; `exchange` writes `units_out: units, units_in: units`.

So after `Books::new` clears the journal, `cash_residual_cents` and
`goods_residual_units` are *invariantly* zero for every sequence an ordinary
caller can produce. The second conjunct of `check_money`'s
`delta_cents == 0 && journal_residual_cents == 0` has no production failure mode;
only the `#[cfg(test)]` corruption vocabulary can move it. That is defensible for
the check itself (the unit tests in `invariants::negative` do exercise it), but
it makes the "two genuinely separate derivations" claim in
`src/books.rs:1010-1015` and `src/invariants.rs:467-472` weaker than stated: the
journal side is not an independent observation of what was written, it is a
restatement of the same argument the balance write used.

The consequence is a test blind spot.
`tests/ledger_props.rs::posting_residuals_agree_with_the_balance_derived_quantities`
is documented as **"the property that states the design directly"** and as the
thing that would catch "calling `total_money()` inside `record`". It cannot.
Both sides of both comparisons are constantly zero, so the assertion is
`0 == 0`; replacing `record`'s residual arithmetic with
`self.cash_residual_cents = self.total_money().cents() - self.opening_stock.cents()`
— the exact single-source collapse the doc comment names — leaves the property
green. This is the same failure shape the phase already found and documented for
`ending_a_tick_leaves_the_residuals_and_the_balances_untouched`
(`tests/ledger_props.rs:719-730`), but here it is undocumented and the doc
comment actively claims the opposite.

**Fix:** either (a) add a unit-test counterpart in `src/books.rs` that seeds a
non-zero residual with `corrupt_recorded_cash` / `corrupt_appended_posting` and
asserts the two sources still agree — mirroring
`ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched`
(`src/books.rs:1909`), which is the pattern this project already uses for exactly
this problem — or (b) at minimum correct the doc comment to record that the
integration-level property is structurally `0 == 0` and name where the version
with teeth lives, as the sibling property already does.

### WR-03: `tests/lints.sh` check 4b does not match the most idiomatic spelling of the exemption it forbids, and is the one guard never proved to fire

**File:** `tests/lints.sh:235-237`

**Issue:** the pattern is

```
'#!?\[(allow|expect)\((warnings|clippy::(all|disallowed_types|disallowed_methods))'
```

which requires the banned lint name to be the **first** argument of the
`allow(...)`. Verified by execution against three fixtures:

| line | matched |
|---|---|
| `#![allow(clippy::disallowed_types)]` | yes |
| `#[allow(dead_code, clippy::disallowed_methods)]` | **no** |
| `#[cfg_attr(test, allow(clippy::disallowed_methods))]` | **no** |

The comment above it states the guard's purpose precisely — "a single
`#![allow(clippy::disallowed_methods)]` at the top of a module disables all 68
float and clock bans at once" — and both missed spellings do exactly that. Check
4b is also the only grep guard in the file with no `assert_fires` proof; checks
4a–4d predate check 7's stated discipline, which the header itself describes as
"a grep pattern with a typo matches nothing and is indistinguishable from a clean
tree".

**Failure scenario:** a developer adds `#[allow(dead_code, clippy::disallowed_methods)]`
to a module to silence one unrelated warning. Every float and clock ban is
disabled in that module, `cargo clippy -- -D warnings` passes, check 4b reports
nothing, and a `f64::powf` on the behaviour path ships.

**Fix:** allow the banned name at any argument position, and put the guard under
the same `assert_fires`/`assert_ignores` discipline as guards 7a–7h:

```bash
EXEMPTION_PATTERN='#!?\[(cfg_attr\([^)]*,[[:space:]]*)?(allow|expect)\([^)]*(warnings|clippy::(all|disallowed_types|disallowed_methods))'
assert_fires 4b "$EXEMPTION_PATTERN" 4 '#![allow(clippy::disallowed_types)]
#[allow(dead_code, clippy::disallowed_methods)]
#[cfg_attr(test, allow(clippy::disallowed_methods))]
#[expect(warnings)]'
assert_ignores 4b "$EXEMPTION_PATTERN" '#[allow(dead_code)]
#[derive(Debug, Clone)]'
assert_absent "a file carries a lint exemption for a determinism ban" \
    -En "$EXEMPTION_PATTERN" -- "${RUST_SOURCES[@]}"
```

### WR-04: Check 6 names one of the four corruption methods, so three of them can leave the test configuration with the check still green

**File:** `tests/lints.sh:288-314`, `tests/lint-probes/books_cfg_test_probe.rs.txt:44`,
`src/books.rs:1339-1470`

**Issue:** the probe calls exactly one method, `corrupt_silent_cash`, and check 6
asserts the build fails with `E0599`. The `#[cfg(test)] impl Books` block holds
four: `corrupt_recorded_cash`, `corrupt_silent_cash`,
`corrupt_conserving_deficit`, `corrupt_appended_posting`. Check 6 says nothing
about the other three. There is also no source guard asserting that every
`corrupt_*` method sits inside a `#[cfg(test)]` block — guard 7d bans the
`debug_assert` vocabulary in these files but nothing pins the gate on the fault
injection.

**Failure scenario:** someone adds a fifth fault-injection method, or moves
`corrupt_appended_posting` out of the `#[cfg(test)]` block and makes it `pub`, to
reach it from an integration test. The probe still names `corrupt_silent_cash`,
which still does not resolve, so the build still fails with `E0599` and check 6
still prints "the fault-injection vocabulary is refused from tests/ with E0599"
— while a method that writes state the public API cannot reach has shipped in the
library.

**Fix:** call every method in the probe (each on its own line so any one
resolving turns the build green), and add a source guard that pins the gate:

```rust
// tests/lint-probes/books_cfg_test_probe.rs.txt
books.corrupt_silent_cash(Account::Household(HouseholdId(0)), -1);
books.corrupt_recorded_cash(household, firm, 100, -1);
books.corrupt_conserving_deficit(household, firm, 100);
books.corrupt_appended_posting(draft);
```

```bash
# tests/lints.sh, guard 7i: every corruption method is behind the test gate.
CORRUPT_COUNT=$(printf '%s\n' "$BOOKS_CODE" | grep -cE 'fn[[:space:]]+corrupt_[A-Za-z0-9_]+')
GATED_COUNT=$(awk '/^#\[cfg\(test\)\]/{g=1} /^}/{g=0} g && /fn[[:space:]]+corrupt_/{n++} END{print n+0}' "$BOOKS_SRC")
[ "$CORRUPT_COUNT" -eq "$GATED_COUNT" ] || fail "guard 7i: a corrupt_* method sits outside a #[cfg(test)] block"
```

### WR-05: Guard 7h's search set excludes `src/books.rs`, where half of every halt message is rendered

**File:** `tests/lints.sh:611-634`, `src/books.rs:236-262`

**Issue:** guard 7h asserts that no path, clock or process type is named in the
production half of `src/invariants.rs`. But every `Violation` variant that
carries a posting interpolates it through `render_posting`
(`src/invariants.rs:329-337`), which formats it with `impl Display for Posting`
— and that impl lives in `src/books.rs`, which guard 7h never searches. The
guard's own comment claims it is "the source half" of the TICK-06 rule; it covers
only the outer half of the string.

**Failure scenario:** `std::process::id()` is added to `Posting`'s `Display` to
tag concurrent runs. It is not in `clippy.toml`'s `disallowed-methods` (only
`SystemTime::now` and `Instant::now` are). It renders as digits, so
`invariants::message::every_variant_is_exercised_and_no_message_carries_a_path`,
which only rejects `/` and `\`, still passes. Guard 7h never looks at
`src/books.rs`. A process id reaches stderr on every halt, in violation of
TICK-06. The one thing that would catch it is the pinned exact string in
`src/books.rs:1824-1827` — which the same commit would mechanically update.

**Fix:** add `src/books.rs`'s production half to guard 7h's search set:

```bash
BOOKS_PRODUCTION=$(production_source "$BOOKS_SRC")
[ -n "$BOOKS_PRODUCTION" ] || fail "guard 7h: the books' production half is empty"
assert_absent_in "guard 7h: the ledger names a path, clock or process type ..." \
    "$ENVIRONMENT_PATTERN" "$INVARIANTS_PRODUCTION
$BOOKS_PRODUCTION"
```

and add `std::process::id` to `clippy.toml`'s `disallowed-methods` (with the
corresponding `// BANNEDCALL` line in `tests/lint-probes/float_ban_probe.rs.txt`,
so check 3's count stays balanced).

### WR-06: Guard 7d's scope is two files, and unlike guard 7f it records no inherited obligation for the phase that adds the tick loop

**File:** `tests/lints.sh:496-517`

**Issue:** guard 7d asserts `debug_assert` (which correctly also catches
`debug_assertions`, verified) is absent from `$BOOKS_SRC` and `$INVARIANTS_SRC`,
and its permitted fixture correctly distinguishes `#[cfg(test)]` from
`#[cfg(debug_assertions)]`. The predicate distinction is right. The *scope* is
not: LEDG-10's claim is about the invariant path, and Phase 3 puts the tick loop
that calls `CheckSet::run` in a new file (`src/world.rs` per
`src/books.rs:6-7`). Guard 7f faced the identical problem and handled it by
recording the extension as **ROADMAP Phase 3 success criterion 7**
(`tests/lints.sh:566-570`). Guard 7d records nothing.

**Failure scenario:** Phase 3's `src/world.rs` wraps the check-set call in
`debug_assert!` or `#[cfg(debug_assertions)]` for a "fast release run". Because
`overflow-checks = true` does not enable `debug_assertions`, the invariant phase
vanishes from the binary that produces the actual run. Guard 7d is silent
because `src/world.rs` is not in its two-file list.

**Fix:** search every tracked source under `src/` and carve out the one
legitimate site by file, exactly as guard 7c does for `RefCell`:

```bash
set +e
DEBUG_GATE_FILES=$(grep -rlE "$COMPILED_OUT_PATTERN" -- "${SRC_FILES[@]}" | sort | tr '\n' ' ')
GREP_STATUS=$?
set -e
[ "$GREP_STATUS" -le 1 ] || fail "guard 7d: could not search for the debug-only vocabulary"
if [ "$DEBUG_GATE_FILES" != "src/rng.rs " ]; then
    fail "guard 7d: the debug-only assertion vocabulary appears under src/ in [$DEBUG_GATE_FILES] — expected exactly src/rng.rs (the debug-only sub-stream re-entry guard). An invariant a build profile can compile out is not an invariant of the binary that produced a run (LEDG-10)"
fi
```

and record the obligation in the ROADMAP the way guard 7f's is.

### WR-07: `check_goods` is documented as "already shaped" for a multi-good table, and it is not

**File:** `src/invariants.rs:516-547`, `src/books.rs:1073-1099`

**Issue:** `check_goods` reads `journal_residual_units` **once, outside the
loop** (`src/invariants.rs:519`) and compares that single global number against
every good in turn. On the books side, `total_stock`, `produced` and `consumed`
all check `Books::carries(good)` and then ignore the argument entirely,
returning the crate-wide totals (`src/books.rs:1073-1099`). The comment at
`src/invariants.rs:517-518` says "the loop below is already shaped for that", and
`src/books.rs:126-131` says "no call site moves when it happens". Both are false:
Phase 5 has to change the accessors' bodies, the residual's arity, and the
loop's use of it.

**Failure scenario:** Phase 5 widens `GOODS` to two entries and adds per-good
`Vec`s. Until the accessors are also rewritten, `total_stock(GoodId(1))` returns
the sum over *both* goods, `produced(GoodId(1))` returns the global produced
count, and the identity `produced − consumed − Σstock` is nonsense for both
goods while looking well typed. Even after the accessors are fixed, a single
shared `journal_residual_units` makes `check_goods` report `GOODS[0]` as the
offending good whenever the residual is non-zero, regardless of which good's
posting broke it — sending a debugger to the wrong column, which is the exact
failure `src/invariants.rs:512-515` says the check was kept unfactored to avoid.

**Fix:** for this phase, correct the two comments so they state the work Phase 5
actually inherits rather than promising none. For the check itself, make the
residual per-good at the point it is read, so the loop body cannot silently
become a broadcast:

```rust
for &good in books.goods() {
    let journal_residual_units = books.goods_residual_units_for(good);
    // ... unchanged
}
```

### WR-08: `ZeroSumDetail::UnitLegsDiffer` renders a self-contradicting message for a production or consumption with both unit legs set

**File:** `src/invariants.rs:713-732`, `src/invariants.rs:281-287`

**Issue:** the `Produce` arm reports
`UnitLegsDiffer { units_out, units_in }` when `units_out != 0`, and the
`Consume` arm reports the same variant when `units_in != 0`. Neither compares
the two legs. When they are equal — which is the case the project's own test
exercises (`src/invariants.rs`, `each_malformed_shape_is_named_exactly`, case
"a production that also releases units", `units_out: 4, units_in: 4`) — the
rendered message is:

> the unit legs disagree: 4 units left but 4 arrived

The message states the opposite of the numbers it carries. The real fault is
"a production also released units", which is a different shape from "the legs
disagree", and `ZeroSumDetail` has no variant for it.

**Failure scenario:** a halt fires on a Phase-5 production path that mistakenly
sets both legs. The operator reads a message asserting a disagreement between two
identical numbers, concludes the invariant module is broken rather than the
production rule, and looks in the wrong file. The whole point of
`ZeroSumDetail` carrying "exactly what disagreed" (`src/invariants.rs:226-237`)
is defeated for two of the five kinds.

**Fix:** add a dedicated variant with an honest message.

```rust
/// A one-party goods posting moved units in the direction its kind does not
/// permit: a production that also released units, or a consumption that also
/// received them.
UnitsInTheWrongDirection { kind: PostingKind, units_out: i64, units_in: i64 },
```

and return it from the `Produce` and `Consume` arms instead of
`UnitLegsDiffer`. `detail_position`/`every_detail` in `invariants::message` are
exhaustive matches, so the compiler will name the lines that need the new case.

## Info

### IN-01: `check_non_negative` silently skips an account whose address does not resolve

**File:** `src/invariants.rs:579-603`

**Issue:** both arms are `if let Some(...) = ... && ... < 0`. If `cash_of` or
`stock_of` ever returned `None` for an address that `Books::accounts()` yielded,
the account is skipped in silence and the check reports `Ok(())`. Today
`accounts()` yields the current identity for every slot, so `resolve` always
succeeds and the `None` branch is unreachable — but a check that answers "clean"
for an account it could not read is the vacuous-pass shape this module
otherwise works hard to avoid (see the headcount note at
`src/invariants.rs:566-572`).

**Fix:** make the unreachable case loud rather than silent —
`let Some(cash) = books.cash_of(account) else { unreachable!("accounts() yields only live addresses") };`
— or route it to a `Violation` so a future `accounts()` change surfaces as a
failure rather than as a shrinking check.

### IN-02: `produce` and `consume` accept zero units and record a no-op posting

**File:** `src/books.rs:769-804`, `src/books.rs:820-859`

**Issue:** `produce(who, good, 0)` and `consume(who, good, 0)` pass the sign
check, pass `carries`, resolve, and record a posting with every leg zero.
`well_formed` accepts it. These are not transactions, so they do not affect
liveness (unlike CR-01), but they consume a sequence number and add a line to a
journal that Phase 3 writes into `events.jsonl` — a no-op event in a
byte-compared log.

**Fix:** refuse zero units at the boundary with the `EmptyExchange`-shaped
reasoning, or record nothing and return `Ok(0)` early. The former is more
consistent with the rest of the module.

### IN-03: `record` uses saturating residual arithmetic while the goods operations use aborting bare arithmetic

**File:** `src/books.rs:1196-1221` vs `src/books.rs:784-785`, `src/books.rs:839-840`

**Issue:** `produce`/`consume` deliberately use bare `+`/`-` so an unrepresentable
count aborts before any write (T-02-17, `src/books.rs:779-783`). `record` does
the opposite for the two residuals — `saturating_add`/`saturating_sub` — with no
comment explaining the difference. A saturated residual is a wrong number that
`check_money` then reports as `journal_residual_cents` and that
`first_breaking_cash_posting` scans against.

**Fix:** either state the reason in `record` (the residual is a diagnostic
quantity, so it saturates rather than aborting — the same sentence
`check_money` already carries at `src/invariants.rs:477-479`), or make it
`checked_*` and surface the overflow. Consistency matters more than which one is
chosen; today a reader cannot tell which rule applies where.

### IN-04: `Params::validate` bounds no endowment key

**File:** `src/config.rs:139-290`, `config/baseline.toml`

**Issue:** `validate` is documented as the place values the model cannot run on
are rejected "at run start because every alternative place to notice is thousands
of ticks too late". It bounds ticks, populations, the money pile, the one float,
the exponent, the probabilities and the sample widths — but not
`household.initial_liquidity_cents`, `firm.initial_liquidity_cents` or
`firm.initial_inventory_units`. A negative liquidity is accepted, opens books
that hold negative balances, and is reported by `check_non_negative` at tick 0
rather than by name at config load. (`initial_inventory_units` is bounded, but by
`Books::new`, not by `validate`.) See also WR-01, which is the sharper form of
this gap.

**Fix:** add non-negativity bounds for the three keys in `validate`, naming
`check_non_negative` and `Books::new` as the consumers that impose them. The
unit tests that rely on a negative endowment
(`invariants::non_negative::households_endowed_negative`) mutate `Params` after
`load`, so they are unaffected.

### IN-05: `well_formed`'s `Endow` rule permits a negative credit and permits cash and units on one posting

**File:** `src/invariants.rs:733-741`, `src/books.rs:602-655`

**Issue:** the `Endow` arm checks only `one_party` and
`debit_cents != 0 || units_out != 0`. It accepts a negative `credit_cents` or
`units_in`, and it accepts a posting carrying both cash and units — neither of
which the constructor produces (it records them as two separate postings), so the
rule is looser than the shape it documents at `src/invariants.rs:669-670`.
Unreachable today because `Books::new` clears the journal before tick 0, but
Phase 3 or a later phase adding a runtime endowment inherits the loose rule.

**Fix:** tighten the arm to match the constructor's actual shapes —
`credit_cents >= 0 && units_in >= 0` and `credit_cents == 0 || units_in == 0`.

### IN-06: Two guards with narrower coverage than their comments claim

**File:** `tests/lints.sh:216-217` (4a), `src/invariants.rs`
(`every_variant_is_exercised_and_no_message_carries_a_path`)

**Issue:** (a) check 4a's alias pattern requires `type X = ... HashMap ...` on
one physical line, so a rustfmt-wrapped alias
(`type Table =\n    HashMap<u32, u32>;`) escapes it. Like 4b it has no
`assert_fires` proof. (b) The runtime TICK-06 message test rejects only `/` and
`\`; a host name, a Windows drive letter, or a wall-clock reading rendered as an
epoch integer would all pass. The comment describes the rule as "no path, host
name, wall-clock reading or process id".

**Fix:** for (a), put the pattern under `assert_fires`/`assert_ignores` and add a
wrapped-alias fixture. For (b), either narrow the comment to what is asserted, or
assert the stronger claim — every message character is drawn from
`[0-9a-z:#>_ .,;()-]` plus the fixed prose, which is checkable and would catch a
host name.

---

_Reviewed: 2026-08-31_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
