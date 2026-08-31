//! Property tests for the ledger invariants a unit test cannot pin down
//! (LEDG-03, LEDG-04, LEDG-05).
//!
//! The unit tests in plans 02-02 through 02-05 pin behaviour at chosen points.
//! These properties pin it across the input space: no sequence of public ledger
//! operations, however adversarial the generator gets, may change the total
//! money, break the goods identity, or make a returned amount disagree with
//! what actually moved.
//!
//! **This file living under `tests/` is load-bearing rather than incidental.**
//! An integration test cannot reach the crate's `pub(crate)` fault-injection
//! vocabulary — `Books::corrupt_recorded_cash` and its siblings — so every
//! property here is a statement about what an *ordinary caller* is able to do.
//! Every economic phase from 5 onward is such a caller. If a property here
//! fails, a real caller can break the ledger; the seeded corruptions belong in
//! the unit tests of plan 02-05 and are deliberately unreachable from here.
//!
//! **The load-bearing property is [`transfer_return_matches_delta`].** LEDG-03's
//! real risk after this phase is a Phase 6 accumulator — payroll paid this
//! month, revenue this tick — bumped by the amount a caller *asked for* rather
//! than the amount that *moved*. The ledger itself stays perfect while the
//! derived total leaks, so no conservation check can catch it. This property is
//! what makes the returned value trustworthy enough to be the rule the module
//! documentation states.
//!
//! **A refusal is not a failure.** `PostError::Overdraft`, `SelfDealing`,
//! `NegativeAmount`, `NegativeUnits`, `ShortStock`, `EmptyExchange`,
//! `UnknownAccount` and `UnknownGood` are legitimate runtime outcomes — an
//! overdraft is an economic event. The properties therefore ignore whether an
//! operation succeeded and assert only what the books look like afterwards. The
//! one exception is the return-agreement property, which asserts that a refusal
//! changed *nothing at all*.
//!
//! No dependency is added here. `proptest` is already a locked dev-dependency
//! and continuous integration builds with `--locked`, so reaching for a crate
//! in this file would be a defect rather than a choice.

use std::path::Path;

use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

use sim::books::Books;
use sim::config::{self, Params};
use sim::ids::{Account, FirmId, FirmSlot, GoodId, HouseholdId};
use sim::invariants::CheckSet;
use sim::money::Money;

/// Households in the property economy.
///
/// Single digits on purpose. These properties are about the algebra of the
/// operations and not about scale, and a small account set makes a generated
/// address collide with a live one — and with *itself*, which is the
/// self-dealing region — often enough to matter.
const HOUSEHOLDS: u32 = 3;

/// Firm slots in the property economy. Same reasoning.
const FIRM_SLOTS: u16 = 2;

/// The one good these books carry in v1.
///
/// Named here rather than imported because `books::ONLY_GOOD` is private.
/// [`the_books_carry_exactly_the_good_these_strategies_name`] is the guard that
/// stops this from silently drifting away from the real table when Phase 5
/// (PROD-01) widens it.
const CARRIED_GOOD: GoodId = GoodId(0);

/// A small but valid parameter set whose endowment sums exactly to its stock.
///
/// The shipped configuration is loaded through the **real** loader — the same
/// deserialisation the binary uses — and only the three sizing keys are
/// overridden afterwards. The money total is *derived* from the two counts and
/// the two configured liquidities rather than typed: a hand-written total would
/// fail construction with `BooksError::EndowmentDoesNotSumToStock`, so deriving
/// it and then letting `Books::new` verify it makes the helper itself a check on
/// the constructor's arithmetic.
///
/// **The liveness gate is turned off, and that is a statement rather than a
/// convenience.** Liveness (LEDG-08) is a claim about a whole *tick* — that
/// money changed hands before the tick closed. These properties assert after
/// *every operation*, and a sequence that has so far only produced units is a
/// perfectly legitimate mid-tick state, not a conservation failure. Leaving the
/// gate on would make every such sequence fail for a reason that has nothing to
/// do with what is under test. The gate itself is proved end to end in
/// `tests/invariant_halt.rs`, at the tick level where it means something.
fn small_params() -> Params {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
    let (mut params, _hash) = config::load(&path).expect("the shipped configuration loads");

    params.sim.households = HOUSEHOLDS;
    params.sim.firms = u32::from(FIRM_SLOTS);
    params.money.total_money_cents = i64::from(HOUSEHOLDS)
        * params.household.initial_liquidity_cents
        + i64::from(FIRM_SLOTS) * params.firm.initial_liquidity_cents;
    params.invariants.liveness_enabled = false;

    params
}

/// The books these properties run against, plus the check set for them.
fn small_books() -> (Books, CheckSet) {
    let params = small_params();
    let books = Books::new(&params).expect(
        "the derived money total is exactly the endowment, so construction cannot \
         report EndowmentDoesNotSumToStock",
    );
    let checks = CheckSet::from_params(&params);
    (books, checks)
}

/// A ledger operation a caller can ask for. One variant per public operation.
#[derive(Debug, Clone, Copy)]
enum Op {
    Transfer {
        from: Account,
        to: Account,
        cents: i64,
    },
    Produce {
        who: Account,
        good: GoodId,
        units: i64,
    },
    Consume {
        who: Account,
        good: GoodId,
        units: i64,
    },
    Exchange {
        buyer: Account,
        seller: Account,
        good: GoodId,
        units: i64,
        cents: i64,
    },
}

/// Ask the books for `op`, discarding the outcome.
///
/// Discarding is the point: a refusal is a legitimate outcome and these
/// properties are about the state afterwards. The one property that *does* look
/// at the outcome — [`transfer_return_matches_delta`] — calls the books
/// directly instead of going through here.
fn apply(op: Op, books: &mut Books) {
    match op {
        Op::Transfer { from, to, cents } => {
            let _ = books.transfer(from, to, Money::from_cents(cents));
        }
        Op::Produce { who, good, units } => {
            let _ = books.produce(who, good, units);
        }
        Op::Consume { who, good, units } => {
            let _ = books.consume(who, good, units);
        }
        Op::Exchange {
            buyer,
            seller,
            good,
            units,
            cents,
        } => {
            let _ = books.exchange(buyer, seller, good, units, Money::from_cents(cents));
        }
    }
}

/// Any address, live ones drawn far more often than dead ones.
///
/// The two dead arms are the `PostError::UnknownAccount` region, and they are
/// drawn explicitly rather than hoped for. The second of them is the subtler
/// one: a firm slot that genuinely exists, named at a generation that does not
/// occupy it. Phase 10's respawn is exactly that shape, and an address that
/// resolved by slot alone would silently hit the successor's balance.
fn any_account() -> impl Strategy<Value = Account> {
    prop_oneof![
        8 => (0..HOUSEHOLDS).prop_map(|index| Account::Household(HouseholdId(index))),
        8 => (0..FIRM_SLOTS).prop_map(|slot| Account::Firm(FirmId {
            slot: FirmSlot(slot),
            generation: 0,
        })),
        1 => (HOUSEHOLDS..HOUSEHOLDS + 3)
            .prop_map(|index| Account::Household(HouseholdId(index))),
        1 => (0..FIRM_SLOTS + 2, 1u32..3).prop_map(|(slot, generation)| Account::Firm(FirmId {
            slot: FirmSlot(slot),
            generation,
        })),
    ]
}

/// Any cent amount, with the refusal regions drawn deliberately often.
///
/// A uniform `any::<i64>()` reaches zero, the negatives near the boundary and
/// the "just over a balance" band with probability effectively zero over a few
/// hundred cases, so each is drawn explicitly. This is the lesson
/// `tests/money_props.rs` records in its own header: a strategy that only draws
/// plausible middle-of-the-range values proves the happy path works and nothing
/// else.
///
/// **The ordinary band is `1..5_000`, deliberately off the round numbers.** The
/// configured liquidities are 5 000 and 50 000 cents, so no draw from that band
/// coincides with a whole balance and a rule that happens to work on exactly one
/// endowment gets no help here. The two upper arms straddle the largest balance
/// in these books, so the overdraft boundary is crossed from both sides.
fn any_cents() -> impl Strategy<Value = i64> {
    prop_oneof![
        // Zero: a no-op transfer, and the `EmptyExchange` region for exchange.
        3 => Just(0i64),
        // The negative region, just inside the boundary and at it.
        2 => Just(-1i64),
        1 => Just(i64::MIN),
        // Larger than any balance these books hold, at two very different
        // scales: one a plausible overdraft, one the representable maximum.
        3 => 50_001i64..1_000_000,
        1 => Just(i64::MAX),
        // Ordinary amounts.
        10 => 1i64..5_000,
    ]
}

/// Any unit count, with the refusal regions drawn deliberately often.
///
/// **The upper boundary is deliberately absent, and its absence is a decision.**
/// `Books::produce` adds the count to a stock with bare integer arithmetic under
/// this project's overflow checks, so `i64::MAX` units *aborts* rather than
/// being refused — that is T-02-17 working as designed, before any write. An
/// abort is not a conservation failure and generating one here would test the
/// panic rather than the algebra. The counts below stay small enough that no
/// sequence can reach the boundary.
///
/// The `400..4_000` arm is the `PostError::ShortStock` region: larger than the
/// 165-unit endowment any single account starts with. The ordinary arm runs to
/// 166 rather than 165, so a draw can sit one unit either side of a whole
/// inventory.
fn any_units() -> impl Strategy<Value = i64> {
    prop_oneof![
        // Zero: the `EmptyExchange` region for exchange, a no-op elsewhere.
        3 => Just(0i64),
        // The negative region, just inside the boundary and at it.
        2 => Just(-1i64),
        1 => Just(i64::MIN),
        // More units than any account can be holding.
        3 => 400i64..4_000,
        // Ordinary counts, straddling the configured 165-unit endowment.
        10 => 1i64..167,
    ]
}

/// Any good identifier: the carried one, or one the books do not carry.
///
/// The second arm is the `PostError::UnknownGood` region. Reading an uncarried
/// good as zero rather than refusing it would let a caller "consume" from a good
/// that does not exist, so the refusal is load-bearing and has to be generated.
fn any_good() -> impl Strategy<Value = GoodId> {
    prop_oneof![
        9 => Just(CARRIED_GOOD),
        1 => (1u16..4).prop_map(GoodId),
    ]
}

/// Any single ledger operation.
///
/// Three arms are drawn explicitly rather than left to chance, all for the same
/// reason: a region reached by coincidence is reached at a rate nobody chose,
/// and that rate silently falls as the economy grows.
///
/// **The two `SelfDealing` arms** name one account on both legs by
/// construction. With five live accounts a collision would happen anyway, but
/// not at a rate anyone decided on.
///
/// **The plausible-trade arm** is the opposite case, and it is the one that was
/// measured rather than guessed. An exchange succeeds only when six conditions
/// hold at once — positive cash, positive units, two different live accounts, a
/// carried good, a buyer who can pay and a seller who holds the stock — and
/// households open with no stock at all, so a seller drawn uniformly is usually
/// one that cannot sell. Instrumenting the generator gave 50 successful
/// exchanges against 875 refusals: about one draw in twenty. A property whose
/// interesting branch fires that rarely is close to not being tested, so a
/// household-buys-from-firm arm at counts and amounts both sides can meet is
/// drawn on purpose. Re-instrumented afterwards it gives 391 against 850 — both
/// branches genuinely exercised. It is also what puts stock into household
/// hands, which is what lets a household's `consume` and a resale ever succeed.
fn any_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => (any_account(), any_account(), any_cents())
            .prop_map(|(from, to, cents)| Op::Transfer { from, to, cents }),
        1 => (any_account(), any_cents())
            .prop_map(|(who, cents)| Op::Transfer { from: who, to: who, cents }),
        4 => (any_account(), any_good(), any_units())
            .prop_map(|(who, good, units)| Op::Produce { who, good, units }),
        4 => (any_account(), any_good(), any_units())
            .prop_map(|(who, good, units)| Op::Consume { who, good, units }),
        6 => (any_account(), any_account(), any_good(), any_units(), any_cents())
            .prop_map(|(buyer, seller, good, units, cents)| Op::Exchange {
                buyer,
                seller,
                good,
                units,
                cents,
            }),
        1 => (any_account(), any_good(), any_units(), any_cents())
            .prop_map(|(who, good, units, cents)| Op::Exchange {
                buyer: who,
                seller: who,
                good,
                units,
                cents,
            }),
        3 => (0..HOUSEHOLDS, 0..FIRM_SLOTS, 1i64..40, 1i64..500)
            .prop_map(|(household, slot, units, cents)| Op::Exchange {
                buyer: Account::Household(HouseholdId(household)),
                seller: Account::Firm(FirmId {
                    slot: FirmSlot(slot),
                    generation: 0,
                }),
                good: CARRIED_GOOD,
                units,
                cents,
            }),
    ]
}

/// A sequence of operations long enough for state to accumulate — a firm sells
/// out, a household is drained, stock moves twice — and short enough that a few
/// hundred cases run in well under a second.
fn any_ops() -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(any_op(), 1..24)
}

/// The strategies above name [`CARRIED_GOOD`] and nothing else as carried.
///
/// A plain test rather than a property: it is a single fact about the books, and
/// it is what turns the `1u16..4` arm of [`any_good`] from "some numbers" into
/// "goods these books do not carry". When Phase 5 widens the table this fails
/// immediately, rather than the uncarried arm quietly becoming a carried one and
/// the refusal region silently emptying.
#[test]
fn the_books_carry_exactly_the_good_these_strategies_name() {
    let (books, _checks) = small_books();
    assert_eq!(books.goods(), [CARRIED_GOOD]);
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Explicit, so the run time is a property of this file rather than of
        // whatever environment happens to invoke it.
        cases: 256,
        // Explicit, so counterexamples land at the committed repository path
        // rather than wherever the default source-parallel rule resolves to. A
        // counterexample found once — in continuous integration, at three in the
        // morning, on a case that recurs one run in a thousand — is replayed on
        // every future run instead of being discarded.
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(
            ".proptest-regressions/ledger_props.txt",
        ))),
        ..ProptestConfig::default()
    })]

    /// The total money in the books equals the opening stock after **every**
    /// operation of any generated sequence (LEDG-04).
    ///
    /// Both sides are asserted: the direct comparison against the opening stock,
    /// *and* the check set. The check set is the thing under test elsewhere in
    /// this phase, so an independent direct comparison here means a broken check
    /// cannot hide a broken ledger — if `check_money` were weakened to `Ok(())`
    /// this property still fails on a leak.
    #[test]
    fn total_money_is_conserved_under_any_operation_sequence(ops in any_ops()) {
        let (mut books, checks) = small_books();
        let opening = books.opening_stock();

        prop_assert_eq!(books.total_money(), opening, "the books opened off the stock");

        for (index, op) in ops.iter().enumerate() {
            apply(*op, &mut books);

            prop_assert_eq!(
                books.total_money(),
                opening,
                "total money moved after operation {} ({:?})",
                index,
                op
            );

            let outcome = checks.run(&books, 0);
            prop_assert!(
                outcome.is_ok(),
                "the check set reported {:?} after operation {} ({:?})",
                outcome,
                index,
                op
            );
        }
    }

    /// What `transfer` returns is what the books actually moved, and a refusal
    /// moves nothing at all (LEDG-02, LEDG-03).
    ///
    /// **The load-bearing property of this file.** On success the returned
    /// amount is compared against the payer's observed decrease *and* the
    /// payee's observed increase, separately — a return value that matched only
    /// one side would be a half-applied transfer and is what the two assertions
    /// together exclude.
    ///
    /// On refusal the assertion is exhaustive rather than representative: both
    /// balances, the journal length, the transaction count and **both** running
    /// residuals are unchanged. That is the every-refused-input half of the
    /// atomicity claim (LEDG-02); plan 02-06 covers the panic half. Between them
    /// no path writes half a transfer.
    #[test]
    fn transfer_return_matches_delta(ops in any_ops()) {
        let (mut books, _checks) = small_books();

        for (index, op) in ops.iter().enumerate() {
            let Op::Transfer { from, to, cents } = *op else {
                // Not a transfer: apply it anyway, so the transfers that follow
                // run against books someone has already moved stock and cash
                // around in.
                apply(*op, &mut books);
                continue;
            };

            let payer_before = books.cash_of(from);
            let payee_before = books.cash_of(to);
            let journal_before = books.journal().len();
            let transactions_before = books.transactions_this_tick();
            let cash_residual_before = books.cash_residual_cents();
            let goods_residual_before = books.goods_residual_units();

            let outcome = books.transfer(from, to, Money::from_cents(cents));

            let payer_after = books.cash_of(from);
            let payee_after = books.cash_of(to);

            match outcome {
                Ok(moved) => {
                    // A successful transfer resolved both addresses, and
                    // `SelfDealing` guarantees they are two different accounts,
                    // so the two deltas below are independent observations.
                    let paid = payer_before.expect("a successful transfer resolved its payer")
                        - payer_after.expect("the payer is still a live account");
                    let received = payee_after.expect("the payee is still a live account")
                        - payee_before.expect("a successful transfer resolved its payee");

                    prop_assert_eq!(
                        paid, moved,
                        "operation {}: transfer returned {:?} but the payer lost {:?}",
                        index, moved, paid
                    );
                    prop_assert_eq!(
                        received, moved,
                        "operation {}: transfer returned {:?} but the payee gained {:?}",
                        index, moved, received
                    );
                }
                Err(refusal) => {
                    prop_assert_eq!(
                        payer_before, payer_after,
                        "operation {}: refused with {:?} but the payer's balance changed",
                        index, refusal
                    );
                    prop_assert_eq!(
                        payee_before, payee_after,
                        "operation {}: refused with {:?} but the payee's balance changed",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.journal().len(), journal_before,
                        "operation {}: refused with {:?} but wrote a posting",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.transactions_this_tick(), transactions_before,
                        "operation {}: refused with {:?} but counted a transaction",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.cash_residual_cents(), cash_residual_before,
                        "operation {}: refused with {:?} but moved the cash residual",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.goods_residual_units(), goods_residual_before,
                        "operation {}: refused with {:?} but moved the goods residual",
                        index, refusal
                    );
                }
            }
        }
    }

    /// What `exchange` returns is what the books actually moved, on both legs,
    /// and a refusal moves nothing at all (LEDG-02, LEDG-03).
    ///
    /// The same claim as [`transfer_return_matches_delta`] for the two-legged
    /// operation, and it is not redundant with it. `exchange` returns a *pair*,
    /// and its module documentation says a caller must use **both**: an
    /// accumulator bumped by the intended unit count while the cash leg is taken
    /// from the return value would leak on one side only. Asserting the two legs
    /// separately, against four observed balances, is what makes that
    /// unavailable.
    #[test]
    fn exchange_returns_match_deltas(ops in any_ops()) {
        let (mut books, _checks) = small_books();

        for (index, op) in ops.iter().enumerate() {
            let Op::Exchange { buyer, seller, good, units, cents } = *op else {
                apply(*op, &mut books);
                continue;
            };

            let buyer_cash_before = books.cash_of(buyer);
            let seller_cash_before = books.cash_of(seller);
            let buyer_stock_before = books.stock_of(buyer, good);
            let seller_stock_before = books.stock_of(seller, good);
            let journal_before = books.journal().len();
            let transactions_before = books.transactions_this_tick();
            let cash_residual_before = books.cash_residual_cents();
            let goods_residual_before = books.goods_residual_units();

            let outcome = books.exchange(buyer, seller, good, units, Money::from_cents(cents));

            match outcome {
                Ok((cash_moved, units_moved)) => {
                    let paid = buyer_cash_before.expect("a successful exchange resolved its buyer")
                        - books.cash_of(buyer).expect("the buyer is still a live account");
                    let received = books.cash_of(seller).expect("the seller is still live")
                        - seller_cash_before.expect("a successful exchange resolved its seller");
                    let gained = books.stock_of(buyer, good).expect("the buyer is still live")
                        - buyer_stock_before.expect("a successful exchange resolved its buyer");
                    let given = seller_stock_before.expect("a successful exchange resolved its seller")
                        - books.stock_of(seller, good).expect("the seller is still live");

                    prop_assert_eq!(
                        paid, cash_moved,
                        "operation {}: exchange returned {:?} cash but the buyer paid {:?}",
                        index, cash_moved, paid
                    );
                    prop_assert_eq!(
                        received, cash_moved,
                        "operation {}: exchange returned {:?} cash but the seller received {:?}",
                        index, cash_moved, received
                    );
                    prop_assert_eq!(
                        gained, units_moved,
                        "operation {}: exchange returned {} units but the buyer gained {}",
                        index, units_moved, gained
                    );
                    prop_assert_eq!(
                        given, units_moved,
                        "operation {}: exchange returned {} units but the seller gave up {}",
                        index, units_moved, given
                    );
                }
                Err(refusal) => {
                    prop_assert_eq!(
                        buyer_cash_before, books.cash_of(buyer),
                        "operation {}: refused with {:?} but the buyer's cash changed",
                        index, refusal
                    );
                    prop_assert_eq!(
                        seller_cash_before, books.cash_of(seller),
                        "operation {}: refused with {:?} but the seller's cash changed",
                        index, refusal
                    );
                    prop_assert_eq!(
                        buyer_stock_before, books.stock_of(buyer, good),
                        "operation {}: refused with {:?} but the buyer's stock changed",
                        index, refusal
                    );
                    prop_assert_eq!(
                        seller_stock_before, books.stock_of(seller, good),
                        "operation {}: refused with {:?} but the seller's stock changed",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.journal().len(), journal_before,
                        "operation {}: refused with {:?} but wrote a posting",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.transactions_this_tick(), transactions_before,
                        "operation {}: refused with {:?} but counted a transaction",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.cash_residual_cents(), cash_residual_before,
                        "operation {}: refused with {:?} but moved the cash residual",
                        index, refusal
                    );
                    prop_assert_eq!(
                        books.goods_residual_units(), goods_residual_before,
                        "operation {}: refused with {:?} but moved the goods residual",
                        index, refusal
                    );
                }
            }
        }
    }
}
