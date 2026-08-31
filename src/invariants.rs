//! The invariant phase: an ordered set of checks that reads the books and
//! returns a `Result` (LEDG-04, LEDG-05, LEDG-06, LEDG-07, LEDG-08, LEDG-09,
//! LEDG-10).
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
//! full order is money conservation, goods conservation, non-negativity,
//! zero-sum, liveness, and all five are implemented. Money conservation is
//! **first** because a leak is the highest-severity finding and reporting it as
//! "some account went negative" sends a debugger to the wrong place. Liveness is
//! **last** because it is the only check that can fire on books that are
//! entirely correct.
//!
//! **The table is complete by construction, not by promise.** [`CheckId::ALL`]
//! is the single source of truth for the sequence; the order test reads
//! [`ALL_CHECKS`] and compares against it rather than against a second
//! hand-written list, and a further test pattern-matches every [`CheckId`]
//! exhaustively, so adding an identifier without giving it a table entry is a
//! compile error rather than a silently smaller check set.
//!
//! **Two of the five need no aggregate at all.** Non-negativity is a property
//! of one balance and zero-sum is a property of one posting — which is why
//! zero-sum is checkable in a phase that has no economic notion of a sale. Both
//! are structural.
//!
//! The checks read the books through shared references and mutate nothing.
//! None of them draws from the random number generator: a draw here would shift
//! every downstream sub-stream and silently re-trajectory every run (CORE-04).
//!
//! Every amount here is an integer count of cents or of units. Nothing in this
//! module belongs to the float domain, and it names no type from that domain at
//! all.

use thiserror::Error;

use crate::books::{Books, Posting, PostingKind};
use crate::config::Params;
use crate::ids::{Account, GoodId};

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

    /// An account holds a negative amount of a quantity it cannot hold a
    /// negative amount of (LEDG-06).
    ///
    /// Names the account, the **column** and the value, because "something went
    /// negative" is not a finding. The posting is owned, boxed and optional on
    /// exactly the terms [`Violation::MoneyConservation`] sets out, and here the
    /// optional case is the interesting one: a balance driven negative *outside*
    /// the posting path leaves no posting naming it, and saying so is the honest
    /// answer.
    ///
    /// **Independent of [`Violation::MoneyConservation`].** A deficit moved from
    /// one account to another conserves the total perfectly and is invisible to
    /// the conservation check; it is this variant that reports it.
    #[error(
        "tick {tick}: {account} holds {value} in {field}, which is not a quantity \
         an account can hold; {}",
        render_posting(.posting)
    )]
    Negative {
        tick: u32,
        account: Account,
        field: NegativeField,
        value: i64,
        posting: Option<Box<Posting>>,
    },

    /// A posting is not well formed for its kind (LEDG-07): its two cash
    /// amounts or its two unit amounts do not stand in the relation the kind
    /// requires.
    ///
    /// **The posting is not optional here, and that is structural rather than a
    /// choice.** This check is evaluated one posting at a time, so the posting
    /// it failed on is by construction always known. Boxed for the same size
    /// reason the other variants' are.
    ///
    /// `detail` names exactly what disagreed, as a small comparable value rather
    /// than a formatted string, so a test can assert the finding by value.
    #[error("tick {tick}: zero-sum broken — {detail}; offending posting {posting}")]
    ZeroSum {
        tick: u32,
        posting: Box<Posting>,
        detail: ZeroSumDetail,
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

/// Which column of an account went negative (LEDG-06).
///
/// **Two variants, and exactly two.** LEDG-06 names three quantities — cash,
/// inventory and headcount — but only two of them can be negative. A payroll is
/// an unsigned count, so a negative headcount is not representable and there is
/// nothing for a third variant to report. See [`check_non_negative`], which
/// documents the same fact from the check's side.
///
/// A small copyable enum rather than a string, so a test asserts the column by
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NegativeField {
    /// Cents held by an account.
    Cash,
    /// Units of a good held by an account.
    Stock,
}

impl std::fmt::Display for NegativeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let column = match self {
            NegativeField::Cash => "cash",
            NegativeField::Stock => "stock",
        };
        f.write_str(column)
    }
}

/// Exactly what was wrong with a posting's shape (LEDG-07).
///
/// Every variant carries the offending numbers or identities and nothing else:
/// small, copyable, comparable. That is what lets a test assert the finding as a
/// whole value. A formatted string here would force a substring match, which
/// passes when the wrong check fired, when the tick is wrong and when the named
/// agent is wrong.
///
/// The variants exist because a [`Posting`] carries **two** cash amounts and
/// **two** unit amounts. With one of each, an over-credit would not be
/// expressible as data and this whole enum would be unreachable — the check
/// would be a structural tautology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroSumDetail {
    /// The cents that left the debit account are not the cents that arrived at
    /// the credit account.
    CashLegsDiffer { debit_cents: i64, credit_cents: i64 },
    /// The units that left are not the units that arrived, in a way the
    /// posting's kind does not permit.
    UnitLegsDiffer { units_out: i64, units_in: i64 },
    /// Cash on a posting whose kind moves only units.
    CashOnAGoodsOnlyPosting { debit_cents: i64, credit_cents: i64 },
    /// Units on a posting whose kind moves only cash.
    UnitsOnACashOnlyPosting { units_out: i64, units_in: i64 },
    /// A two-party kind — a transfer or an exchange — names one account on both
    /// legs. Nothing changed hands, whatever the amounts say.
    SelfDealing { account: Account },
    /// A one-party kind — a production, a consumption or an endowment — names
    /// two different accounts, so it is not the single-account posting its kind
    /// claims to be.
    SplitParties { debit: Account, credit: Account },
    /// An exchange with an empty leg. An exchange moves cash one way and units
    /// the other; one that moves nothing on either side is a different shape,
    /// and it would count towards the liveness minimum while nothing changed
    /// hands. [`crate::books::Books::exchange`] refuses it, so no public path
    /// can record one.
    EmptyExchange { cents: i64, units: i64 },
    /// An endowment carries a debit leg. Its counterparty is outside the books
    /// by definition, so nothing can have left an account inside them.
    EndowmentHasADebitLeg { debit_cents: i64, units_out: i64 },
}

impl std::fmt::Display for ZeroSumDetail {
    /// Integer amounts and integer identities only. No path, host name,
    /// wall-clock reading or process id can reach a halt message (TICK-06).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZeroSumDetail::CashLegsDiffer {
                debit_cents,
                credit_cents,
            } => write!(
                f,
                "the cash legs disagree: {debit_cents} cents left the debit account \
                 but {credit_cents} cents arrived at the credit account"
            ),
            ZeroSumDetail::UnitLegsDiffer {
                units_out,
                units_in,
            } => write!(
                f,
                "the unit legs disagree: {units_out} units left but {units_in} arrived"
            ),
            ZeroSumDetail::CashOnAGoodsOnlyPosting {
                debit_cents,
                credit_cents,
            } => write!(
                f,
                "cash on a posting that moves only units: {debit_cents} out, \
                 {credit_cents} in"
            ),
            ZeroSumDetail::UnitsOnACashOnlyPosting {
                units_out,
                units_in,
            } => write!(
                f,
                "units on a posting that moves only cash: {units_out} out, {units_in} in"
            ),
            ZeroSumDetail::SelfDealing { account } => write!(
                f,
                "a two-party posting names {account} on both legs, so nothing changed hands"
            ),
            ZeroSumDetail::SplitParties { debit, credit } => write!(
                f,
                "a one-party posting names {debit} on the debit leg and {credit} on \
                 the credit leg"
            ),
            ZeroSumDetail::EmptyExchange { cents, units } => write!(
                f,
                "an exchange with an empty leg: {cents} cents against {units} units"
            ),
            ZeroSumDetail::EndowmentHasADebitLeg {
                debit_cents,
                units_out,
            } => write!(
                f,
                "an endowment carries a debit leg: {debit_cents} cents and {units_out} \
                 units left an account inside the books"
            ),
        }
    }
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
    NonNegative,
    ZeroSum,
    Liveness,
}

impl CheckId {
    /// Every check identifier, in the order the checks run.
    ///
    /// **The single source of truth for the sequence.** The order test reads
    /// [`ALL_CHECKS`] and compares it against this constant element for
    /// element; there is no second hand-written list for either to drift from.
    /// A new identifier that is added to the enum but not to this constant is
    /// caught by the exhaustive match in the order tests, which stops
    /// compiling.
    pub const ALL: [CheckId; 5] = [
        CheckId::MoneyConservation,
        CheckId::GoodsConservation,
        CheckId::NonNegative,
        CheckId::ZeroSum,
        CheckId::Liveness,
    ];
}

/// A check: the books and the tick in, a violation or nothing out.
///
/// A plain function pointer, not a trait object and not a closure. The checks
/// are a fixed table known at compile time, and nothing here needs to capture.
pub type CheckFn = fn(&Books, u32) -> Result<(), Violation>;

/// The full, ordered table: every check, in the order it runs.
///
/// The order test reads this array and compares it against [`CheckId::ALL`];
/// there is no second hand-written list for either to drift from.
///
/// The positions are the contract, not a convenience. Money conservation is
/// **first** because a leak is the highest-severity finding. Goods conservation
/// is **second** for the same reason one column down. Non-negativity is
/// **third**, because an account holding a negative amount is a finding about
/// one account rather than about the books as a whole. Zero-sum is **fourth**,
/// because a malformed posting will usually already have shown up as a broken
/// conservation identity, and the identity is the more useful report of the two.
/// Liveness is **last**, because it is the only check that can fire on books
/// that are entirely correct.
///
/// The second element of each triple is the check's stable name, which plan
/// 02-05's seeded-corruption tests report against and Phase 3 logs. The names
/// are the snake-case spelling of their identifiers, and an order test asserts
/// exactly that rather than leaving the two spellings free to diverge.
pub const ALL_CHECKS: [(CheckId, &str, CheckFn); 5] = [
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
    (CheckId::NonNegative, "non_negative", check_non_negative),
    (CheckId::ZeroSum, "zero_sum", check_zero_sum),
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

/// Non-negativity (LEDG-06): no account holds a negative amount of anything.
///
/// **Independent of [`check_money`], and that independence is the point.** A
/// deficit moved from one account to another leaves the total intact, so the
/// conservation check passes and only this one fires. Ordering it third — after
/// the two conservation checks and before zero-sum — means a genuine leak is
/// still reported as a leak rather than as "some account went negative".
///
/// **The walk order is part of the contract.** Accounts are visited in the
/// order [`crate::books::Books::accounts`] yields them — households by
/// ascending index, then firm slots by ascending slot — and for each account
/// cash is read before stock. Two accounts can be negative at the same time,
/// and without a fixed order the answer to "which one" would vary between runs,
/// making this check's own negative test flaky in a way indistinguishable from
/// a real failure. The order rests on the derived total order `src/ids.rs`
/// already carries.
///
/// **Headcount has no arm here, and that is a fact about the type rather than
/// an omission.** LEDG-06 names cash, inventory *and* headcount. The books own
/// all three, but a payroll is an unsigned count: a negative headcount is not
/// representable, so a loop over the payrolls would be a check that can never
/// fire. Documenting a type-level guarantee is the honest form of that claim.
/// Writing the unreachable loop is the vacuous one — it would report as a
/// passing check, look identical in a coverage number, and hold nothing at all.
///
/// Localisation is a forward linear scan for the first posting naming the
/// offending account on either leg. If no posting names it, the violation says
/// so rather than pointing at the nearest one: a balance driven negative outside
/// the posting path is exactly the case that leaves no posting to blame, and a
/// plausible wrong answer sends a debugger somewhere the defect is not.
fn check_non_negative(books: &Books, tick: u32) -> Result<(), Violation> {
    for account in books.accounts() {
        if let Some(cash) = books.cash_of(account)
            && cash.cents() < 0
        {
            return Err(negative(
                books,
                tick,
                account,
                NegativeField::Cash,
                cash.cents(),
            ));
        }

        for &good in books.goods() {
            if let Some(units) = books.stock_of(account, good)
                && units < 0
            {
                return Err(negative(books, tick, account, NegativeField::Stock, units));
            }
        }
    }

    Ok(())
}

/// Build the violation, attaching the first posting that names the account.
fn negative(
    books: &Books,
    tick: u32,
    account: Account,
    field: NegativeField,
    value: i64,
) -> Violation {
    Violation::Negative {
        tick,
        account,
        field,
        value,
        posting: first_posting_naming(books.journal(), account).map(Box::new),
    }
}

/// Zero-sum (LEDG-07): every posting is well formed for its kind.
///
/// **Checked one posting at a time, with no aggregate anywhere.** That is
/// possible only because a [`Posting`] carries two cash amounts and two unit
/// amounts: with one of each, an over-credit would be inexpressible as data and
/// this check would be a structural tautology that could never fire.
///
/// **This phase has no economic notion of a sale and does not need one.** The
/// property is structural — the cash leg and the units leg name the same pair of
/// accounts in opposite directions, in the amounts the kind requires — which is
/// exactly why it is well defined here, four phases before a goods market
/// exists. Its negative test uses a synthesised posting for the same reason:
/// every public path already refuses the shapes it looks for.
///
/// Walks the journal in order and reports the first malformed posting, so the
/// answer does not depend on which of several malformed postings a search
/// happened to reach first.
fn check_zero_sum(books: &Books, tick: u32) -> Result<(), Violation> {
    for posting in books.journal() {
        if let Err(detail) = well_formed(posting) {
            return Err(Violation::ZeroSum {
                tick,
                posting: Box::new(*posting),
                detail,
            });
        }
    }

    Ok(())
}

/// The well-formedness rule for one posting, by kind.
///
/// Separated from [`check_zero_sum`] so the rule can be driven by a synthesised
/// posting: no public operation can produce a malformed one, which is the point
/// of the rest of this phase, and a check that could only be tested through
/// books nobody can corrupt would go untested until plan 02-05.
///
/// The rules, and what each kind promises:
///
/// - **Transfer** — cash between two *distinct* accounts, in equal amounts,
///   and no units.
/// - **Exchange** — equal cash one way and equal units the other, between two
///   *distinct* accounts, with neither leg empty.
/// - **Produce** / **Consume** — one account on both legs, no cash, and units in
///   exactly one direction: a production may not also release units and a
///   consumption may not also receive them.
/// - **Endow** — one account on both legs and no debit leg at all, because an
///   endowment's counterparty is outside the books.
///
/// Within a kind the clauses are evaluated in the order written, so the detail
/// reported for a posting that breaks two of them is fixed rather than
/// incidental.
fn well_formed(posting: &Posting) -> Result<(), ZeroSumDetail> {
    match posting.kind {
        PostingKind::Transfer => {
            two_party(posting)?;
            if posting.units_out != 0 || posting.units_in != 0 {
                return Err(ZeroSumDetail::UnitsOnACashOnlyPosting {
                    units_out: posting.units_out,
                    units_in: posting.units_in,
                });
            }
            if posting.debit_cents != posting.credit_cents {
                return Err(ZeroSumDetail::CashLegsDiffer {
                    debit_cents: posting.debit_cents,
                    credit_cents: posting.credit_cents,
                });
            }
        }
        PostingKind::Exchange => {
            two_party(posting)?;
            if posting.debit_cents != posting.credit_cents {
                return Err(ZeroSumDetail::CashLegsDiffer {
                    debit_cents: posting.debit_cents,
                    credit_cents: posting.credit_cents,
                });
            }
            if posting.units_out != posting.units_in {
                return Err(ZeroSumDetail::UnitLegsDiffer {
                    units_out: posting.units_out,
                    units_in: posting.units_in,
                });
            }
            if posting.debit_cents == 0 || posting.units_out == 0 {
                return Err(ZeroSumDetail::EmptyExchange {
                    cents: posting.debit_cents,
                    units: posting.units_out,
                });
            }
        }
        PostingKind::Produce => {
            one_party(posting)?;
            no_cash(posting)?;
            if posting.units_out != 0 {
                return Err(ZeroSumDetail::UnitLegsDiffer {
                    units_out: posting.units_out,
                    units_in: posting.units_in,
                });
            }
        }
        PostingKind::Consume => {
            one_party(posting)?;
            no_cash(posting)?;
            if posting.units_in != 0 {
                return Err(ZeroSumDetail::UnitLegsDiffer {
                    units_out: posting.units_out,
                    units_in: posting.units_in,
                });
            }
        }
        PostingKind::Endow => {
            one_party(posting)?;
            if posting.debit_cents != 0 || posting.units_out != 0 {
                return Err(ZeroSumDetail::EndowmentHasADebitLeg {
                    debit_cents: posting.debit_cents,
                    units_out: posting.units_out,
                });
            }
        }
    }

    Ok(())
}

/// A kind that moves something between two parties must name two of them.
fn two_party(posting: &Posting) -> Result<(), ZeroSumDetail> {
    if posting.debit == posting.credit {
        return Err(ZeroSumDetail::SelfDealing {
            account: posting.debit,
        });
    }
    Ok(())
}

/// A kind that acts on one account must name the same one on both legs.
fn one_party(posting: &Posting) -> Result<(), ZeroSumDetail> {
    if posting.debit != posting.credit {
        return Err(ZeroSumDetail::SplitParties {
            debit: posting.debit,
            credit: posting.credit,
        });
    }
    Ok(())
}

/// A kind that moves only units must carry no cash on either leg.
fn no_cash(posting: &Posting) -> Result<(), ZeroSumDetail> {
    if posting.debit_cents != 0 || posting.credit_cents != 0 {
        return Err(ZeroSumDetail::CashOnAGoodsOnlyPosting {
            debit_cents: posting.debit_cents,
            credit_cents: posting.credit_cents,
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

/// The first posting in the tick that names `account` on either leg, if there is
/// one.
///
/// A forward linear scan, and it must stay one, for the reason its two siblings
/// above give: the earliest posting touching an account is the one a debugger
/// wants, and a search that discards half the journal cannot promise to find it.
///
/// Returning `None` is a real answer rather than a failure. A balance driven
/// negative outside the posting path genuinely has no posting to blame, and
/// [`Violation::Negative`] renders that case in those terms — a synthetic
/// posting in a halt message is a lie a future reader will chase.
fn first_posting_naming(journal: &[Posting], account: Account) -> Option<Posting> {
    journal
        .iter()
        .copied()
        .find(|posting| posting.debit == account || posting.credit == account)
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
                CheckId::NonNegative,
                CheckId::ZeroSum,
                CheckId::Liveness
            ],
            "with the gate on, liveness runs and runs last"
        );
        assert_eq!(
            CheckSet::from_params(&shipped_with_liveness(false)).active_ids(),
            vec![
                CheckId::MoneyConservation,
                CheckId::GoodsConservation,
                CheckId::NonNegative,
                CheckId::ZeroSum
            ],
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

/// The check table's completeness and order, asserted from the table itself.
///
/// Named `order` so that an `invariants::order` module-path filter selects
/// exactly these. Three claims, and they are deliberately different claims:
/// the table runs the documented sequence, an identifier cannot be added
/// without a table entry, and the names in the table are the snake-case
/// spelling of their identifiers.
#[cfg(test)]
mod order {
    use super::*;

    /// Where each identifier sits in the table.
    ///
    /// **Exhaustive on purpose.** A new [`CheckId`] variant stops this function
    /// compiling until it is given a position, and the assertions below then
    /// force it into both [`CheckId::ALL`] and [`ALL_CHECKS`]. That is what
    /// makes "a check cannot be silently dropped from the set" a compile-time
    /// property rather than a promise in a comment.
    fn documented_position(id: CheckId) -> usize {
        match id {
            CheckId::MoneyConservation => 0,
            CheckId::GoodsConservation => 1,
            CheckId::NonNegative => 2,
            CheckId::ZeroSum => 3,
            CheckId::Liveness => 4,
        }
    }

    /// The snake-case spelling of an identifier, derived rather than written
    /// out — a second hand-written list of names would be the very thing this
    /// test exists to catch.
    fn snake_case(id: CheckId) -> String {
        let spelled = format!("{id:?}");
        let mut out = String::with_capacity(spelled.len() + 2);
        for (index, character) in spelled.char_indices() {
            if index != 0 && character.is_ascii_uppercase() {
                out.push('_');
            }
            out.push(character.to_ascii_lowercase());
        }
        out
    }

    #[test]
    fn the_table_runs_the_documented_sequence() {
        // Read out of ALL_CHECKS and compared against the constant, element for
        // element. The constant is the single source of truth; there is no
        // second hand-written list here for either to drift from.
        let sequence: Vec<CheckId> = ALL_CHECKS.iter().map(|(id, _, _)| *id).collect();

        assert_eq!(sequence, CheckId::ALL.to_vec());
        assert_eq!(
            sequence.first().copied(),
            Some(CheckId::MoneyConservation),
            "money conservation is first: a leak is the highest-severity finding"
        );
        assert_eq!(
            sequence.last().copied(),
            Some(CheckId::Liveness),
            "liveness is last: it is the only check that can fire on correct books"
        );
    }

    #[test]
    fn an_identifier_cannot_exist_without_a_table_entry() {
        assert_eq!(
            ALL_CHECKS.len(),
            CheckId::ALL.len(),
            "every identifier has exactly one entry and the table has no extras"
        );

        for &id in &CheckId::ALL {
            let position = documented_position(id);
            assert_eq!(
                ALL_CHECKS[position].0, id,
                "{id:?} is not at position {position} of the table"
            );
        }

        // The identifiers are declared in run order, so the derived Ord agrees
        // with the table rather than quietly contradicting it.
        let mut sorted = CheckId::ALL.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, CheckId::ALL.to_vec());
    }

    #[test]
    fn the_names_are_distinct_and_spell_their_identifiers() {
        // Phase 3 logs these names and plan 02-05 reports against them, so two
        // checks sharing one name would make a log ambiguous about which check
        // fired.
        let mut names: Vec<&str> = ALL_CHECKS.iter().map(|(_, name, _)| *name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "two checks share a name");

        for (id, name, _) in ALL_CHECKS {
            assert_eq!(
                name,
                snake_case(id),
                "the table name and the identifier have drifted apart"
            );
        }
    }
}

/// Non-negativity (LEDG-06), at unit granularity.
///
/// Named `non_negative` so that an `invariants::non_negative` module-path
/// filter selects exactly these.
///
/// The negative direction is reached through the **entirely public API** and
/// with no fault injection: a configuration whose per-agent endowments still sum
/// to the money stock, but in which one side of the population is endowed a
/// negative amount, opens books that conserve perfectly and hold negative
/// balances. That is the research's "driven-negative balance with the total
/// intact" case, and it is what proves this check and `check_money` are
/// genuinely independent.
///
/// Violations are asserted by whole-value equality, never by matching a
/// substring of a rendered message.
#[cfg(test)]
mod non_negative {
    use super::*;
    use std::path::Path;

    use crate::ids::{FirmId, FirmSlot, HouseholdId};
    use crate::money::Money;

    /// Where non-negativity sits in the table. Third, read from the array
    /// rather than hand-written so it cannot drift from the run order.
    const NON_NEGATIVE_POSITION: usize = 2;

    /// The one good v1 carries.
    const FOOD: GoodId = GoodId(0);

    fn shipped_with_liveness(enabled: bool) -> Params {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
        let (mut params, _hash) = crate::config::load(&path).expect("the configuration loads");
        params.invariants.liveness_enabled = enabled;
        params
    }

    /// Parameters whose endowment still sums to the configured stock, but in
    /// which every household opens `deficit_cents` below zero and the firms
    /// carry the difference.
    ///
    /// The point of the shape: the books conserve exactly, so `check_money`
    /// passes and only this check fires.
    fn households_endowed_negative(deficit_cents: i64) -> Params {
        let mut params = shipped_with_liveness(false);
        let households = i64::from(params.sim.households);
        let firms = i64::from(params.sim.firms);

        params.household.initial_liquidity_cents = -deficit_cents;
        let owed = params.money.total_money_cents + households * deficit_cents;
        assert_eq!(
            owed % firms,
            0,
            "choose a deficit that divides evenly across the firm slots"
        );
        params.firm.initial_liquidity_cents = owed / firms;
        params
    }

    /// The mirror: every firm slot opens below zero and the households carry the
    /// difference. Used to prove the walk reaches the firms only after finding
    /// every household clean.
    fn firms_endowed_negative(deficit_cents: i64) -> Params {
        let mut params = shipped_with_liveness(false);
        let households = i64::from(params.sim.households);
        let firms = i64::from(params.sim.firms);

        params.firm.initial_liquidity_cents = -deficit_cents;
        let owed = params.money.total_money_cents + firms * deficit_cents;
        assert_eq!(owed % households, 0, "choose a deficit that divides evenly");
        params.household.initial_liquidity_cents = owed / households;
        params
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

    /// The check itself, taken from the table by position — so every test below
    /// runs the function the tick loop runs, and the position it occupies is
    /// asserted rather than assumed.
    fn non_negative_check() -> CheckFn {
        let (id, name, check) = ALL_CHECKS[NON_NEGATIVE_POSITION];
        assert_eq!(id, CheckId::NonNegative, "non-negativity is third");
        assert_eq!(name, "non_negative");
        check
    }

    #[test]
    fn a_healthy_economy_holds_no_negative_quantity() {
        // The positive direction, and the thing that stops the check from being
        // permanently red.
        let check = non_negative_check();
        let mut books = Books::new(&shipped_with_liveness(false)).expect("the books open");
        assert_eq!(check(&books, 0), Ok(()));

        books.produce(firm(0), FOOD, 40).expect("a firm produces");
        books
            .exchange(household(0), firm(0), FOOD, 3, Money::from_cents(300))
            .expect("a household buys");
        books.consume(household(0), FOOD, 3).expect("and eats");
        books
            .transfer(household(1), firm(2), Money::from_cents(1_000))
            .expect("and another pays");

        assert_eq!(check(&books, 5), Ok(()));
    }

    #[test]
    fn a_negative_balance_with_the_total_intact_is_reported_and_conservation_is_not() {
        // The case that proves the two checks are independent: the books hold
        // exactly the configured stock and the journal residual is zero, so
        // money conservation passes — and the deficit is still a defect.
        let params = households_endowed_negative(100);
        let books = Books::new(&params).expect("the endowment still sums to the stock");

        assert_eq!(
            books.total_money().cents(),
            params.money.total_money_cents,
            "the total is intact, which is the whole point of this shape"
        );
        assert_eq!(check_money(&books, 6), Ok(()));
        assert_eq!(check_goods(&books, 6), Ok(()));

        assert_eq!(
            non_negative_check()(&books, 6),
            Err(Violation::Negative {
                tick: 6,
                account: household(0),
                field: NegativeField::Cash,
                value: -100,
                posting: None,
            }),
            "the first household in the walk, its column and its value"
        );
    }

    #[test]
    fn the_check_set_reports_the_deficit_before_it_reports_liveness() {
        // Position three beats position five: books that opened negative have
        // also traded nothing, and the diagnostically useful finding is the
        // deficit rather than the silence.
        let mut params = households_endowed_negative(100);
        params.invariants.liveness_enabled = true;
        let books = Books::new(&params).expect("the endowment still sums to the stock");
        let checks = CheckSet::from_params(&params);

        assert_eq!(
            books.transactions_this_tick(),
            0,
            "liveness would also fire"
        );
        assert_eq!(
            checks.run(&books, 0),
            Err(Violation::Negative {
                tick: 0,
                account: household(0),
                field: NegativeField::Cash,
                value: -100,
                posting: None,
            })
        );
    }

    #[test]
    fn households_are_walked_before_firm_slots_and_slots_in_ascending_order() {
        // Two accounts can be negative at once, so which one is reported has to
        // be a fixed fact rather than an incidental one. Every firm slot is
        // negative here and every household is clean, which is only reportable
        // as slot 0 if the walk order holds.
        let params = firms_endowed_negative(500);
        let books = Books::new(&params).expect("the endowment still sums to the stock");

        assert_eq!(check_money(&books, 2), Ok(()));
        assert_eq!(
            books.cash_of(household(0)).map(|cash| cash.cents() >= 0),
            Some(true),
            "the households are clean, so the walk must pass through them first"
        );
        assert_eq!(
            non_negative_check()(&books, 2),
            Err(Violation::Negative {
                tick: 2,
                account: firm(0),
                field: NegativeField::Cash,
                value: -500,
                posting: None,
            })
        );
    }

    #[test]
    fn the_violation_names_the_first_posting_touching_the_account_or_says_there_is_none() {
        // Localisation, in both of its directions. A posting naming the account
        // is attached; a deficit no posting describes reports that in those
        // terms rather than pointing at the nearest posting, which would send a
        // debugger somewhere the defect is not.
        let params = households_endowed_negative(100);
        let mut books = Books::new(&params).expect("the endowment still sums to the stock");

        assert_eq!(
            non_negative_check()(&books, 1),
            Err(Violation::Negative {
                tick: 1,
                account: household(0),
                field: NegativeField::Cash,
                value: -100,
                posting: None,
            }),
            "an empty journal names no posting, and the message says exactly that"
        );

        // A posting that touches the offending account, but does not lift it
        // out of deficit.
        books
            .transfer(firm(0), household(0), Money::from_cents(10))
            .expect("a firm can pay ten cents");
        let expected = books.journal().first().copied().expect("one posting");

        assert_eq!(
            non_negative_check()(&books, 1),
            Err(Violation::Negative {
                tick: 1,
                account: household(0),
                field: NegativeField::Cash,
                value: -90,
                posting: Some(Box::new(expected)),
            })
        );
        assert_eq!(
            first_posting_naming(books.journal(), household(7)),
            None,
            "a posting naming another account is not this account's posting"
        );
    }

    #[test]
    fn the_message_names_the_account_the_column_and_the_value() {
        // The message contract (LEDG-09) is a different claim from the value,
        // and this is the one test in this module that reads the rendered form.
        let violation = Violation::Negative {
            tick: 12,
            account: firm(3),
            field: NegativeField::Stock,
            value: -7,
            posting: None,
        };
        let rendered = violation.to_string();

        assert!(rendered.contains("tick 12"), "{rendered}");
        assert!(rendered.contains("firm:3:0"), "{rendered}");
        assert!(rendered.contains("stock"), "{rendered}");
        assert!(rendered.contains("-7"), "{rendered}");
        assert_eq!(NegativeField::Cash.to_string(), "cash");
        assert_eq!(NegativeField::Stock.to_string(), "stock");
    }
}

/// Zero-sum (LEDG-07), at unit granularity.
///
/// Named `zero_sum` so that an `invariants::zero_sum` module-path filter selects
/// exactly these.
///
/// The positive direction runs the real check over a real journal built through
/// the public API. The negative direction drives [`well_formed`] with
/// **synthesised** postings, and that is not a shortcut: no public operation can
/// produce a malformed posting — which is the point of the rest of this phase —
/// so a synthesised one is the only way to prove the rule discriminates at all.
/// Plan 02-05 drives [`check_zero_sum`] end to end from a corrupted ledger.
///
/// Every finding is asserted as a whole [`ZeroSumDetail`] value, never as a
/// substring of a rendered message.
#[cfg(test)]
mod zero_sum {
    use super::*;
    use std::path::Path;

    use crate::ids::{FirmId, FirmSlot, HouseholdId};
    use crate::money::Money;

    /// Where zero-sum sits in the table. Fourth, read from the array rather
    /// than hand-written so it cannot drift from the run order.
    const ZERO_SUM_POSITION: usize = 3;

    /// The one good v1 carries.
    const FOOD: GoodId = GoodId(0);

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

    fn zero_sum_check() -> CheckFn {
        let (id, name, check) = ALL_CHECKS[ZERO_SUM_POSITION];
        assert_eq!(id, CheckId::ZeroSum, "zero-sum is fourth");
        assert_eq!(name, "zero_sum");
        check
    }

    /// A posting of `kind` with every leg empty, to be shaped by the caller.
    ///
    /// Synthesised because no public operation can record a malformed posting.
    fn line(kind: PostingKind) -> Posting {
        Posting {
            seq: 0,
            kind,
            debit: buyer(),
            credit: seller(),
            debit_cents: 0,
            credit_cents: 0,
            good: FOOD,
            units_out: 0,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        }
    }

    /// The same, with both legs naming one account — the shape the one-party
    /// kinds require.
    fn one_sided(kind: PostingKind) -> Posting {
        Posting {
            credit: buyer(),
            ..line(kind)
        }
    }

    #[test]
    fn every_posting_a_real_run_records_is_well_formed() {
        // The positive direction, over the real check and a real journal: one
        // of each kind the public API can produce, in one tick.
        let check = zero_sum_check();
        let mut books = Books::new(&shipped_with_liveness(false)).expect("the books open");
        assert_eq!(check(&books, 0), Ok(()));

        books.produce(seller(), FOOD, 20).expect("a firm produces");
        books
            .transfer(buyer(), seller(), Money::from_cents(15))
            .expect("a household pays");
        books
            .exchange(buyer(), seller(), FOOD, 4, Money::from_cents(400))
            .expect("and buys four units");
        books.consume(buyer(), FOOD, 4).expect("and eats them");

        assert_eq!(books.journal().len(), 4, "one posting of each kind");
        assert_eq!(check(&books, 0), Ok(()));
        for posting in books.journal() {
            assert_eq!(well_formed(posting), Ok(()), "{posting}");
        }
    }

    #[test]
    fn the_endowment_shape_the_constructor_records_is_well_formed() {
        // Construction clears the journal before tick 0, so the real endowment
        // postings are not observable here; their shape is.
        let cash = Posting {
            credit_cents: 5_000,
            ..one_sided(PostingKind::Endow)
        };
        let units = Posting {
            units_in: 165,
            ..one_sided(PostingKind::Endow)
        };

        assert_eq!(well_formed(&cash), Ok(()));
        assert_eq!(well_formed(&units), Ok(()));
    }

    #[test]
    fn each_malformed_shape_is_named_exactly() {
        // One case per detail variant, asserted by value. A substring match on
        // a message would pass when the wrong shape was detected.
        let cases = [
            (
                "a transfer whose cash legs disagree",
                Posting {
                    debit_cents: 100,
                    credit_cents: 101,
                    ..line(PostingKind::Transfer)
                },
                ZeroSumDetail::CashLegsDiffer {
                    debit_cents: 100,
                    credit_cents: 101,
                },
            ),
            (
                "an exchange whose unit legs disagree",
                Posting {
                    debit_cents: 100,
                    credit_cents: 100,
                    units_out: 3,
                    units_in: 2,
                    ..line(PostingKind::Exchange)
                },
                ZeroSumDetail::UnitLegsDiffer {
                    units_out: 3,
                    units_in: 2,
                },
            ),
            (
                "cash on a production, which moves only units",
                Posting {
                    debit_cents: 0,
                    credit_cents: 40,
                    units_in: 5,
                    ..one_sided(PostingKind::Produce)
                },
                ZeroSumDetail::CashOnAGoodsOnlyPosting {
                    debit_cents: 0,
                    credit_cents: 40,
                },
            ),
            (
                "units on a transfer, which moves only cash",
                Posting {
                    debit_cents: 100,
                    credit_cents: 100,
                    units_out: 2,
                    units_in: 2,
                    ..line(PostingKind::Transfer)
                },
                ZeroSumDetail::UnitsOnACashOnlyPosting {
                    units_out: 2,
                    units_in: 2,
                },
            ),
            (
                "a transfer naming one account on both legs",
                Posting {
                    debit_cents: 100,
                    credit_cents: 100,
                    ..one_sided(PostingKind::Transfer)
                },
                ZeroSumDetail::SelfDealing { account: buyer() },
            ),
            (
                "a consumption naming two accounts",
                Posting {
                    units_out: 3,
                    ..line(PostingKind::Consume)
                },
                ZeroSumDetail::SplitParties {
                    debit: buyer(),
                    credit: seller(),
                },
            ),
            (
                "an exchange with an empty leg",
                Posting {
                    debit_cents: 250,
                    credit_cents: 250,
                    ..line(PostingKind::Exchange)
                },
                ZeroSumDetail::EmptyExchange {
                    cents: 250,
                    units: 0,
                },
            ),
            (
                "an endowment carrying a debit leg",
                Posting {
                    debit_cents: 70,
                    credit_cents: 70,
                    ..one_sided(PostingKind::Endow)
                },
                ZeroSumDetail::EndowmentHasADebitLeg {
                    debit_cents: 70,
                    units_out: 0,
                },
            ),
            (
                "a production that also releases units",
                Posting {
                    units_out: 4,
                    units_in: 4,
                    ..one_sided(PostingKind::Produce)
                },
                ZeroSumDetail::UnitLegsDiffer {
                    units_out: 4,
                    units_in: 4,
                },
            ),
        ];

        for (what, posting, expected) in cases {
            assert_eq!(well_formed(&posting), Err(expected), "{what}: {posting}");
        }
    }

    #[test]
    fn an_over_credited_exchange_is_expressible_as_data_and_is_caught() {
        // The whole reason a Posting carries TWO cash amounts and TWO unit
        // amounts. With one of each this posting could not be written down, the
        // check would be a structural tautology, and an over-credit would be
        // detectable only as a broken conservation total one layer away.
        let over_credited = Posting {
            debit_cents: 500,
            credit_cents: 501,
            units_out: 2,
            units_in: 2,
            ..line(PostingKind::Exchange)
        };

        assert_eq!(
            well_formed(&over_credited),
            Err(ZeroSumDetail::CashLegsDiffer {
                debit_cents: 500,
                credit_cents: 501,
            })
        );
    }

    #[test]
    fn the_public_api_refuses_the_shapes_this_check_looks_for() {
        // Refusing at the operation boundary and reporting from the check are
        // two different guarantees, and both are wanted: the first means no run
        // records such a posting, the second means one that somehow appears is
        // named rather than passing.
        let mut books = Books::new(&shipped_with_liveness(false)).expect("the books open");
        books.produce(seller(), FOOD, 10).expect("a firm produces");

        assert!(
            books
                .exchange(buyer(), seller(), FOOD, 0, Money::from_cents(250))
                .is_err(),
            "an exchange with an empty units leg is refused"
        );
        assert!(
            books
                .exchange(buyer(), buyer(), FOOD, 1, Money::from_cents(1))
                .is_err(),
            "an exchange naming one account on both legs is refused"
        );
        assert!(
            books
                .transfer(buyer(), buyer(), Money::from_cents(1))
                .is_err(),
            "a self-dealing transfer is refused"
        );

        assert_eq!(
            books.journal().len(),
            1,
            "the refusals wrote nothing: only the production is on the journal"
        );
        assert_eq!(zero_sum_check()(&books, 0), Ok(()));
    }

    #[test]
    fn the_violation_names_the_posting_and_what_disagreed() {
        // The message contract (LEDG-09), and the one test in this module that
        // reads the rendered form. The posting is not optional on this variant:
        // the check is per-posting, so the offending one is always known.
        let posting = Posting {
            seq: 3,
            debit_cents: 100,
            credit_cents: 101,
            ..line(PostingKind::Transfer)
        };
        let violation = Violation::ZeroSum {
            tick: 9,
            posting: Box::new(posting),
            detail: ZeroSumDetail::CashLegsDiffer {
                debit_cents: 100,
                credit_cents: 101,
            },
        };
        let rendered = violation.to_string();

        assert!(rendered.contains("tick 9"), "{rendered}");
        assert!(rendered.contains("household:0"), "{rendered}");
        assert!(rendered.contains("firm:0:0"), "{rendered}");
        assert!(rendered.contains("#3"), "{rendered}");
        assert!(rendered.contains("100"), "{rendered}");
        assert!(rendered.contains("101"), "{rendered}");
    }
}
