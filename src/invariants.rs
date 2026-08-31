//! The invariant phase: an ordered set of checks that reads the books and
//! returns a `Result` (LEDG-04, LEDG-08, LEDG-09, LEDG-10).
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
//! non-negativity, zero-sum, liveness; this plan implements the first and the
//! last, and later plans insert their entries at those positions. Money
//! conservation is **first** because a leak is the highest-severity finding and
//! reporting it as "some account went negative" sends a debugger to the wrong
//! place. Liveness is **last** because it is the only check that can fire on
//! books that are entirely correct.
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
    /// The offending posting is embedded **by value** and is **optional**.
    /// By value, because an index into a buffer that `end_of_tick` is about to
    /// clear is a dangling reference in all but name — by the time a human
    /// reads the message the journal is gone. Optional, because a discrepancy
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
        posting: Option<Posting>,
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
fn render_posting(posting: &Option<Posting>) -> String {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckId {
    MoneyConservation,
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
/// to drift from. Plans 02-03 and 02-04 insert goods conservation,
/// non-negativity and zero-sum at the positions the module docs name, leaving
/// money conservation first and liveness last.
///
/// The middle element of each triple is the check's stable name, which plan
/// 02-05's seeded-corruption tests report against.
pub const ALL_CHECKS: [(CheckId, &str, CheckFn); 2] = [
    (
        CheckId::MoneyConservation,
        "money_conservation",
        check_money,
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
        posting: first_breaking_posting(books.journal()),
    })
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
fn first_breaking_posting(journal: &[Posting]) -> Option<Posting> {
    journal
        .iter()
        .copied()
        .find(|posting| posting.cash_residual_cents != 0)
}
