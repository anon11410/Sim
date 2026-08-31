//! Panic-atomicity: the fourth leg of LEDG-02, as a test that passes standing
//! next to the mutant that fails it.
//!
//! The ROADMAP phrases LEDG-02's criterion as "a test observing the books
//! mid-transaction is impossible to write". That sentence can be asserted but
//! never verified, so plan 02-06 replaces it with four checkable facts. Three of
//! them are negative — a compile-fail probe and two source guards in
//! `tests/lints.sh`. This file is the fourth, and it is the only one that is
//! **positive and executable**: a transfer that cannot complete leaves the books
//! exactly as it found them, and it says so by returning rather than by
//! unwinding between two writes.
//!
//! **Why the mutant is here, and why the file would be worthless without it.**
//! The first assertion each refusal test makes is "the attempt did not unwind".
//! On its own that assertion passes for any function that happens not to panic,
//! including one that does nothing at all — so a harness that only ever watches
//! the real design cannot distinguish a genuinely atomic ledger from a lucky
//! one. [`NaiveBooks`] is a two-account ledger with the opposite ordering: it
//! writes the payer's leg first and checks afterwards, which puts a fallible
//! step after a write. Driven through the *identical* `catch_unwind` harness it
//! unwinds and leaves its total wrong, which is what proves the harness can tell
//! the two designs apart. This is the same discipline `tests/lints.sh` applies
//! to the lint gate: inject a known hazard and watch the gate stop it, rather
//! than check that a configuration file contains the right lines.
//!
//! **The measured numbers, from the research session that settled the design.**
//! Against an opening total of 100 cents, a transfer of 500:
//!
//! | design | outcome | total afterwards |
//! |---|---|---|
//! | naive: decrement, then check, then increment | unwinds | **−400** |
//! | compute-then-commit ([`Books::transfer`]) | returns `Err(Overdraft)` | **100** |
//!
//! [`NaiveBooks`] reproduces the first row exactly, at those numbers.
//!
//! **The mutant is not reachable from the library.** It is a private type in
//! this file, it is not exported, and it is not added to the crate under any
//! configuration. An integration test compiles as its own crate, so nothing in
//! `src/` can name it.
//!
//! **Expected noise.** The mutant test deliberately provokes a panic and catches
//! it. `libtest` captures a passing test's output, so the unwind message appears
//! only if something in this file actually fails.

use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use sim::books::{Books, PostError};
use sim::config::{self, Params};
use sim::ids::{Account, FirmId, FirmSlot, GoodId, HouseholdId};
use sim::money::Money;

/// Households in the atomicity economy. Small on purpose: these tests are about
/// the ordering of writes inside one operation, not about scale.
const HOUSEHOLDS: u32 = 3;

/// Firm slots in the atomicity economy. Same reasoning.
const FIRM_SLOTS: u16 = 2;

/// The one good these books carry in v1.
const FOOD: GoodId = GoodId(0);

/// A good these books do not carry, used to reach the unknown-good refusal.
const NOT_A_GOOD: GoodId = GoodId(7);

/// A small but valid parameter set whose endowment sums exactly to its stock.
///
/// The shipped configuration is loaded through the **real** loader — the same
/// deserialisation the binary uses — and only the three sizing keys are
/// overridden afterwards. The money total is derived from the two counts and the
/// two configured liquidities rather than typed, so `Books::new` verifies this
/// helper's arithmetic rather than the other way round.
fn small_params() -> Params {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
    let (mut params, _hash) = config::load(&path).expect("the shipped configuration loads");

    params.sim.households = HOUSEHOLDS;
    params.sim.firms = u32::from(FIRM_SLOTS);
    params.money.total_money_cents = i64::from(HOUSEHOLDS)
        * params.household.initial_liquidity_cents
        + i64::from(FIRM_SLOTS) * params.firm.initial_liquidity_cents;

    params
}

fn books() -> Books {
    Books::new(&small_params()).expect(
        "the derived money total is exactly the endowment, so construction cannot \
         report EndowmentDoesNotSumToStock",
    )
}

fn household(index: u32) -> Account {
    Account::Household(HouseholdId(index))
}

fn firm(slot: u16) -> Account {
    Account::Firm(FirmId {
        slot: FirmSlot(slot),
        generation: 0,
    })
}

/// An address these books do not hold: one past the last household.
fn absent() -> Account {
    household(HOUSEHOLDS)
}

fn cash_cents(books: &Books, who: Account) -> i64 {
    books
        .cash_of(who)
        .expect("the account is one these books hold")
        .cents()
}

fn stock_units(books: &Books, who: Account) -> i64 {
    books
        .stock_of(who, FOOD)
        .expect("the account is one these books hold")
}

/// Every quantity a refused operation must leave exactly as it found it.
///
/// Compared as a whole rather than field by field so that a quantity added to
/// the books in a later phase is caught by this test the moment it is added to
/// [`snapshot`], instead of being silently omitted from the comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    total_money_cents: i64,
    balances: Vec<i64>,
    stock: Vec<i64>,
    total_stock_units: i64,
    produced_units: i64,
    consumed_units: i64,
    journal_len: usize,
    transactions_this_tick: u32,
    cash_residual_cents: i64,
    goods_residual_units: i64,
}

fn snapshot(books: &Books) -> Snapshot {
    let addresses: Vec<Account> = books.accounts().collect();
    Snapshot {
        total_money_cents: books.total_money().cents(),
        balances: addresses
            .iter()
            .map(|&who| cash_cents(books, who))
            .collect(),
        stock: addresses
            .iter()
            .map(|&who| stock_units(books, who))
            .collect(),
        total_stock_units: books.total_stock(FOOD),
        produced_units: books.produced(FOOD),
        consumed_units: books.consumed(FOOD),
        journal_len: books.journal().len(),
        transactions_this_tick: books.transactions_this_tick(),
        cash_residual_cents: books.cash_residual_cents(),
        goods_residual_units: books.goods_residual_units(),
    }
}

/// Drive one refusal through the panic-catching harness and assert the four
/// separate claims it makes.
///
/// They are four claims and not one, so they are four assertions. A refusal that
/// unwinds, a refusal that reports the wrong condition, a refusal that moved a
/// cent and a refusal that recorded a posting are four different defects, and a
/// single combined assertion would report whichever it noticed first.
///
/// The journal claim is stated separately even though [`Snapshot`] already
/// carries the journal length: it is the claim a reader of LEDG-02 comes here
/// looking for, and burying it inside a struct comparison would make its failure
/// read as "a snapshot differs" rather than "a refusal wrote to the journal".
fn refusal_is_atomic<F>(what: &str, attempt: F, expected: PostError)
where
    F: FnOnce(&mut Books) -> Result<(), PostError>,
{
    let mut books = books();
    let before = snapshot(&books);

    // AssertUnwindSafe is required rather than incidental: `&mut Books` is not
    // `UnwindSafe`, precisely because a panic could leave it half-written. That
    // is the possibility under test, so the assertion is exactly the right
    // thing to make and exactly the wrong thing to route around by cloning.
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| attempt(&mut books)));

    // 1. It did not unwind.
    let returned = match outcome {
        Ok(returned) => returned,
        Err(_) => panic!(
            "{what}: the attempt unwound instead of returning a refusal — a fallible \
             step ran after a write, which is the state compute-then-commit exists to \
             make unreachable"
        ),
    };

    // 2. It returned the refusal a caller can act on.
    assert_eq!(
        returned,
        Err(expected),
        "{what}: the books refused, but not with the condition the caller must see"
    );

    // 3. Every captured quantity is unchanged.
    assert_eq!(
        snapshot(&books),
        before,
        "{what}: a refused operation changed the books"
    );

    // 4. The journal grew by nothing.
    assert_eq!(
        books.journal().len(),
        before.journal_len,
        "{what}: a refused operation recorded a posting"
    );
}

#[test]
fn an_overdraft_leaves_the_books_exactly_as_it_found_them() {
    let balance = cash_cents(&books(), household(0));
    let amount = balance + 1;
    refusal_is_atomic(
        "an overdraft",
        |books| {
            books
                .transfer(household(0), firm(0), Money::from_cents(amount))
                .map(|_| ())
        },
        PostError::Overdraft {
            account: household(0),
            amount_cents: amount,
            balance_cents: balance,
        },
    );
}

#[test]
fn a_negative_amount_leaves_the_books_exactly_as_it_found_them() {
    refusal_is_atomic(
        "a negative amount",
        |books| {
            books
                .transfer(household(0), firm(0), Money::from_cents(-1))
                .map(|_| ())
        },
        PostError::NegativeAmount { amount_cents: -1 },
    );
}

#[test]
fn an_unknown_account_leaves_the_books_exactly_as_it_found_them() {
    // The payee is the unknown one, so the refusal happens after the payer has
    // already resolved — the deepest point in the compute step a caller can
    // reach with an address alone.
    refusal_is_atomic(
        "an unknown payee",
        |books| {
            books
                .transfer(household(0), absent(), Money::from_cents(100))
                .map(|_| ())
        },
        PostError::UnknownAccount(absent()),
    );
}

#[test]
fn self_dealing_leaves_the_books_exactly_as_it_found_them() {
    refusal_is_atomic(
        "a payer and payee that are the same account",
        |books| {
            books
                .transfer(household(0), household(0), Money::from_cents(100))
                .map(|_| ())
        },
        PostError::SelfDealing {
            account: household(0),
        },
    );
}

#[test]
fn an_exchange_refused_for_short_stock_leaves_the_books_exactly_as_it_found_them() {
    // The refusal that matters most for atomicity: `exchange` commits four
    // assignments, and the stock check is the LAST fallible step before them.
    // A design that wrote the cash legs before checking the stock would leave a
    // buyer who had paid for units it never received.
    let held = stock_units(&books(), firm(0));
    let requested = held + 1;
    refusal_is_atomic(
        "an exchange the seller cannot stock",
        |books| {
            books
                .exchange(
                    household(0),
                    firm(0),
                    FOOD,
                    requested,
                    Money::from_cents(100),
                )
                .map(|_| ())
        },
        PostError::ShortStock {
            account: firm(0),
            good: FOOD,
            units_requested: requested,
            units_held: held,
        },
    );
}

#[test]
fn a_consume_refused_for_short_stock_leaves_the_books_exactly_as_it_found_them() {
    let held = stock_units(&books(), household(0));
    let requested = held + 1;
    refusal_is_atomic(
        "a consumption the account cannot stock",
        |books| books.consume(household(0), FOOD, requested).map(|_| ()),
        PostError::ShortStock {
            account: household(0),
            good: FOOD,
            units_requested: requested,
            units_held: held,
        },
    );
}

#[test]
fn an_unknown_good_leaves_the_books_exactly_as_it_found_them() {
    refusal_is_atomic(
        "production of a good these books do not carry",
        |books| books.produce(firm(0), NOT_A_GOOD, 1).map(|_| ()),
        PostError::UnknownGood(NOT_A_GOOD),
    );
}

#[test]
fn a_transfer_that_can_complete_still_commits() {
    // The control for every test above. Without it, a `transfer` that refused
    // *everything* would satisfy all seven of them, and this file would be
    // proving that a ledger which does nothing is admirably atomic.
    let mut books = books();
    let payer_before = cash_cents(&books, household(0));
    let payee_before = cash_cents(&books, firm(0));
    let total_before = books.total_money().cents();

    let moved = books
        .transfer(household(0), firm(0), Money::from_cents(250))
        .expect("the payer holds more than 250 cents");

    assert_eq!(moved.cents(), 250);
    assert_eq!(cash_cents(&books, household(0)), payer_before - 250);
    assert_eq!(cash_cents(&books, firm(0)), payee_before + 250);
    assert_eq!(books.total_money().cents(), total_before);
    assert_eq!(books.journal().len(), 1);
    assert_eq!(books.cash_residual_cents(), 0);
}

// ---------------------------------------------------------------------------
// The mutant.
// ---------------------------------------------------------------------------

/// A two-account ledger with the write ordering [`Books::transfer`] rejects.
///
/// Private to this file and exported nowhere. It exists only to be driven
/// through the same harness as the real books and to fail where they pass.
struct NaiveBooks {
    payer_cents: i64,
    payee_cents: i64,
}

impl NaiveBooks {
    fn total_cents(&self) -> i64 {
        self.payer_cents + self.payee_cents
    }

    /// Decrement, then check, then increment: **a fallible step after a write.**
    ///
    /// This is the shape the real ledger inverts. Every step that can fail runs
    /// before any write there; here the payer's leg is already gone by the time
    /// the overdraft is noticed, and the abort leaves the second leg unwritten.
    fn transfer(&mut self, cents: i64) {
        self.payer_cents -= cents;
        assert!(
            self.payer_cents >= 0,
            "the payer cannot cover {cents} cents"
        );
        self.payee_cents += cents;
    }
}

#[test]
fn the_naive_ordering_unwinds_and_corrupts_its_total_under_the_same_harness() {
    let mut naive = NaiveBooks {
        payer_cents: 100,
        payee_cents: 0,
    };
    assert_eq!(naive.total_cents(), 100, "the opening total");

    let outcome = panic::catch_unwind(AssertUnwindSafe(|| naive.transfer(500)));

    assert!(
        outcome.is_err(),
        "the naive ordering did NOT unwind — the harness cannot discriminate \
         between the two designs, so every atomicity assertion above is vacuous"
    );
    assert_eq!(
        naive.total_cents(),
        -400,
        "the naive ordering unwound but left its total intact — the harness \
         cannot observe a half-applied transfer, so every atomicity assertion \
         above is vacuous"
    );
}

#[test]
fn the_real_books_answer_the_mutant_case_by_returning_instead() {
    // The mutant's exact scenario, at the mutant's exact numbers, against the
    // real ledger: opening 100, a transfer of 500. The naive design ends at
    // -400 having unwound; this one ends at 100 having returned a refusal.
    let mut params = small_params();
    params.sim.households = 1;
    params.sim.firms = 1;
    params.household.initial_liquidity_cents = 100;
    params.firm.initial_liquidity_cents = 0;
    params.money.total_money_cents = 100;
    let mut books = Books::new(&params).expect("100 cents endowed against a stock of 100");

    assert_eq!(books.total_money().cents(), 100, "the opening total");

    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        books.transfer(household(0), firm(0), Money::from_cents(500))
    }));

    let returned = outcome.expect("the real ledger returns rather than unwinding");
    assert_eq!(
        returned,
        Err(PostError::Overdraft {
            account: household(0),
            amount_cents: 500,
            balance_cents: 100,
        })
    );
    assert_eq!(books.total_money().cents(), 100, "not -400");
    assert!(books.journal().is_empty());
}
