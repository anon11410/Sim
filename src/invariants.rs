//! The invariant phase: an ordered set of checks that reads the books and
//! returns a `Result` (LEDG-04, LEDG-05, LEDG-08, LEDG-09, LEDG-10).
//!
//! **This is a real step, not an assertion that a build profile can remove.**
//! LEDG-10 asks for a phase that exists in the shipped binary and reports a
//! violation as a value the caller must handle. Every check here therefore
//! returns `Result<(), Violation>`, the caller propagates it with `?`, and the
//! loop stops because the caller stopped it. Nothing in this module is
//! conditional on how the crate was compiled.
//!
//! **The set is built once, from the parameters.** [`CheckSet::from_params`] is
//! the one place in the whole crate that reads the liveness configuration key.
//! Filtering happens there, at construction, so the per-tick path carries no
//! configuration lookup and no branch on the gate — the checks a tick runs were
//! decided before the run started and cannot change under it.
//!
//! **The order is part of the contract.** A single corruption can trip more
//! than one check, so the order decides which [`Violation`] a caller sees. The
//! documented full order is money conservation, goods conservation,
//! non-negativity, zero-sum, liveness; the first, second and last are
//! implemented, and plan 02-04 inserts the remaining two at those positions.
//! Money conservation is **first** because a leak is the highest-severity
//! finding and reporting it as "some account went negative" sends a debugger to
//! the wrong place. Liveness is **last** because it is the only check that can
//! fire on books that are entirely correct.
//!
//! The checks read the books through shared references and mutate nothing.
//! None of them draws from the random number generator: a draw here would shift
//! every downstream sub-stream and silently re-trajectory every run (CORE-04).
//!
//! Every amount here is an integer count of cents or of units. Nothing in this
//! module belongs to the float domain, and it names no type from that domain at
//! all.

use thiserror::Error;

use crate::books::{Books, Posting};
use crate::config::Params;
use crate::ids::GoodId;

/// How many cash transactions a tick must record for the liveness check to
/// pass.
///
/// A named constant here and deliberately **not** a configuration key. "At
/// least one" is the definition of the check rather than a tunable quantity,
/// and encoding the minimum as a parameter whose zero value means "disabled"
/// would be a hidden second switch. The switch is the boolean gate in
/// `[invariants]` and nothing else.
pub const MINIMUM_TRANSACTIONS_PER_TICK: u32 = 1;

/// The books are wrong. Always a defect, never an economic event.
///
/// Derives `Clone`, `PartialEq` and `Eq` so a test can assert the exact
/// expected value; a test that matches a substring of the rendered message
/// passes when the wrong check fired, when the tick is wrong and when the named
/// agent is wrong.
///
/// Deliberately **not** `Copy`: a later variant may need an owned collection,
/// and this type has no reason to promise otherwise.
///
/// Every interpolated field below is a number, an identity or a posting. No
/// path, host name, wall-clock reading or process id can reach a halt message,
/// which is a determinism requirement (TICK-06) before it is anything else.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Violation {
    /// The cents the books hold do not match the opening stock, or the
    /// journal's running residual is not zero, or both.
    ///
    /// The offending posting is **owned** and is **optional**. Owned, because
    /// an index into a buffer that `end_of_tick` is about to clear is a
    /// dangling reference in all but name — by the time a human reads the
    /// message the journal is gone. Behind a `Box` because a posting is around
    /// eighty bytes and two of these variants carry one: inline, the whole
    /// enum passes the size at which `clippy::result_large_err` refuses to
    /// compile a `Result` carrying it, and every function in the crate that
    /// propagates a violation returns exactly that `Result`. The indirection
    /// costs one allocation on a path that has already decided to abort the
    /// run, and changes nothing a reader of the message can observe.
    /// Optional, because a discrepancy
    /// the journal does not describe is a real, reachable case: a write that
    /// happened outside the posting path leaves every posting's residual at
    /// zero and there genuinely is no offending posting to name. The rendered
    /// message then says so in those terms; a synthetic posting in an error
    /// message is a lie a future reader will chase.
    #[error(
        "tick {tick}: money conservation broken by {delta_cents} cents \
         (books hold {actual_cents} cents against an opening stock of \
         {expected_cents} cents; journal residual {journal_residual_cents} cents); {}",
        render_posting(.posting)
    )]
    MoneyConservation {
        tick: u32,
        expected_cents: i64,
        actual_cents: i64,
        delta_cents: i64,
        journal_residual_cents: i64,
        posting: Option<Box<Posting>>,
    },

    /// The goods identity does not hold for a good: `produced − consumed −
    /// Σstock` is not zero, or the journal's running goods residual is not
    /// zero, or both.
    ///
    /// Names all three quantities, so a reader can tell **which** of them
    /// moved without re-running the tick. The posting is owned, boxed and
    /// optional on exactly the terms
    /// [`Violation::MoneyConservation`] sets out, and the residual named here is
    /// the goods one — never the cash one. Reporting a cash residual against a
    /// goods discrepancy would send a debugger to the wrong column.
    #[error(
        "tick {tick}: goods conservation broken for {good} by {delta_units} units \
         (produced {produced}, consumed {consumed}, held {stock}; journal residual \
         {journal_residual_units} units); {}",
        render_posting(.posting)
    )]
    GoodsConservation {
        tick: u32,
        good: GoodId,
        produced: i64,
        consumed: i64,
        stock: i64,
        delta_units: i64,
        journal_residual_units: i64,
        posting: Option<Box<Posting>>,
    },

    /// The tick recorded fewer cash transactions than the minimum.
    ///
    /// Carries no posting: by construction there was none. That is the whole
    /// finding.
    #[error(
        "tick {tick}: liveness — {counted} transactions recorded, at least \
         {required} required; no posting, which is the violation"
    )]
    Liveness {
        tick: u32,
        counted: u32,
        required: u32,
    },
}

/// How a violation names the posting it found, or says that there is none.
fn render_posting(posting: &Option<Box<Posting>>) -> String {
    match posting {
        Some(posting) => format!("offending posting {posting}"),
        None => String::from(
            "no offending posting: every posting in the tick's journal conserves, \
             so the discrepancy was written outside the posting path",
        ),
    }
}

/// Which check a [`Violation`] came from, as a value a test can order and
/// compare.
///
/// **Declared in the order the checks run**, so the derived `Ord` agrees with
/// [`ALL_CHECKS`] rather than quietly contradicting it. A new identifier goes
/// at its run position, not at the end: this enum is not a wire shape and has
/// no append-only obligation, and an `Ord` that disagreed with the run order
/// would be a trap laid for the first test that sorts one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckId {
    MoneyConservation,
    GoodsConservation,
    Liveness,
}

/// A check: the books and the tick in, a violation or nothing out.
///
/// A plain function pointer, not a trait object and not a closure. The checks
/// are a fixed table known at compile time, and nothing here needs to capture.
pub type CheckFn = fn(&Books, u32) -> Result<(), Violation>;

/// The full, ordered table, and the single source of truth for the order.
///
/// The order test reads this array; there is no second hand-written list for it
/// to drift from. Plan 02-04 inserts non-negativity and zero-sum at the
/// positions the module docs name, leaving money conservation first, goods
/// conservation second and liveness last.
///
/// Goods conservation sits **second, immediately after money conservation**,
/// and not at the end for convenience. The order decides which violation a
/// caller observes when one corruption trips two checks, and plan 02-04 asserts
/// the exact final sequence.
///
/// The second element of each triple is the check's stable name, which plan
/// 02-05's seeded-corruption tests report against.
pub const ALL_CHECKS: [(CheckId, &str, CheckFn); 3] = [
    (
        CheckId::MoneyConservation,
        "money_conservation",
        check_money,
    ),
    (
        CheckId::GoodsConservation,
        "goods_conservation",
        check_goods,
    ),
    (CheckId::Liveness, "liveness", check_liveness),
];

/// The checks active for this run, in order.
pub struct CheckSet {
    active: Vec<(CheckId, &'static str, CheckFn)>,
}

impl CheckSet {
    /// Build the active set from the run's parameters.
    ///
    /// **This is the one place in the crate that reads
    /// `invariants.liveness_enabled`.** It is read once, here, and never again;
    /// plan 02-06 adds a guard asserting exactly one read site. Filtering at
    /// construction is what keeps [`CheckSet::run`] free of a per-tick branch
    /// on a value that cannot change under it.
    pub fn from_params(params: &Params) -> CheckSet {
        let liveness_enabled = params.invariants.liveness_enabled;
        CheckSet {
            active: ALL_CHECKS
                .iter()
                .copied()
                .filter(|(id, _, _)| liveness_enabled || *id != CheckId::Liveness)
                .collect(),
        }
    }

    /// Run the active checks in order and return the first violation.
    ///
    /// The first violation wins and the rest are not run — the same convention
    /// the configuration module already documents. There is no value in
    /// enumerating every consequence of one corruption, and the ordering above
    /// is what makes the first one the diagnostically useful one.
    ///
    /// No configuration lookup, no allocation and no branch on the gate.
    pub fn run(&self, books: &Books, tick: u32) -> Result<(), Violation> {
        for (_id, _name, check) in &self.active {
            check(books, tick)?;
        }
        Ok(())
    }

    /// The active check identifiers, in the order they run.
    ///
    /// Exposed so a test can assert what the gate produced without reaching
    /// into a private field, and so an assertion can be made on the *sequence*
    /// rather than on a length — a length assertion passes when two checks are
    /// swapped.
    pub fn active_ids(&self) -> Vec<CheckId> {
        self.active.iter().map(|(id, _, _)| *id).collect()
    }
}

/// Money conservation (LEDG-04): the books hold exactly the opening stock, and
/// the journal agrees.
///
/// Two **independent** sources are compared, and both must be clean. The total
/// comes from the balance vectors; the running residual comes from the
/// postings. Neither is derived from the other, which is what makes this check
/// non-vacuous — a corruption that fooled one would have to fool the other in
/// the same direction.
fn check_money(books: &Books, tick: u32) -> Result<(), Violation> {
    let expected_cents = books.opening_stock().cents();
    let actual_cents = books.total_money().cents();
    let journal_residual_cents = books.cash_residual_cents();
    // The difference is a diagnostic quantity, not an amount of money, so it
    // saturates rather than aborting: a report of a broken invariant must not
    // itself be the thing that fails.
    let delta_cents = actual_cents.saturating_sub(expected_cents);

    if delta_cents == 0 && journal_residual_cents == 0 {
        return Ok(());
    }

    Err(Violation::MoneyConservation {
        tick,
        expected_cents,
        actual_cents,
        delta_cents,
        journal_residual_cents,
        posting: first_breaking_cash_posting(books.journal()).map(Box::new),
    })
}

/// Goods conservation (LEDG-05): for every good, `produced − consumed − Σstock`
/// is zero, and the journal agrees.
///
/// Two **independent** sources, exactly as [`check_money`] uses for cash, and
/// both must be clean. One side is the identity recomputed from the books'
/// fields — the produced total, the consumed total and the stock summed over
/// every account, each maintained from the *arguments* of the operations. The
/// other is the running residual accumulated from the *legs of the postings*.
/// Neither is derived from the other. A single-source check here — recomputing
/// `produced` by walking the journal, say — would compare a number against
/// itself and pass forever, which is threat T-02-15.
///
/// The loop runs over the goods the books carry, so its body is entered on
/// every tick of every run. A conservation check whose loop never runs passes
/// vacuously.
///
/// Deliberately **not** factored into a shared helper with [`check_money`].
/// The cash residual and the goods residual are two different quantities, and a
/// generic scan that hid which one it was reporting would put the wrong number
/// in the message and send a debugger to the wrong column.
fn check_goods(books: &Books, tick: u32) -> Result<(), Violation> {
    // One residual for one good in v1. Phase 5's goods table makes this a
    // per-good quantity; the loop below is already shaped for that.
    let journal_residual_units = books.goods_residual_units();

    for &good in books.goods() {
        let produced = books.produced(good);
        let consumed = books.consumed(good);
        let stock = books.total_stock(good);
        // A diagnostic quantity, not a count of units that exist, so it
        // saturates rather than aborting: a report of a broken invariant must
        // not itself be the thing that fails.
        let delta_units = produced.saturating_sub(consumed).saturating_sub(stock);

        if delta_units == 0 && journal_residual_units == 0 {
            continue;
        }

        return Err(Violation::GoodsConservation {
            tick,
            good,
            produced,
            consumed,
            stock,
            delta_units,
            journal_residual_units,
            posting: first_breaking_goods_posting(books.journal()).map(Box::new),
        });
    }

    Ok(())
}

/// Liveness (LEDG-08): the tick recorded at least one cash transaction.
///
/// The only check that can fire on books that are entirely correct, which is
/// why it runs last and why it has a switch at all. It closes the degenerate
/// pass where money conserves because nothing ever moved.
fn check_liveness(books: &Books, tick: u32) -> Result<(), Violation> {
    let counted = books.transactions_this_tick();
    if counted >= MINIMUM_TRANSACTIONS_PER_TICK {
        return Ok(());
    }
    Err(Violation::Liveness {
        tick,
        counted,
        required: MINIMUM_TRANSACTIONS_PER_TICK,
    })
}

/// The first posting in the tick whose running cash residual is non-zero, if
/// there is one.
///
/// **A forward linear scan, and it must stay one.** A search that repeatedly
/// discards half the journal assumes the residual has a monotone onset, and it
/// does not: residuals cancel. A cent dropped at posting 50 and healed by an
/// equal over-credit at posting 120, then broken differently at posting 200,
/// gives a residual sequence that is non-zero, then zero, then non-zero again.
/// A halving search over that answers 200; the correct answer — the posting
/// where the books first stopped conserving — is 50, and a debugger sent to 200
/// spends a day in the wrong part of the tick. Measured on exactly that
/// journal.
///
/// The scan costs less than the recompute it accompanies: about 80 nanoseconds
/// for a tick of 274 postings, against about 175 for the conservation sum. It
/// is not a candidate for optimisation.
fn first_breaking_cash_posting(journal: &[Posting]) -> Option<Posting> {
    journal
        .iter()
        .copied()
        .find(|posting| posting.cash_residual_cents != 0)
}

/// The first posting in the tick whose running **goods** residual is non-zero,
/// if there is one.
///
/// A forward linear scan, and it must stay one, for the reason spelled out
/// immediately above: residuals cancel, so a search that repeatedly discards
/// half the journal reports a later, healthy-looking posting. Measured on the
/// cash side against a journal broken at #50, healed at #120 and broken again
/// at #200 — the linear scan answers 50, a halving search answers 200. Nothing
/// about the goods residual makes it better behaved.
///
/// A near-copy of its cash sibling, and deliberately not folded into one
/// generic scan over a residual selector. The two residuals are different
/// quantities, and the saving would be one line at the cost of a message that
/// cannot say which one it found.
fn first_breaking_goods_posting(journal: &[Posting]) -> Option<Posting> {
    journal
        .iter()
        .copied()
        .find(|posting| posting.goods_residual_units != 0)
}

/// The gate and the construction rule, at unit granularity.
///
/// `tests/invariant_halt.rs` proves that the loop aborts; these prove *why*,
/// and they are what a future change to [`CheckSet::from_params`] trips over
/// first.
///
/// Every violation below is asserted by whole-value equality against a
/// constructed [`Violation`]. A test that matches a substring of a rendered
/// message passes when the wrong check fired, when the tick is wrong and when
/// the named agent is wrong — precisely the class of negative test that passes
/// for the wrong reason.
#[cfg(test)]
mod liveness {
    use super::*;
    use std::path::Path;

    use crate::ids::{Account, FirmId, FirmSlot, HouseholdId};
    use crate::money::Money;

    /// The shipped parameters with only the gate set afterwards.
    ///
    /// Loaded through the real deserialisation path rather than hand-written:
    /// a parameter literal would need updating every time a key is added and
    /// would drift out of agreement with the shipped file.
    fn shipped_with_liveness(enabled: bool) -> Params {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
        let (mut params, _hash) = crate::config::load(&path).expect("the configuration loads");
        params.invariants.liveness_enabled = enabled;
        params
    }

    fn empty_books(enabled: bool) -> (Books, CheckSet) {
        let params = shipped_with_liveness(enabled);
        let books = Books::new(&params).expect("the shipped endowment sums to the stock");
        let checks = CheckSet::from_params(&params);
        (books, checks)
    }

    fn move_a_cent(books: &mut Books) {
        let payer = Account::Household(HouseholdId(0));
        let payee = Account::Firm(FirmId {
            slot: FirmSlot(0),
            generation: 0,
        });
        books
            .transfer(payer, payee, Money::from_cents(1))
            .expect("an endowed household can pay a cent");
    }

    #[test]
    fn the_gate_decides_the_exact_sequence_of_active_checks() {
        // On the sequence, never on the length: a length assertion passes when
        // two checks are swapped, and the order is what decides which violation
        // a caller sees.
        assert_eq!(
            CheckSet::from_params(&shipped_with_liveness(true)).active_ids(),
            vec![
                CheckId::MoneyConservation,
                CheckId::GoodsConservation,
                CheckId::Liveness
            ],
            "with the gate on, liveness runs and runs last"
        );
        assert_eq!(
            CheckSet::from_params(&shipped_with_liveness(false)).active_ids(),
            vec![CheckId::MoneyConservation, CheckId::GoodsConservation],
            "with the gate off, every check except liveness still runs"
        );
    }

    #[test]
    fn a_tick_that_traded_nothing_fails_only_because_the_gate_is_on() {
        let (books, on) = empty_books(true);
        assert_eq!(
            on.run(&books, 7),
            Err(Violation::Liveness {
                tick: 7,
                counted: 0,
                required: MINIMUM_TRANSACTIONS_PER_TICK,
            })
        );

        // The same books value, checked by the set the gate off produces. The
        // books are identical; the gate is the only variable.
        let off = CheckSet::from_params(&shipped_with_liveness(false));
        assert_eq!(off.run(&books, 7), Ok(()));
    }

    #[test]
    fn a_tick_that_moved_a_cent_passes_with_the_gate_on() {
        // The positive direction, and the thing that stops the check from being
        // permanently red.
        let (mut books, checks) = empty_books(true);
        move_a_cent(&mut books);

        assert_eq!(books.transactions_this_tick(), 1);
        assert_eq!(checks.run(&books, 0), Ok(()));
    }

    #[test]
    fn the_transaction_count_resets_each_tick_so_liveness_is_a_per_tick_property() {
        // Without the reset, one transfer on tick 0 would satisfy liveness for
        // the whole decade and the check would prove nothing after tick 0.
        let (mut books, checks) = empty_books(true);
        move_a_cent(&mut books);
        assert_eq!(checks.run(&books, 0), Ok(()));

        books.end_of_tick();

        assert_eq!(books.transactions_this_tick(), 0);
        assert_eq!(
            checks.run(&books, 1),
            Err(Violation::Liveness {
                tick: 1,
                counted: 0,
                required: MINIMUM_TRANSACTIONS_PER_TICK,
            })
        );
    }
}

/// The goods identity, proved under both of the consumption models Phase 7 will
/// choose between (MKT-06).
///
/// Named `goods` so that an `invariants::goods` module-path filter selects
/// exactly these. The point of the pair of models is that the choice changes
/// nothing here: the same check function, reached through the same table entry,
/// and the same three accessors, evaluate both worlds. Everything below runs
/// through the public API with no fault injection — plan 02-05 owns the seeded
/// corruptions.
///
/// Violations are asserted by whole-value equality against a constructed
/// [`Violation`], never by matching a substring of a rendered message.
#[cfg(test)]
mod goods {
    use super::*;
    use std::path::Path;

    use crate::books::{Posting, PostingKind};
    use crate::ids::{Account, FirmId, FirmSlot, HouseholdId};
    use crate::money::Money;

    /// The one good v1 carries.
    const FOOD: GoodId = GoodId(0);

    /// Where goods conservation sits in the table. Second, immediately after
    /// money conservation, and read from the array rather than hand-written so
    /// this cannot drift from the order the run uses.
    const GOODS_POSITION: usize = 1;

    fn shipped_with_liveness(enabled: bool) -> Params {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
        let (mut params, _hash) = crate::config::load(&path).expect("the configuration loads");
        params.invariants.liveness_enabled = enabled;
        params
    }

    fn buyer() -> Account {
        Account::Household(HouseholdId(0))
    }

    fn seller() -> Account {
        Account::Firm(FirmId {
            slot: FirmSlot(0),
            generation: 0,
        })
    }

    /// The goods check itself, taken from the table by position — so every test
    /// below runs the same function the tick loop runs, and the position it
    /// occupies is asserted rather than assumed.
    fn goods_check() -> CheckFn {
        let (id, name, check) = ALL_CHECKS[GOODS_POSITION];
        assert_eq!(
            id,
            CheckId::GoodsConservation,
            "goods conservation is second"
        );
        assert_eq!(name, "goods_conservation");
        check
    }

    /// The identity read through the public accessors, which is the same
    /// arithmetic `check_goods` performs.
    fn identity(books: &Books) -> i64 {
        books.produced(FOOD) - books.consumed(FOOD) - books.total_stock(FOOD)
    }

    #[test]
    fn the_identity_holds_at_tick_zero_before_anything_has_happened() {
        // The endowment is counted into `produced`, so the books do not open
        // already failing by exactly the initial inventory.
        let books = Books::new(&shipped_with_liveness(false)).expect("the books open");
        assert!(books.total_stock(FOOD) > 0, "firms open holding inventory");
        assert_eq!(goods_check()(&books, 0), Ok(()));
        assert_eq!(identity(&books), 0);
    }

    #[test]
    fn immediate_consumption_holds_the_identity_at_every_step() {
        // Phase 7 option A: a purchased unit is consumed in the same tick.
        let check = goods_check();
        let mut books = Books::new(&shipped_with_liveness(false)).expect("the books open");

        books.produce(seller(), FOOD, 12).expect("a firm produces");
        assert_eq!(check(&books, 0), Ok(()));

        books
            .exchange(buyer(), seller(), FOOD, 4, Money::from_cents(420))
            .expect("a household buys four units");
        assert_eq!(check(&books, 0), Ok(()));
        assert_eq!(books.stock_of(buyer(), FOOD), Some(4));

        books.consume(buyer(), FOOD, 4).expect("and eats them");
        assert_eq!(check(&books, 0), Ok(()));

        assert_eq!(
            books.stock_of(buyer(), FOOD),
            Some(0),
            "the household's stock returns to zero within the tick"
        );
        assert_eq!(books.consumed(FOOD), 4);
        assert_eq!(identity(&books), 0);
        assert_eq!(books.goods_residual_units(), 0);
    }

    #[test]
    fn held_stock_across_a_tick_boundary_holds_the_same_identity() {
        // Phase 7 option B: the purchase ships, and consumption happens later.
        let check = goods_check();
        let mut books = Books::new(&shipped_with_liveness(false)).expect("the books open");

        books.produce(seller(), FOOD, 12).expect("a firm produces");
        books
            .exchange(buyer(), seller(), FOOD, 4, Money::from_cents(420))
            .expect("a household buys four units");
        assert_eq!(check(&books, 0), Ok(()));

        books.end_of_tick();

        assert_eq!(
            books.stock_of(buyer(), FOOD),
            Some(4),
            "the household's stock is non-zero across the tick boundary"
        );
        assert_eq!(check(&books, 1), Ok(()));

        books
            .consume(buyer(), FOOD, 4)
            .expect("it eats them on a later tick");
        assert_eq!(check(&books, 1), Ok(()));
        assert_eq!(books.stock_of(buyer(), FOOD), Some(0));
        assert_eq!(identity(&books), 0);
        assert_eq!(books.goods_residual_units(), 0);
    }

    #[test]
    fn both_consumption_models_use_the_same_check_and_the_same_accessors() {
        // The "one shape" claim, demonstrated rather than stated: one function
        // pointer, taken once from the table, evaluates both worlds, and the
        // identity is read through the same three accessors in each.
        let check = goods_check();

        let mut immediate = Books::new(&shipped_with_liveness(false)).expect("the books open");
        immediate.produce(seller(), FOOD, 9).expect("produces");
        immediate
            .exchange(buyer(), seller(), FOOD, 2, Money::from_cents(210))
            .expect("sells");
        immediate.consume(buyer(), FOOD, 2).expect("eaten at once");

        let mut held = Books::new(&shipped_with_liveness(false)).expect("the books open");
        held.produce(seller(), FOOD, 9).expect("produces");
        held.exchange(buyer(), seller(), FOOD, 2, Money::from_cents(210))
            .expect("sells");
        held.end_of_tick();

        assert_eq!(
            immediate.stock_of(buyer(), FOOD),
            Some(0),
            "the two worlds genuinely differ"
        );
        assert_eq!(held.stock_of(buyer(), FOOD), Some(2));

        for books in [&immediate, &held] {
            assert_eq!(check(books, 1), Ok(()));
            assert_eq!(
                books.produced(FOOD) - books.consumed(FOOD) - books.total_stock(FOOD),
                0,
                "the same formula over the same accessors, in both worlds"
            );
            assert_eq!(books.goods_residual_units(), 0);
        }
    }

    #[test]
    fn a_production_only_tick_passes_goods_conservation_and_fails_liveness() {
        // The pair of facts that keeps the two checks from being mistaken for
        // each other: the units are all accounted for, and nothing traded.
        let params = shipped_with_liveness(true);
        let mut books = Books::new(&params).expect("the books open");
        let checks = CheckSet::from_params(&params);

        books.produce(seller(), FOOD, 30).expect("a firm produces");

        assert_eq!(goods_check()(&books, 3), Ok(()));
        assert_eq!(books.transactions_this_tick(), 0);
        assert_eq!(
            checks.run(&books, 3),
            Err(Violation::Liveness {
                tick: 3,
                counted: 0,
                required: MINIMUM_TRANSACTIONS_PER_TICK,
            })
        );
    }

    #[test]
    fn localisation_names_the_first_break_and_not_a_later_one() {
        // Residuals cancel. A journal broken at #1, healed at #2 and broken
        // differently at #3 is exactly the shape that makes a search over
        // halves answer #3 when the correct answer is #1. Built by hand
        // because no public path can produce a non-zero residual — which is
        // the point of the rest of this phase.
        let line = |seq: u32, goods_residual_units: i64| Posting {
            seq,
            kind: PostingKind::Produce,
            debit: seller(),
            credit: seller(),
            debit_cents: 0,
            credit_cents: 0,
            good: FOOD,
            units_out: 0,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units,
        };
        let journal = [line(0, 0), line(1, -3), line(2, 0), line(3, 7)];

        assert_eq!(first_breaking_goods_posting(&journal), Some(line(1, -3)));
        assert_eq!(
            first_breaking_cash_posting(&journal),
            None,
            "a goods break is not a cash break, and the two scans do not share a residual"
        );
        assert_eq!(first_breaking_goods_posting(&[]), None);
    }
}
