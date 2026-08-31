//! The books: the one place a cent exists, and the one place a cent moves
//! (LEDG-01, LEDG-02, LEDG-03, LEDG-04, LEDG-05, LEDG-06, LEDG-07, LEDG-09).
//!
//! Every cent in the simulation is held by a [`Books`] value behind private
//! fields, and the only way to move one is [`Books::transfer`]. No agent type
//! holds a balance. `Household` and `Firm` do not exist yet — Phase 3 owns
//! `world.rs` — so LEDG-01 is stated here as the positive property that will
//! still be checkable then: **the only writable path to a balance is inside
//! this file.** Phase 3 inherits that obligation and must not add a balance
//! field or a cash setter to an agent struct; the books already own the
//! quantity.
//!
//! **The books own every goods unit on the same terms (LEDG-05).** Stock is
//! held per *account*, addressed exactly as cash is, and the only ways to move
//! a unit are [`Books::produce`], [`Books::consume`] and [`Books::exchange`].
//! The identity is
//!
//! ```text
//! produced − consumed − Σ_accounts stock == 0        for each good
//! ```
//!
//! and it is a real check rather than a tautology only because its two sides
//! come from two separately maintained sources. The `produced` and `consumed`
//! totals are advanced from the **arguments** of the operations; the running
//! goods residual is advanced from the **legs of the postings** those
//! operations record. Deriving either from the other — recomputing `produced`
//! by walking the journal at check time, say — would compare a number against
//! itself and pass forever.
//!
//! **The identity has one shape under either Phase 7 consumption model.** If a
//! purchased unit is consumed in the same tick, a household's stock returns to
//! zero within the tick; if it is held, that stock is non-zero across a tick
//! boundary. The only difference between the two worlds is whether a household
//! stock slot happens to be non-zero at the moment the check runs. No formula,
//! no field and no check differs, which is why Phase 7 (MKT-06) can settle the
//! question without touching this file.
//!
//! **The books own the headcount too, and that is a decision rather than a
//! convenience (LEDG-06).** LEDG-06 names cash, inventory *and* headcount as
//! the three quantities no account may hold a negative amount of. No employment
//! relation exists before Phase 6, so the headcount could have been left for
//! Phase 6 to introduce wherever it liked. It is here instead, for three
//! reasons, and a later reader who wants to move it should weigh all three.
//!
//! 1. The books own every quantity an invariant reads. Putting one of the three
//!    somewhere else makes that sentence false, and a non-negativity check that
//!    has to reach outside the books for its third column is a check with two
//!    owners.
//! 2. It removes a cross-phase promise rather than recording one. Phase 6 has
//!    nowhere else to put a headcount, so nothing has to be remembered, and the
//!    alternative — a note in a roadmap saying "Phase 6 must also check
//!    headcount" — is exactly the kind of promise that is kept until it is not.
//! 3. **A count is unsigned, so the non-negativity of that column is a fact of
//!    its type and not a runtime loop.** That matters more than it looks. The
//!    honest alternative for a quantity nobody owns yet is a loop over an empty
//!    structure, which passes vacuously and is indistinguishable — in a test
//!    report, in a coverage number, in a reviewer's reading — from a check that
//!    works. A type-level fact documented as one cannot rot into that.
//!
//! Two things about it are easy to misread, so they are stated flatly.
//!
//! **A headcount is not conserved value.** Unlike a balance it has no
//! counterparty, no opening stock and no conservation identity in this
//! milestone. [`Books::set_headcount`] therefore does *not* contradict LEDG-01,
//! which is about cash: there is nothing for a headcount to be moved *from*.
//! Hiring is not a transfer of people between firms in this model; a firm's
//! payroll count is simply a number about that firm.
//!
//! **[`Books::set_headcount`] is the whole of Phase 2's headcount vocabulary.**
//! Phase 6 (LABR-01 … LABR-08) owns hiring, firing and the employment relation
//! itself, and it builds them *on top of* this accessor rather than beside it.
//! The constructor leaves every slot at zero because no employment exists before
//! Phase 6 — that is an initial condition of the model, not a tunable, so it
//! belongs in code and not in the configuration file.
//!
//! **Atomicity (LEDG-02) has four legs, and an exclusive borrow is only one.**
//!
//! 1. [`Books::transfer`] takes `&mut self`, so no shared view of the books can
//!    coexist with a transfer in progress.
//! 2. No method that borrows the books mutably takes a closure, a function
//!    pointer, a trait object or a callback of any kind, and [`Books`] holds no
//!    field of such a type. This leg is not redundant: the borrow checker
//!    constrains an *external* observer, and a callback is an *internal* one,
//!    exempt by construction. A hook of that shape was written against this
//!    design and compiled clean while observing a total of 50 cents against an
//!    opening stock of 100. If a later phase wants a logging hook, read the
//!    journal after the call — that is what it is for.
//! 3. No shared-mutability or reference-counted wrapper appears in this file,
//!    and the crate forbids unsafe code in `src/lib.rs`, which closes the
//!    raw-pointer route.
//! 4. [`Books::transfer`] is compute-then-commit: every step that can fail runs
//!    before any write, and the commit step is assignments only. The naive
//!    ordering — decrement, then fail — leaves the books at -400 against an
//!    opening 100 and is observable from a caller that catches the unwind.
//!
//! **The caller's obligation (LEDG-03).** [`Books::transfer`] returns the amount
//! it actually moved. A caller keeping any derived total — payroll paid this
//! month, revenue this tick — bumps it by the **returned** value and never by
//! the amount it asked for; better still, derives the total from the journal.
//! A partial payment path lands in Phase 6, and an accumulator bumped by the
//! intended amount leaks there while the ledger itself stays perfect.
//!
//! **Money is an integer count of cents throughout.** No value in this module
//! belongs to the float domain, and this file names no type from it at all,
//! which is the grep-able form of that rule (`tests/numeric_det.rs`).
//!
//! **The two keying rules are deliberately different.** Balances are keyed on
//! the firm **slot**, which is stable for the whole run, so a Phase 10 respawn
//! cannot orphan a slot's money. Postings are keyed on the full
//! [`Account`] identity, so the journal records *which* occupant acted.
//! Do not unify them: a balance vector indexed by the generational identity
//! loses the previous occupant's money at exactly the tick a firm goes
//! bankrupt.

use serde::{Serialize, Serializer};
use thiserror::Error;

use crate::config::Params;
use crate::ids::{Account, FirmId, FirmSlot, GoodId, HouseholdId};
use crate::money::{Money, MoneyOverflow};

/// The good every posting in this phase names. One good ("food") in v1.
const ONLY_GOOD: GoodId = GoodId(0);

/// Every good these books carry.
///
/// Exactly one in v1. Phase 5 (PROD-01) widens this into a goods table with
/// recipes; that is a change to the **dimension** of the containers below and
/// not to the shape of the identity, and every accessor is already
/// account-and-good-shaped, so no call site moves when it happens.
static GOODS: [GoodId; 1] = [ONLY_GOOD];

/// What a posting records.
///
/// New variants are **appended**, never inserted or reordered: the serialised
/// form below is the wire shape Phase 3 writes into its event stream, and a
/// renamed or reordered variant is a trajectory-visible change to a committed
/// log rather than a refactor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostingKind {
    /// Opening endowment, of cash or of units. Its counterparty is outside the
    /// books by definition, so its debit leg carries no amount. A units
    /// endowment counts into the produced total — see [`Books::new`]. Not a
    /// transaction: see [`Books::transactions_this_tick`].
    Endow,
    /// Cash moved from one account in these books to another.
    Transfer,
    /// Cash moved one way and units the other, as a single posting. A
    /// transaction, exactly as a [`PostingKind::Transfer`] is.
    Exchange,
    /// Units entered the system at an account. The one place a unit is created,
    /// and therefore the one thing that advances the produced total. Not a
    /// transaction: a tick in which firms only produced has traded nothing,
    /// which is the degenerate state LEDG-08 exists to catch.
    Produce,
    /// Units left the system from an account. Recorded as a posting rather than
    /// performed as a bare subtraction, so that a consumption defect is nameable
    /// in the journal (LEDG-09) and consumption is an explicit modelled step
    /// (MKT-06). Not a transaction, for the same reason production is not.
    Consume,
}

/// One line of the journal: what moved, between whom, and what the running
/// residuals were immediately after it applied.
///
/// Three shape decisions carry weight.
///
/// **The cash leg is two amounts, not one.** `debit_cents` leaves the debit
/// account and `credit_cents` arrives at the credit account. Their *inequality*
/// is what makes a non-conserving posting expressible as data rather than only
/// as a balance discrepancy, and it is why LEDG-07 is checkable on a single
/// posting. Collapsing them into one amount would make that check a tautology:
/// an over-credit would be inexpressible and the check could never fire.
///
/// **The units leg is two amounts for the same reason,** and it points the
/// other way. Units move *opposite* to cash: the buyer is the debit account
/// because it pays, and the units it receives left the credit account. LEDG-07
/// is exactly the statement that the two legs name the same pair of accounts in
/// opposite directions, and it is checkable on one posting only because each
/// leg carries its own amount.
///
/// **The two residual fields are the running residuals *after* this posting
/// applied.** That is what turns localisation into a scan over
/// already-computed numbers instead of a replay from a tick-start snapshot.
///
/// The serialised form renders each address through its `Display` impl —
/// `household:12`, `firm:3:0` — so an event stream stays greppable by agent and
/// the ledger, not `src/ids.rs`, owns the wire shape of an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Posting {
    /// Index within this tick's journal, from zero. Reset by
    /// [`Books::end_of_tick`].
    pub seq: u32,
    pub kind: PostingKind,
    #[serde(serialize_with = "serialize_account")]
    pub debit: Account,
    #[serde(serialize_with = "serialize_account")]
    pub credit: Account,
    /// Cents that left `debit`.
    pub debit_cents: i64,
    /// Cents that arrived at `credit`. Equal to `debit_cents` for a conserving
    /// posting.
    pub credit_cents: i64,
    #[serde(serialize_with = "serialize_good")]
    pub good: GoodId,
    /// Units that left the account they left: the **credit** account for a
    /// [`PostingKind::Exchange`] (the seller), and the single account named on
    /// both legs for a [`PostingKind::Consume`].
    pub units_out: i64,
    /// Units that arrived where they arrived: the **debit** account for a
    /// [`PostingKind::Exchange`] (the buyer), and the single account named on
    /// both legs for a [`PostingKind::Produce`] or a units
    /// [`PostingKind::Endow`].
    pub units_in: i64,
    /// The books' cash residual after this posting: cents posted so far, less
    /// the opening stock. Zero when the run conserves.
    pub cash_residual_cents: i64,
    /// The books' goods residual after this posting. Zero when the run
    /// conserves.
    pub goods_residual_units: i64,
}

/// Render an address through its `Display` form, so a serialised posting names
/// `"household:12"` rather than a structural encoding of the identity.
fn serialize_account<S: Serializer>(account: &Account, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(account)
}

/// Render a good as its bare index. Unlike an address, a good has no ambiguity
/// to disambiguate and is read numerically on the analysis side.
fn serialize_good<S: Serializer>(good: &GoodId, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u16(good.0)
}

impl std::fmt::Display for Posting {
    /// The form a halt message embeds (LEDG-09). Integer identifiers and
    /// integer amounts only: no path, no host name, no wall-clock reading and
    /// no process id may reach a message a run emits (TICK-06).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            PostingKind::Endow => "endow",
            PostingKind::Transfer => "transfer",
            PostingKind::Exchange => "exchange",
            PostingKind::Produce => "produce",
            PostingKind::Consume => "consume",
        };
        write!(
            f,
            "#{} {} {} -> {} debit {}c credit {}c {} out {} in {}",
            self.seq,
            kind,
            self.debit,
            self.credit,
            self.debit_cents,
            self.credit_cents,
            self.good,
            self.units_out,
            self.units_in
        )
    }
}

/// The books declined to act, and **nothing was written**.
///
/// A refusal is a legitimate runtime condition rather than a defect: an
/// overdraft is an economic event. Every field is a scalar or an identity so
/// the whole enum is `Copy` and can be logged, matched on and returned without
/// a clone.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PostError {
    /// The payer cannot cover the amount.
    #[error("{account} cannot pay {amount_cents} cents: balance is {balance_cents} cents")]
    Overdraft {
        account: Account,
        amount_cents: i64,
        balance_cents: i64,
    },

    /// The address names no account in these books, or names a firm identity
    /// whose generation no longer occupies its slot.
    #[error("{0} is not an account in these books")]
    UnknownAccount(Account),

    /// A negative amount is refused rather than treated as a reverse transfer.
    /// Without this refusal a negative amount would credit the payer and debit
    /// the payee while skipping the overdraft check on the account it actually
    /// debits.
    #[error("{amount_cents} cents is not an amount that can be transferred")]
    NegativeAmount { amount_cents: i64 },

    /// A two-party posting that names one account on both legs is not well
    /// formed. Plan 02-04's zero-sum check reports exactly that shape;
    /// refusing it here means the journal never records one.
    #[error("{account} cannot transfer to itself")]
    SelfDealing { account: Account },

    /// A balance could not be represented after the move. Reported rather than
    /// aborted because it is reached through the named, non-panicking half of
    /// the money API, which is what keeps the commit step infallible.
    #[error("the transfer does not fit in the money range: {0}")]
    Range(#[from] MoneyOverflow),

    /// The account holds fewer units of the good than the operation asked for.
    /// The goods counterpart of an overdraft, and an economic event in exactly
    /// the same way: a firm that has sold out is not a defect.
    #[error(
        "{account} cannot release {units_requested} units of {good}: it holds \
         {units_held}"
    )]
    ShortStock {
        account: Account,
        good: GoodId,
        units_requested: i64,
        units_held: i64,
    },

    /// The identifier names no good these books carry. Refused rather than
    /// indexed: a good outside the table has no stock vector, and reading one
    /// as zero would let a caller "consume" from a good that does not exist.
    #[error("{0} is not a good these books carry")]
    UnknownGood(GoodId),

    /// An exchange with an empty leg: no cash, or no units, or neither.
    ///
    /// Refused rather than recorded, for the same reason
    /// [`PostError::SelfDealing`] is. An exchange moves cash one way and units
    /// the other; one that moves nothing is not a smaller exchange but a
    /// different shape, and it would still count towards the liveness minimum
    /// (LEDG-08) — the degenerate "a transaction happened" pass that check
    /// exists to close. `invariants::check_zero_sum` reports exactly this
    /// shape, so refusing it here means the journal never records one.
    #[error(
        "an exchange of {units} units for {amount_cents} cents has an empty leg: an \
         exchange moves cash one way and units the other, and neither leg may be empty"
    )]
    EmptyExchange { units: i64, amount_cents: i64 },

    /// A transfer of nothing.
    ///
    /// Refused on exactly the terms [`PostError::EmptyExchange`] is, and for
    /// exactly the same reason: it moves no cent, yet
    /// [`Books::transactions_this_tick`] would count it, and
    /// `invariants::check_liveness` would then read `counted >= 1` for a tick
    /// in which nothing changed hands — the degenerate "a transaction
    /// happened" pass LEDG-08 exists to close. The counting rule is what makes
    /// LEDG-08 mean "money changed hands" rather than "something happened", and
    /// it only means that if a zero-cent transfer never reaches the recorder.
    ///
    /// This is not a hypothetical. Phase 6 introduces partial payroll payment
    /// and Phase 8 dividends; a firm with no cash paying a wage of zero calls
    /// `transfer(firm, household, Money::ZERO)`, and the baseline configuration
    /// records that the liveness gate turns on in the same phase.
    /// `invariants::check_zero_sum` reports the same shape through
    /// `ZeroSumDetail::EmptyTransfer`, so refusing it here means the journal
    /// never records one and a posting that somehow appears is still named.
    #[error(
        "a transfer of {amount_cents} cents moves nothing: it would change no \
         balance and would still count towards the liveness minimum"
    )]
    EmptyTransfer { amount_cents: i64 },

    /// A negative unit count is refused rather than treated as a movement in
    /// the opposite direction, for the same reason [`PostError::NegativeAmount`]
    /// is: reversing the direction skips the stock check on the side that
    /// actually gives up the units, so units would be created from nothing.
    #[error("{units} units is not a quantity that can be moved")]
    NegativeUnits { units: i64 },
}

/// The books could not be constructed from these parameters.
///
/// A distinct type from [`PostError`] because it reports a configuration
/// combination — operator error — and not a refused posting. It is a returned
/// error rather than an abort for the same reason: the numbers came from the
/// operator, and this project surfaces operator input as a named typed error.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum BooksError {
    /// The per-agent endowments do not add up to the configured money stock, so
    /// the books would begin the run already failing conservation.
    #[error(
        "the endowment sums to {endowed_cents} cents but the configured money stock is \
         {opening_cents} cents; the books would begin the run already broken"
    )]
    EndowmentDoesNotSumToStock {
        endowed_cents: i64,
        opening_cents: i64,
    },

    /// The configured initial inventory is not a quantity a firm can hold:
    /// negative, or so large that endowing every slot leaves the total outside
    /// the integer range.
    ///
    /// The configuration layer bounds the money stock but not this key, and a
    /// negative endowment would open the books with negative inventory while
    /// the identity still balanced — negative `produced` against negative
    /// stock. The identity cannot catch it, so the constructor refuses it,
    /// which is the same boundary at which [`PostError::NegativeUnits`] refuses
    /// one.
    #[error(
        "an initial inventory of {units_per_firm} units across {firms} firm slots \
         is not a quantity these books can hold"
    )]
    InitialInventoryOutOfRange { units_per_firm: i64, firms: u16 },

    /// The goods identity does not hold once construction has endowed every
    /// firm slot, so the books would begin the run already failing goods
    /// conservation.
    #[error(
        "the initial inventory endows {endowed_units} units but the goods identity \
         is off by {residual_units} units; the books would begin the run already \
         broken"
    )]
    InventoryDoesNotBalance {
        endowed_units: i64,
        residual_units: i64,
    },
}

/// Where a resolved account's quantities live — its cash and its stock, which
/// are indexed identically.
///
/// Private, and the only way to reach a balance or a stock vector index.
/// Constructing one requires passing [`Books::resolve`], which is what
/// bounds-checks the index and compares a firm's generation. That one
/// resolution serving both quantities is what keeps a firm's inventory keyed on
/// the same slot as its cash, so a Phase 10 respawn cannot carry one forward
/// and orphan the other (T-02-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccountSlot {
    Household(usize),
    Firm(usize),
}

/// Every cent and every unit in the simulation, plus this tick's journal.
///
/// All fields are private and there is exactly one constructor. No accessor
/// returns a mutable reference to a balance, a stock value or either vector:
/// that would hand a caller the mutation point [`Books::transfer`] and the
/// three goods operations exist to monopolise, and no search for a setter
/// *name* would find a getter shaped that way.
#[derive(Debug, Clone)]
pub struct Books {
    /// The conservation baseline, set once in [`Books::new`] from the
    /// configured money stock. It is an input, independent of the balances it
    /// is later compared against — never a sum over them. Deriving it from the
    /// balances would compare a number to itself and pass forever.
    opening_stock: Money,
    /// Cash by household index.
    household_cash: Vec<Money>,
    /// Cash by firm **slot**, not by firm identity. A slot is stable for the
    /// whole run, so a Phase 10 respawn carries the slot's money forward
    /// instead of orphaning it.
    firm_cash: Vec<Money>,
    /// Which occupant currently owns each slot's balance. Resolving a firm
    /// address compares against this, so an identity held across a respawn is a
    /// typed miss and never a silent hit on a different firm.
    ///
    /// Phase 2 adds no method that advances a generation. Phase 10 adds exactly
    /// one, and it **must not** reset the slot's balance: conservation depends
    /// on that money carrying forward into the successor's hands.
    firm_generation: Vec<u32>,
    /// Stock by household index, for the one good v1 carries. The same
    /// two-vector shape as cash, and indexed by the same resolved
    /// [`AccountSlot`], so a household's units and its cents cannot drift onto
    /// different keys.
    ///
    /// Phase 5 (PROD-01) widens this to a `Vec` per good. That changes the
    /// containers' dimension and nothing about the identity's shape.
    household_stock: Vec<i64>,
    /// Stock by firm **slot**, for the same reason [`Books::firm_cash`] is: a
    /// slot outlives its occupant, and a Phase 10 respawn must carry the
    /// inventory forward rather than orphan it.
    firm_stock: Vec<i64>,
    /// Employees on each firm slot's payroll, keyed by **slot** for the same
    /// reason that slot's cash and stock are: a slot outlives its occupant, and
    /// a Phase 10 respawn decides what the successor inherits rather than having
    /// the answer forced by an orphaned vector entry.
    ///
    /// **Unsigned on purpose.** A negative headcount is not representable, so
    /// LEDG-06's third column is closed by the type rather than by a runtime
    /// loop over a structure that, in this milestone, no operation fills. See
    /// this module's headcount note and `check_non_negative` in
    /// `src/invariants.rs`, which documents the same fact from the other side.
    ///
    /// Not a conserved quantity: it has no counterparty, no opening stock and
    /// no identity to hold. Phase 6 owns the employment relation that gives the
    /// number its meaning.
    firm_headcount: Vec<u32>,
    /// Units that have entered the system, ever. Advanced from the **argument**
    /// of [`Books::produce`] and from the constructor's inventory endowment —
    /// never from the journal. That independence from the running residual
    /// below is the whole reason the goods check is not a tautology.
    produced: i64,
    /// Units that have left the system, ever. Advanced from the **argument** of
    /// [`Books::consume`], on the same terms.
    consumed: i64,
    /// This tick's postings. Cleared, not reallocated, by
    /// [`Books::end_of_tick`], so the capacity is reused.
    journal: Vec<Posting>,
    next_seq: u32,
    /// Cents posted so far, less the opening stock. Maintained incrementally by
    /// the recorder and never reset by [`Books::end_of_tick`] — it measures the
    /// whole run against its opening stock and is meaningful only cumulatively.
    cash_residual_cents: i64,
    /// The goods identity's running residual, maintained the same way and from
    /// the **legs of the postings** — the second, independent source the goods
    /// check compares the recomputed identity against. Zero when the run
    /// conserves, and never reset by [`Books::end_of_tick`].
    goods_residual_units: i64,
    transactions_this_tick: u32,
}

impl Books {
    /// Build the books from the run's parameters and endow every agent.
    ///
    /// Five steps, in order:
    ///
    /// 1. `opening_stock` is set from the configured money stock — the config
    ///    value, never a sum over the balances it will be compared against.
    /// 2. The running cash residual is seeded to the negative of that stock.
    /// 3. Every household receives the configured household liquidity and every
    ///    firm slot the configured firm liquidity, each recorded as an
    ///    [`PostingKind::Endow`] posting whose credit leg carries the amount and
    ///    whose debit leg carries nothing: an endowment's counterparty is
    ///    outside the books by definition. Every firm slot also receives the
    ///    configured initial inventory, as a second endowment posting carrying
    ///    the units arriving.
    /// 4. The running residuals must have returned to zero — the cash one
    ///    because the endowment must sum to the stock, the goods one because the
    ///    inventory endowment must balance against the produced total it
    ///    advances. If either has not, construction fails with both numbers.
    ///    These are construction-time checks and are *different* checks from the
    ///    per-tick ones; both are needed.
    /// 5. The journal is cleared and the sequence and transaction counters are
    ///    reset, so **tick 0 begins with an empty journal.**
    ///
    /// **The initial inventory is counted into `produced`, and that is
    /// load-bearing.** The goods identity is `produced − consumed − Σstock`.
    /// Endowing a firm's inventory raises `Σstock`, so without the matching
    /// count into `produced` the identity fails on tick 0 by exactly the
    /// endowment — the single most likely "it fails on tick 0 and nobody knows
    /// why" defect in this phase. The money side says the same thing in its own
    /// terms: `opening_stock` is set from the configured stock so that the cash
    /// endowment nets to zero against it.
    ///
    /// Step 5 closes the subtlest trap in this phase. If the endowment postings
    /// survived into tick 0's journal, the liveness check (LEDG-08) could pass
    /// on the strength of the endowment alone — exactly the degenerate pass it
    /// exists to close. Phase 3 therefore reads opening balances from the
    /// accessors below rather than from an endowment event.
    ///
    /// **Every firm slot opens with an empty payroll.** No employment relation
    /// exists before Phase 6, so zero is the initial condition of the model
    /// rather than a value anyone chose — which is why it is written here and
    /// not read from the configuration file. It is not endowed and records no
    /// posting: a headcount is not conserved value and has no counterparty.
    ///
    /// There is no other constructor and no default-construction impl. A
    /// default would build books with a zero opening stock, against which every
    /// conservation check passes trivially.
    pub fn new(params: &Params) -> Result<Books, BooksError> {
        let opening_cents = params.money.total_money_cents;

        // Mirrors `FirmArena::with_occupants`, for the identical reason: a firm
        // count past the slot range would issue one `FirmSlot` for two firms.
        // `config::load` refuses such a count before this point, so the bound is
        // restated here rather than re-derived, and no narrowing cast appears
        // below it.
        let firm_slots = u16::try_from(params.sim.firms).expect(
            "a run has at most u16::MAX firm slots; FirmSlot is a u16 and a wider \
             arena would silently alias two firms onto one identity",
        );
        let households = params.sim.households as usize;
        let firms = firm_slots as usize;

        // A stock at the far negative end of the range has no representable
        // negation, so no endowment can sum to it. Unreachable for parameters
        // that came through `config::load`, which refuses a non-positive stock,
        // and reported rather than aborted because the number is the operator's.
        let Some(residual_seed) = opening_cents.checked_neg() else {
            return Err(BooksError::EndowmentDoesNotSumToStock {
                endowed_cents: 0,
                opening_cents,
            });
        };

        // The configuration layer bounds the money stock but not the inventory
        // key, so the bound is imposed here. A negative endowment would open the
        // books with negative inventory and a *balanced* identity — negative
        // `produced` against negative stock — so the identity cannot catch it
        // and the refusal has to be at the boundary.
        let units_per_firm = params.firm.initial_inventory_units;
        let endowed_units = i64::from(firm_slots).checked_mul(units_per_firm);
        let Some(endowed_units) = endowed_units.filter(|_| units_per_firm >= 0) else {
            return Err(BooksError::InitialInventoryOutOfRange {
                units_per_firm,
                firms: firm_slots,
            });
        };

        let mut books = Books {
            opening_stock: Money::from_cents(opening_cents),
            household_cash: vec![Money::ZERO; households],
            firm_cash: vec![Money::ZERO; firms],
            firm_generation: vec![0; firms],
            household_stock: vec![0; households],
            firm_stock: vec![0; firms],
            // Every slot opens with an empty payroll: no employment relation
            // exists before Phase 6. An initial condition of the model, not a
            // parameter, so it is written here and not read from the config.
            firm_headcount: vec![0; firms],
            produced: 0,
            consumed: 0,
            journal: Vec::new(),
            next_seq: 0,
            cash_residual_cents: residual_seed,
            goods_residual_units: 0,
            transactions_this_tick: 0,
        };

        let household_liquidity = Money::from_cents(params.household.initial_liquidity_cents);
        for index in 0..params.sim.households {
            books.household_cash[index as usize] = household_liquidity;
            let account = Account::Household(HouseholdId(index));
            books.record(Posting {
                seq: 0,
                kind: PostingKind::Endow,
                debit: account,
                credit: account,
                debit_cents: 0,
                credit_cents: household_liquidity.cents(),
                good: ONLY_GOOD,
                units_out: 0,
                units_in: 0,
                cash_residual_cents: 0,
                goods_residual_units: 0,
            });
        }

        let firm_liquidity = Money::from_cents(params.firm.initial_liquidity_cents);
        for index in 0..firm_slots {
            books.firm_cash[index as usize] = firm_liquidity;
            let account = Account::Firm(FirmId {
                slot: FirmSlot(index),
                generation: books.firm_generation[index as usize],
            });
            books.record(Posting {
                seq: 0,
                kind: PostingKind::Endow,
                debit: account,
                credit: account,
                debit_cents: 0,
                credit_cents: firm_liquidity.cents(),
                good: ONLY_GOOD,
                units_out: 0,
                units_in: 0,
                cash_residual_cents: 0,
                goods_residual_units: 0,
            });

            // The stock and the produced total move together, or the identity
            // fails at tick 0 by exactly this endowment. The total for every
            // slot fits, because `endowed_units` above was computed and checked.
            books.firm_stock[index as usize] = units_per_firm;
            books.produced += units_per_firm;
            books.record(Posting {
                seq: 0,
                kind: PostingKind::Endow,
                debit: account,
                credit: account,
                debit_cents: 0,
                credit_cents: 0,
                good: ONLY_GOOD,
                units_out: 0,
                units_in: units_per_firm,
                cash_residual_cents: 0,
                goods_residual_units: 0,
            });
        }

        if books.cash_residual_cents != 0 {
            let endowed_cents = books
                .cash_residual_cents
                .checked_add(opening_cents)
                .unwrap_or(i64::MAX);
            return Err(BooksError::EndowmentDoesNotSumToStock {
                endowed_cents,
                opening_cents,
            });
        }

        if books.goods_residual_units != 0 {
            return Err(BooksError::InventoryDoesNotBalance {
                endowed_units,
                residual_units: books.goods_residual_units,
            });
        }

        books.journal.clear();
        books.next_seq = 0;
        books.transactions_this_tick = 0;
        Ok(books)
    }

    /// Move `amount` from `from` to `to`, returning the amount actually moved.
    ///
    /// **The only cash-mutation point in the crate.**
    ///
    /// Compute-then-commit. Everything that can fail — resolving both
    /// addresses, refusing a negative amount, refusing a payer and payee that
    /// are the same account, subtracting from the payer, refusing an overdraft,
    /// adding to the payee — runs before any write. The commit step is two
    /// assignments and one call to the recorder, and contains nothing that can
    /// fail. Every money operation here goes through the named,
    /// result-returning half of the money API rather than an operator: the
    /// operators abort in every profile, and an abort between the two
    /// assignments is exactly the state this shape exists to make unreachable.
    ///
    /// Takes no closure, function pointer, trait object or callback, and never
    /// will. See this module's atomicity note, leg 2.
    ///
    /// **The return value is the amount moved, and a caller must use it.** For
    /// a whole-amount transfer it equals the argument and looks redundant. It
    /// is not: Phase 6's partial payroll payment and Phase 8's dividend need a
    /// sibling whose return genuinely differs, and giving both the same return
    /// type today means no call site changes shape then.
    pub fn transfer(
        &mut self,
        from: Account,
        to: Account,
        amount: Money,
    ) -> Result<Money, PostError> {
        // --- compute: every fallible step, before any write -----------------
        if amount.cents() < 0 {
            return Err(PostError::NegativeAmount {
                amount_cents: amount.cents(),
            });
        }
        // A transfer of nothing is refused at the boundary on exactly the terms
        // an empty exchange is, and for the same reason: `record` counts a
        // transfer towards the liveness minimum whatever its amount, so a
        // zero-cent transfer would satisfy LEDG-08 for a tick in which not one
        // cent moved. See [`PostError::EmptyTransfer`].
        if amount.cents() == 0 {
            return Err(PostError::EmptyTransfer {
                amount_cents: amount.cents(),
            });
        }
        if from == to {
            return Err(PostError::SelfDealing { account: from });
        }

        let payer_slot = self.resolve(from).ok_or(PostError::UnknownAccount(from))?;
        let payee_slot = self.resolve(to).ok_or(PostError::UnknownAccount(to))?;

        let payer_balance = self.cash_at(payer_slot);
        let payer_after = payer_balance.checked_sub(amount)?;
        if payer_after.cents() < 0 {
            return Err(PostError::Overdraft {
                account: from,
                amount_cents: amount.cents(),
                balance_cents: payer_balance.cents(),
            });
        }
        let payee_after = self.cash_at(payee_slot).checked_add(amount)?;

        // --- commit: assignments only ---------------------------------------
        self.write_cash(payer_slot, payer_after);
        self.write_cash(payee_slot, payee_after);
        self.record(Posting {
            seq: 0,
            kind: PostingKind::Transfer,
            debit: from,
            credit: to,
            debit_cents: amount.cents(),
            credit_cents: amount.cents(),
            good: ONLY_GOOD,
            units_out: 0,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });
        Ok(amount)
    }

    /// Bring `units` of `good` into existence at `who`, returning the units
    /// actually created.
    ///
    /// **The one place a unit enters the system.** That is what makes the
    /// produced total a genuine second source rather than a restatement of the
    /// stock vectors: this method advances the total from its own *argument*,
    /// while the recorder advances the running residual from the *legs* of the
    /// posting it records.
    ///
    /// Compute-then-commit, exactly as [`Books::transfer`] is: the good, the
    /// sign of the count and the account are all settled before the first write,
    /// and the commit step is two assignments and one call to the recorder.
    ///
    /// Not a transaction. A tick in which firms only produced has traded
    /// nothing, and that is precisely the degenerate state LEDG-08 exists to
    /// catch.
    pub fn produce(&mut self, who: Account, good: GoodId, units: i64) -> Result<i64, PostError> {
        // --- compute: every fallible step, before any write -----------------
        if units < 0 {
            return Err(PostError::NegativeUnits { units });
        }
        if !Books::carries(good) {
            return Err(PostError::UnknownGood(good));
        }
        let slot = self.resolve(who).ok_or(PostError::UnknownAccount(who))?;

        // Bare integer arithmetic, deliberately. Unit counts are not `Money`
        // and have no non-panicking half to route through; both build profiles
        // enable overflow checks, so a count that cannot be represented aborts
        // here — before any write — rather than wrapping into a plausible
        // negative inventory (T-02-17).
        let stock_after = self.stock_at(slot) + units;
        let produced_after = self.produced + units;

        // --- commit: assignments only ---------------------------------------
        self.write_stock(slot, stock_after);
        self.produced = produced_after;
        self.record(Posting {
            seq: 0,
            kind: PostingKind::Produce,
            debit: who,
            credit: who,
            debit_cents: 0,
            credit_cents: 0,
            good,
            units_out: 0,
            units_in: units,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });
        Ok(units)
    }

    /// Consume `units` of `good` held by `who`, returning the units actually
    /// consumed.
    ///
    /// **The one place a unit leaves the system,** and a real posting rather
    /// than a bare subtraction. If consumption only moved the numbers, a
    /// consumption defect would be invisible in the journal and LEDG-09 could
    /// not name the posting that caused it. It is also what MKT-06 means by
    /// consumption being an explicit modelled step.
    ///
    /// Refuses with [`PostError::ShortStock`] rather than driving the account
    /// negative — the goods counterpart of refusing an overdraft, and an
    /// economic event in the same way.
    ///
    /// Not a transaction, for the same reason production is not.
    pub fn consume(&mut self, who: Account, good: GoodId, units: i64) -> Result<i64, PostError> {
        // --- compute: every fallible step, before any write -----------------
        if units < 0 {
            return Err(PostError::NegativeUnits { units });
        }
        if !Books::carries(good) {
            return Err(PostError::UnknownGood(good));
        }
        let slot = self.resolve(who).ok_or(PostError::UnknownAccount(who))?;

        let held = self.stock_at(slot);
        if held < units {
            return Err(PostError::ShortStock {
                account: who,
                good,
                units_requested: units,
                units_held: held,
            });
        }
        let stock_after = held - units;
        let consumed_after = self.consumed + units;

        // --- commit: assignments only ---------------------------------------
        self.write_stock(slot, stock_after);
        self.consumed = consumed_after;
        self.record(Posting {
            seq: 0,
            kind: PostingKind::Consume,
            debit: who,
            credit: who,
            debit_cents: 0,
            credit_cents: 0,
            good,
            units_out: units,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });
        Ok(units)
    }

    /// Move `amount` from `buyer` to `seller` and `units` of `good` from
    /// `seller` to `buyer`, returning both quantities actually moved.
    ///
    /// **One posting, not two.** That is the whole point of the method. A cash
    /// posting followed by a goods posting can be half-applied — the first
    /// succeeds, the second is refused, and the buyer has paid for units it
    /// never received. A single posting cannot: every refusal happens in the
    /// compute step, before the first write, and the commit step is four
    /// assignments and one call to the recorder, none of which can fail
    /// (T-02-14). It is also what lets plan 02-04 check zero-sum (LEDG-07) as a
    /// property of one posting rather than of an aggregate.
    ///
    /// The **buyer is the debit account** because it is the one that pays; the
    /// units therefore travel credit-to-debit, opposite to the cash. See
    /// [`Posting`]'s units-leg note.
    ///
    /// **This phase has no notion of a sale.** `exchange` is a ledger
    /// operation. Whether a particular exchange is a household buying food,
    /// and whether the units it receives are consumed at once or held, are
    /// Phase 7 questions (MKT-06) that change nothing here.
    ///
    /// Returns the pair `(cash moved, units moved)`, and a caller must use
    /// both. See this module's LEDG-03 note: an accumulator bumped by the
    /// *intended* quantity leaks the moment a partial path exists, while the
    /// ledger itself stays perfect.
    pub fn exchange(
        &mut self,
        buyer: Account,
        seller: Account,
        good: GoodId,
        units: i64,
        amount: Money,
    ) -> Result<(Money, i64), PostError> {
        // --- compute: every fallible step, before any write -----------------
        if amount.cents() < 0 {
            return Err(PostError::NegativeAmount {
                amount_cents: amount.cents(),
            });
        }
        if units < 0 {
            return Err(PostError::NegativeUnits { units });
        }
        if amount.cents() == 0 || units == 0 {
            return Err(PostError::EmptyExchange {
                units,
                amount_cents: amount.cents(),
            });
        }
        if buyer == seller {
            return Err(PostError::SelfDealing { account: buyer });
        }
        if !Books::carries(good) {
            return Err(PostError::UnknownGood(good));
        }

        let buyer_slot = self
            .resolve(buyer)
            .ok_or(PostError::UnknownAccount(buyer))?;
        let seller_slot = self
            .resolve(seller)
            .ok_or(PostError::UnknownAccount(seller))?;

        let buyer_balance = self.cash_at(buyer_slot);
        let buyer_cash_after = buyer_balance.checked_sub(amount)?;
        if buyer_cash_after.cents() < 0 {
            return Err(PostError::Overdraft {
                account: buyer,
                amount_cents: amount.cents(),
                balance_cents: buyer_balance.cents(),
            });
        }
        let seller_cash_after = self.cash_at(seller_slot).checked_add(amount)?;

        let seller_held = self.stock_at(seller_slot);
        if seller_held < units {
            return Err(PostError::ShortStock {
                account: seller,
                good,
                units_requested: units,
                units_held: seller_held,
            });
        }
        let seller_stock_after = seller_held - units;
        let buyer_stock_after = self.stock_at(buyer_slot) + units;

        // --- commit: assignments only ---------------------------------------
        self.write_cash(buyer_slot, buyer_cash_after);
        self.write_cash(seller_slot, seller_cash_after);
        self.write_stock(seller_slot, seller_stock_after);
        self.write_stock(buyer_slot, buyer_stock_after);
        self.record(Posting {
            seq: 0,
            kind: PostingKind::Exchange,
            debit: buyer,
            credit: seller,
            debit_cents: amount.cents(),
            credit_cents: amount.cents(),
            good,
            units_out: units,
            units_in: units,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });
        Ok((amount, units))
    }

    /// The cash `account` holds, or `None` if it names no account in these
    /// books — including a firm identity whose generation no longer occupies
    /// its slot.
    pub fn cash_of(&self, account: Account) -> Option<Money> {
        self.resolve(account).map(|slot| self.cash_at(slot))
    }

    /// Every account these books hold, in the order the non-negativity check
    /// walks them: households by ascending index, then firm slots by ascending
    /// slot, each firm named by the identity that currently occupies it.
    ///
    /// **The order is part of the invariant contract**, not an incidental
    /// property of two vectors. Two accounts can hold a negative quantity at the
    /// same time, and a check that reported an arbitrary one of them would make
    /// its own negative test flaky in a way that looks exactly like a real
    /// failure. The order rests on the derived total order `src/ids.rs` already
    /// carries, so it is the same order every other part of the system would
    /// produce if it sorted.
    ///
    /// Borrows shared, mutates nothing and allocates nothing: it is walked once
    /// per tick for the whole run.
    pub fn accounts(&self) -> impl Iterator<Item = Account> + '_ {
        let households = (0..self.household_cash.len()).map(|index| {
            let index = u32::try_from(index)
                .expect("the household vectors were sized from a u32 household count");
            Account::Household(HouseholdId(index))
        });
        let firms = self
            .firm_generation
            .iter()
            .copied()
            .enumerate()
            .map(|(index, generation)| {
                let slot = u16::try_from(index)
                    .expect("the firm vectors were sized from a u16 slot count");
                Account::Firm(FirmId {
                    slot: FirmSlot(slot),
                    generation,
                })
            });
        households.chain(firms)
    }

    /// Every cent the books hold, summed over the balances.
    ///
    /// One of the two independent sources the conservation check compares: this
    /// side comes from the **balances**, the journal residual comes from the
    /// **postings**. Two genuinely separate derivations are what make that check
    /// non-vacuous.
    pub fn total_money(&self) -> Money {
        self.household_cash
            .iter()
            .copied()
            .chain(self.firm_cash.iter().copied())
            .sum()
    }

    /// Every cent held by firms. Recomputed on each call and deliberately not
    /// cached: a cache would be a second source of truth for a number nothing
    /// checks, and it would drift invisibly.
    pub fn firm_cash_total(&self) -> Money {
        self.firm_cash.iter().copied().sum()
    }

    /// The conservation baseline: the configured money stock.
    pub fn opening_stock(&self) -> Money {
        self.opening_stock
    }

    /// Cents posted so far, less the opening stock. Zero when the run conserves.
    pub fn cash_residual_cents(&self) -> i64 {
        self.cash_residual_cents
    }

    /// Every good these books carry.
    ///
    /// The goods check iterates this, so its loop body is entered on every run
    /// — a check whose loop never runs passes vacuously, which is the failure
    /// this accessor exists to make impossible. Takes `&self` because Phase 5
    /// makes the table instance data rather than a constant.
    pub fn goods(&self) -> &'static [GoodId] {
        &GOODS
    }

    /// The units of `good` that `account` holds, or `None` if either names
    /// nothing in these books — an unknown account, a firm identity whose
    /// generation no longer occupies its slot, or a good the books do not
    /// carry.
    pub fn stock_of(&self, account: Account, good: GoodId) -> Option<i64> {
        if !Books::carries(good) {
            return None;
        }
        self.resolve(account).map(|slot| self.stock_at(slot))
    }

    /// Every unit of `good` the books hold, summed over the accounts.
    ///
    /// One of the two independent sources the goods check compares: this side
    /// comes from the **stock vectors** and the produced and consumed totals,
    /// while the running residual comes from the **postings**.
    ///
    /// A good these books do not carry yields zero, and that is a fact rather
    /// than a fallback: no unit of it has ever been produced, consumed or held,
    /// because every operation refuses it with
    /// [`PostError::UnknownGood`] before touching a vector. Same for
    /// [`Books::produced`] and [`Books::consumed`].
    pub fn total_stock(&self, good: GoodId) -> i64 {
        if !Books::carries(good) {
            return 0;
        }
        self.household_stock
            .iter()
            .chain(self.firm_stock.iter())
            .sum()
    }

    /// Units of `good` that have entered the system, ever — the constructor's
    /// inventory endowment plus every [`Books::produce`].
    pub fn produced(&self, good: GoodId) -> i64 {
        if !Books::carries(good) {
            return 0;
        }
        self.produced
    }

    /// Units of `good` that have left the system, ever, through
    /// [`Books::consume`].
    pub fn consumed(&self, good: GoodId) -> i64 {
        if !Books::carries(good) {
            return 0;
        }
        self.consumed
    }

    /// The goods identity's running residual, accumulated from the posting legs.
    /// Zero when the run conserves.
    pub fn goods_residual_units(&self) -> i64 {
        self.goods_residual_units
    }

    /// The number of employees on `slot`'s payroll, or `None` if `slot` names no
    /// firm slot in these books.
    ///
    /// Takes a [`FirmSlot`] and not an [`Account`], deliberately. A household
    /// has no payroll, so an address-shaped accessor would carry an arm that
    /// answers `None` for every household that has ever existed — a signature
    /// that invites a caller to ask a question with no answer. Taking the slot
    /// makes "only a firm has employees" a fact of the type.
    ///
    /// An out-of-range slot reads as `None` rather than panicking, on the same
    /// terms as [`Books::cash_of`]: a read is a question, and the answer to a
    /// question about an account these books do not hold is "there is none".
    pub fn headcount_of(&self, slot: FirmSlot) -> Option<u32> {
        self.firm_headcount.get(slot.0 as usize).copied()
    }

    /// Set `slot`'s payroll to `count`, returning the count it replaced, or
    /// `None` if `slot` names no firm slot in these books — in which case
    /// **nothing is written**.
    ///
    /// **This is the whole of Phase 2's headcount vocabulary.** There is no
    /// hire, no fire and no employment relation here; Phase 6 (LABR-01 …
    /// LABR-08) owns those and builds them on top of this rather than beside it.
    ///
    /// **It does not contradict LEDG-01.** A headcount is not conserved value:
    /// it has no counterparty and no opening stock, so unlike a balance there is
    /// nothing for it to be moved *from* and no second account whose number must
    /// change with it. `set` is therefore the honest verb, where for cash it
    /// would be a hole in the ledger.
    ///
    /// `count` is unsigned, so this method cannot record a negative payroll —
    /// which is how LEDG-06's third column is closed. See this module's
    /// headcount note.
    pub fn set_headcount(&mut self, slot: FirmSlot, count: u32) -> Option<u32> {
        let entry = self.firm_headcount.get_mut(slot.0 as usize)?;
        Some(std::mem::replace(entry, count))
    }

    /// Every employee on every payroll.
    ///
    /// Widened to a 64-bit count on the way out: a run may hold up to
    /// `u16::MAX` slots and each payroll is a `u32`, so the sum of every slot
    /// can exceed the width of any one of them. Summing at the element width
    /// would abort under this project's overflow checks on an economy that is
    /// merely large, which is not a defect worth aborting for.
    pub fn total_headcount(&self) -> u64 {
        self.firm_headcount.iter().copied().map(u64::from).sum()
    }

    /// This tick's postings, in the order they were recorded.
    pub fn journal(&self) -> &[Posting] {
        &self.journal
    }

    /// How many cash transactions this tick has recorded.
    ///
    /// Counts [`PostingKind::Transfer`] and [`PostingKind::Exchange`] postings
    /// and nothing else. An endowment is not a transaction, and neither is
    /// production nor consumption. That counting rule is what makes LEDG-08
    /// mean "money changed hands" rather than "something happened".
    pub fn transactions_this_tick(&self) -> u32 {
        self.transactions_this_tick
    }

    /// Close the tick: clear the journal, reset the sequence counter and reset
    /// the transaction count.
    ///
    /// The journal is cleared in place so its capacity is reused. The running
    /// residuals are deliberately **not** reset: they measure the whole run
    /// against its opening stock and are meaningful only cumulatively.
    pub fn end_of_tick(&mut self) {
        self.journal.clear();
        self.next_seq = 0;
        self.transactions_this_tick = 0;
    }

    /// Append `draft` to the journal, stamping its sequence number and the two
    /// running residuals.
    ///
    /// Takes a whole posting rather than nine parameters so that the caller
    /// names each leg at its construction site; `seq` and both residual fields
    /// are placeholders on the way in and are overwritten here.
    ///
    /// Both residual updates are constant time. Never sum the balances here:
    /// that would put a full recompute on the hot path of every later economic
    /// phase, at roughly three hundred times the cost of the per-tick check,
    /// and it would also collapse the two independent sources
    /// [`Books::total_money`] describes into one.
    fn record(&mut self, draft: Posting) {
        let cash_delta = draft.credit_cents.saturating_sub(draft.debit_cents);

        // The posting's own net effect on `produced − consumed − Σstock`: what
        // it adds to the produced total, less what it adds to the consumed
        // total, less the units arriving net of the units leaving. Both terms
        // are read off *this posting's* kind and legs, never off the running
        // totals the operations maintain — which is what makes the two sources
        // independent.
        //
        // The result is zero for every well-formed posting of every kind, and
        // non-zero exactly when a posting's legs contradict the totals its kind
        // claims to move: an exchange whose two units legs disagree, or a
        // produce that credits units it does not count. That is the quantity
        // the goods check localises against.
        let (produced_added, consumed_added) = match draft.kind {
            PostingKind::Produce | PostingKind::Endow => (draft.units_in, 0),
            PostingKind::Consume => (0, draft.units_out),
            PostingKind::Transfer | PostingKind::Exchange => (0, 0),
        };
        let goods_delta = produced_added
            .saturating_sub(consumed_added)
            .saturating_add(draft.units_out)
            .saturating_sub(draft.units_in);

        self.cash_residual_cents = self.cash_residual_cents.saturating_add(cash_delta);
        self.goods_residual_units = self.goods_residual_units.saturating_add(goods_delta);

        // An endowment is not a transaction, and neither is production nor
        // consumption: a tick in which firms only produced has traded nothing,
        // which is exactly the degenerate state LEDG-08 exists to catch. Cash
        // changing hands is what counts, whichever way the units went.
        if matches!(draft.kind, PostingKind::Transfer | PostingKind::Exchange) {
            self.transactions_this_tick = self.transactions_this_tick.saturating_add(1);
        }

        self.journal.push(Posting {
            seq: self.next_seq,
            cash_residual_cents: self.cash_residual_cents,
            goods_residual_units: self.goods_residual_units,
            ..draft
        });
        self.next_seq = self.next_seq.saturating_add(1);
    }

    /// Where `account`'s balance lives, or `None` if it names no account here.
    ///
    /// A firm address resolves only when its generation matches the ledger's
    /// record for that slot, so an identity held across a respawn is a typed
    /// miss rather than a silent hit on the successor.
    fn resolve(&self, account: Account) -> Option<AccountSlot> {
        match account {
            Account::Household(household) => {
                let index = household.0 as usize;
                (index < self.household_cash.len()).then_some(AccountSlot::Household(index))
            }
            Account::Firm(firm) => {
                let index = firm.slot.0 as usize;
                let generation = *self.firm_generation.get(index)?;
                (generation == firm.generation).then_some(AccountSlot::Firm(index))
            }
        }
    }

    /// Read the balance at an already-resolved slot.
    fn cash_at(&self, slot: AccountSlot) -> Money {
        match slot {
            AccountSlot::Household(index) => self.household_cash[index],
            AccountSlot::Firm(index) => self.firm_cash[index],
        }
    }

    /// Write the balance at an already-resolved slot.
    ///
    /// The index came from [`Books::resolve`], which bounds-checked it, and both
    /// vectors are fixed length for the life of the books — so this is an
    /// assignment and nothing more, which is what makes the commit step of
    /// [`Books::transfer`] infallible.
    fn write_cash(&mut self, slot: AccountSlot, value: Money) {
        match slot {
            AccountSlot::Household(index) => self.household_cash[index] = value,
            AccountSlot::Firm(index) => self.firm_cash[index] = value,
        }
    }

    /// Read the stock at an already-resolved slot. Indexed identically to the
    /// cash at that slot, from the same resolution.
    fn stock_at(&self, slot: AccountSlot) -> i64 {
        match slot {
            AccountSlot::Household(index) => self.household_stock[index],
            AccountSlot::Firm(index) => self.firm_stock[index],
        }
    }

    /// Write the stock at an already-resolved slot. An assignment and nothing
    /// more, for the same reason [`Books::write_cash`] is: that is what keeps
    /// the commit step of the three goods operations infallible.
    fn write_stock(&mut self, slot: AccountSlot, value: i64) {
        match slot {
            AccountSlot::Household(index) => self.household_stock[index] = value,
            AccountSlot::Firm(index) => self.firm_stock[index] = value,
        }
    }

    /// Whether these books carry `good`.
    ///
    /// Associated rather than a method on `&self` only until Phase 5 makes the
    /// goods table instance data; every call site is already written against a
    /// value the books own.
    fn carries(good: GoodId) -> bool {
        GOODS.contains(&good)
    }
}

/// The fault-injection vocabulary: four ways to break these books on purpose.
///
/// **Gated on the crate's own test configuration, and visible to the crate
/// only.** Verified on this toolchain in both directions: a method declared
/// this way is callable from a unit test inside this crate — private fields
/// included — and is a hard compile error from an integration test under
/// `tests/`, which reaches this crate exactly as any other consumer does. So no
/// consumer of `sim`, this crate's own integration tests included, can reach
/// anything below, and that boundary is enforced by the compiler rather than by
/// a review, a naming convention or a grep. Plan 02-06 turns the fact into an
/// executed probe rather than leaving it a claim.
///
/// **A cargo feature was considered here and rejected.** Its one advantage is
/// reachability from the integration tests, and it costs a features entry, a
/// second continuous-integration invocation and a standing assertion that the
/// feature stayed out of the default set — that is, a production hole that must
/// be proved shut on every run, bought in exchange for reaching a boundary the
/// unit tests already sit inside. The configuration gate has the same power and
/// leaves no hole to prove shut. There is therefore no feature, no runtime flag
/// and no builder switch anywhere in this file.
///
/// **Every method here writes state the public API cannot reach**, and three of
/// the four leave the books in a condition an invariant exists to reject. They
/// are what turn each check in `src/invariants.rs` from configured into
/// observed to fire: a check never seen to fire has never been shown to work.
///
/// The two that record a posting route it through [`Books::record`], the same
/// private recorder every real posting uses. That is deliberate and it is the
/// point: the residual arithmetic a negative test exercises is the production
/// arithmetic, and no test can hand-fake a residual.
#[cfg(test)]
impl Books {
    /// Move `cents` from `debit` to `credit`, credit `delta_cents` more than
    /// was debited, and record the posting that says so. Returns the posting as
    /// the recorder stamped it.
    ///
    /// **The realistic shape of a leak:** a posting whose two cash legs
    /// disagree, so the balances and the journal residual move together and by
    /// the same amount. A negative `delta_cents` is the dropped cent; a
    /// positive one is the over-credited posting; and applying it twice with
    /// equal and opposite arguments returns the running residual to zero, which
    /// is what the localisation test needs in order to reproduce a residual
    /// that cancels.
    ///
    /// The recorded posting is malformed for its kind as well as
    /// non-conserving, so the zero-sum check fires on the same books. That is a
    /// property of the corruption and not an accident: one fault tripping two
    /// checks is what makes the order of the check table observable.
    ///
    /// Panics if either address names no account in these books, or if the two
    /// name the same one — a single account written twice would lose the first
    /// write and silently seed a different fault than the one asked for.
    pub(crate) fn corrupt_recorded_cash(
        &mut self,
        debit: Account,
        credit: Account,
        cents: i64,
        delta_cents: i64,
    ) -> Posting {
        assert_ne!(
            debit, credit,
            "a corruption naming one account on both legs would write that \
             account twice and lose the first write"
        );
        let debit_slot = self
            .resolve(debit)
            .expect("a corruption names an account these books hold");
        let credit_slot = self
            .resolve(credit)
            .expect("a corruption names an account these books hold");

        let paid = self.cash_at(debit_slot).cents() - cents;
        let received = self.cash_at(credit_slot).cents() + cents + delta_cents;
        self.write_cash(debit_slot, Money::from_cents(paid));
        self.write_cash(credit_slot, Money::from_cents(received));

        self.record(Posting {
            seq: 0,
            kind: PostingKind::Transfer,
            debit,
            credit,
            debit_cents: cents,
            credit_cents: cents + delta_cents,
            good: ONLY_GOOD,
            units_out: 0,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });

        *self
            .journal
            .last()
            .expect("the recorder pushed the posting it was just handed")
    }

    /// Adjust `who`'s cash by `delta_cents` and record **nothing**.
    ///
    /// The case the journal genuinely cannot localise. Every posting in the
    /// tick conserves, so every running residual is zero and there is no
    /// offending posting to name. It exists so that the optional posting on a
    /// conservation violation is exercised rather than theoretical, and so the
    /// message that says the books were changed outside the posting path is
    /// read by a test rather than only written by a developer.
    ///
    /// Panics if `who` names no account in these books.
    pub(crate) fn corrupt_silent_cash(&mut self, who: Account, delta_cents: i64) {
        let slot = self
            .resolve(who)
            .expect("a corruption names an account these books hold");
        let adjusted = self.cash_at(slot).cents() + delta_cents;
        self.write_cash(slot, Money::from_cents(adjusted));
    }

    /// Move `cents` from `from` to `to` by direct field writes, with no
    /// overdraft check and no posting.
    ///
    /// **The total stays exactly intact and one account ends below zero.** This
    /// is the corruption that proves money conservation and non-negativity are
    /// independent checks rather than two names for one condition: the books
    /// hold the opening stock to the cent, so only the second of the two can
    /// fire.
    ///
    /// Panics if either address names no account in these books, or if the two
    /// name the same one, for the reason [`Books::corrupt_recorded_cash`] gives.
    pub(crate) fn corrupt_conserving_deficit(&mut self, from: Account, to: Account, cents: i64) {
        assert_ne!(
            from, to,
            "a corruption naming one account on both legs would write that \
             account twice and lose the first write"
        );
        let from_slot = self
            .resolve(from)
            .expect("a corruption names an account these books hold");
        let to_slot = self
            .resolve(to)
            .expect("a corruption names an account these books hold");

        let drained = self.cash_at(from_slot).cents() - cents;
        let filled = self.cash_at(to_slot).cents() + cents;
        self.write_cash(from_slot, Money::from_cents(drained));
        self.write_cash(to_slot, Money::from_cents(filled));
    }

    /// Append `draft` to the journal without touching any balance, stamping its
    /// sequence number and both running residuals through the recorder.
    /// Returns the posting as it was stamped.
    ///
    /// This is what lets a test synthesise a malformed posting for the zero-sum
    /// check. No public operation can record one — that is the point of the
    /// rest of this file — and it is why the structural check is testable in a
    /// phase with no economic notion of a sale.
    ///
    /// The `seq` and both residual fields on `draft` are placeholders and are
    /// overwritten, exactly as they are for a posting a real operation records.
    pub(crate) fn corrupt_appended_posting(&mut self, draft: Posting) -> Posting {
        self.record(draft);
        *self
            .journal
            .last()
            .expect("the recorder pushed the posting it was just handed")
    }
}

/// The corruption vocabulary itself, in isolation from any invariant.
///
/// Named `corrupt` so that a `books::corrupt` module-path filter selects
/// exactly these.
///
/// **These are not the negative tests.** Those live in `src/invariants.rs` and
/// assert which violation a seeded fault produces. These assert that each
/// corruption had the effect its documentation claims — the balances moved by
/// the stated amount, the journal grew by the stated number of postings, and
/// the running residual moved by the stated amount — which is what stops a
/// broken corruption from producing a green negative test for the wrong reason.
#[cfg(test)]
mod corrupt {
    use super::*;
    use std::path::Path;

    /// The one good v1 carries.
    const FOOD: GoodId = GoodId(0);

    /// The shipped parameters, loaded through the real deserialisation path so
    /// these tests cannot drift from the configuration the binary runs on.
    fn shipped() -> Params {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
        let (params, _hash) = crate::config::load(&path).expect("the shipped configuration loads");
        params
    }

    fn books() -> Books {
        Books::new(&shipped()).expect("the shipped endowment sums to the stock")
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

    fn cash(books: &Books, account: Account) -> i64 {
        books
            .cash_of(account)
            .expect("the account is one these books hold")
            .cents()
    }

    #[test]
    fn a_recorded_corruption_moves_the_balances_the_journal_and_the_residual_together() {
        let mut books = books();
        let opening_total = books.total_money().cents();
        let payer = cash(&books, firm(0));
        let payee = cash(&books, household(0));
        assert_eq!(books.cash_residual_cents(), 0, "the books open conserving");

        let posting = books.corrupt_recorded_cash(firm(0), household(0), 100, -1);

        // The balances moved by the stated amounts: a hundred cents left the
        // payer and ninety-nine arrived, so the total is one cent short.
        assert_eq!(cash(&books, firm(0)), payer - 100);
        assert_eq!(cash(&books, household(0)), payee + 99);
        assert_eq!(books.total_money().cents(), opening_total - 1);

        // The journal grew by exactly one posting, and it is the one returned.
        assert_eq!(books.journal().len(), 1);
        assert_eq!(books.journal().first().copied(), Some(posting));

        // The recorder stamped it, so the residual under test is the production
        // residual rather than a number this method wrote.
        assert_eq!(posting.seq, 0);
        assert_eq!(posting.debit_cents, 100);
        assert_eq!(posting.credit_cents, 99);
        assert_eq!(posting.cash_residual_cents, -1);
        assert_eq!(books.cash_residual_cents(), -1);

        // A cash corruption moves no units.
        assert_eq!(books.goods_residual_units(), 0);
        assert_eq!(posting.goods_residual_units, 0);
    }

    #[test]
    fn two_equal_and_opposite_recorded_corruptions_heal_the_residual() {
        // The property the localisation test rests on: a residual that returns
        // to zero part way through a tick, which is what makes a halving search
        // over the journal unsound.
        let mut books = books();
        let opening_total = books.total_money().cents();

        let broke = books.corrupt_recorded_cash(firm(0), household(0), 100, 1);
        assert_eq!(broke.cash_residual_cents, 1);
        assert_eq!(books.cash_residual_cents(), 1);
        assert_eq!(books.total_money().cents(), opening_total + 1);

        let healed = books.corrupt_recorded_cash(firm(0), household(0), 100, -1);
        assert_eq!(healed.cash_residual_cents, 0);
        assert_eq!(books.cash_residual_cents(), 0);
        assert_eq!(books.total_money().cents(), opening_total);

        assert_eq!(
            books.journal().len(),
            2,
            "both postings survive the healing"
        );
        assert_eq!(healed.seq, 1);
    }

    #[test]
    fn a_silent_corruption_moves_a_balance_and_records_nothing() {
        let mut books = books();
        let opening_total = books.total_money().cents();
        let before = cash(&books, household(0));

        books.corrupt_silent_cash(household(0), -1);

        assert_eq!(cash(&books, household(0)), before - 1);
        assert_eq!(books.total_money().cents(), opening_total - 1);

        // The whole point of this corruption: nothing in the journal describes
        // it, so no posting can be named for it.
        assert!(books.journal().is_empty());
        assert_eq!(books.cash_residual_cents(), 0);
        assert_eq!(books.transactions_this_tick(), 0);
    }

    #[test]
    fn a_conserving_deficit_drives_one_account_below_zero_and_leaves_the_total_intact() {
        let mut books = books();
        let opening_total = books.total_money().cents();
        let opening = cash(&books, household(0));
        let taker = cash(&books, firm(0));

        books.corrupt_conserving_deficit(household(0), firm(0), opening + 250);

        assert_eq!(
            cash(&books, household(0)),
            -250,
            "no overdraft check applied"
        );
        assert_eq!(cash(&books, firm(0)), taker + opening + 250);
        assert_eq!(
            books.total_money().cents(),
            opening_total,
            "the total is intact, which is what makes this corruption a test of \
             non-negativity and not of conservation"
        );
        assert!(books.journal().is_empty());
        assert_eq!(books.cash_residual_cents(), 0);
    }

    #[test]
    fn an_appended_posting_touches_no_balance_and_is_stamped_by_the_recorder() {
        let mut books = books();
        let opening_total = books.total_money().cents();
        let opening_stock = books.total_stock(FOOD);

        // Placeholder values on the three fields the recorder owns, so the test
        // can tell a stamped posting from the draft it was handed.
        let draft = Posting {
            seq: 99,
            kind: PostingKind::Transfer,
            debit: household(0),
            credit: firm(0),
            debit_cents: 500,
            credit_cents: 500,
            good: FOOD,
            units_out: 3,
            units_in: 3,
            cash_residual_cents: 77,
            goods_residual_units: 88,
        };
        let posting = books.corrupt_appended_posting(draft);

        assert_eq!(posting.seq, 0, "the recorder stamps the sequence number");
        assert_eq!(posting.cash_residual_cents, 0, "the cash legs agree");
        assert_eq!(posting.goods_residual_units, 0, "the unit legs agree");
        assert_eq!(books.journal().len(), 1);
        assert_eq!(books.journal().first().copied(), Some(posting));

        // No balance moved: this corruption is a journal-only fault, which is
        // what makes it a test of the structural check alone.
        assert_eq!(books.total_money().cents(), opening_total);
        assert_eq!(books.total_stock(FOOD), opening_stock);
        assert_eq!(books.cash_residual_cents(), 0);
        assert_eq!(books.goods_residual_units(), 0);

        // A transfer counts towards the liveness minimum however malformed it
        // is; that is the recorder's rule and this corruption does not bypass it.
        assert_eq!(books.transactions_this_tick(), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The shipped parameters, loaded through the real deserialisation path so
    /// these tests cannot drift from the configuration the binary runs on.
    fn shipped() -> Params {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
        let (params, _hash) = crate::config::load(&path).expect("the shipped configuration loads");
        params
    }

    fn household(index: u32) -> Account {
        Account::Household(HouseholdId(index))
    }

    fn firm(slot: u16, generation: u32) -> Account {
        Account::Firm(FirmId {
            slot: FirmSlot(slot),
            generation,
        })
    }

    #[test]
    fn construction_endows_every_agent_and_conserves_the_configured_stock() {
        let params = shipped();
        let books = Books::new(&params).expect("the shipped endowment sums to the stock");

        assert_eq!(
            books.total_money().cents(),
            params.money.total_money_cents,
            "the balances must sum to the configured stock"
        );
        assert_eq!(
            books.opening_stock().cents(),
            params.money.total_money_cents
        );
        assert_eq!(books.cash_residual_cents(), 0);
        assert_eq!(
            books.firm_cash_total().cents(),
            i64::from(params.sim.firms) * params.firm.initial_liquidity_cents
        );
        assert_eq!(
            books.cash_of(household(0)),
            Some(Money::from_cents(params.household.initial_liquidity_cents))
        );
        assert_eq!(
            books.cash_of(firm(0, 0)),
            Some(Money::from_cents(params.firm.initial_liquidity_cents))
        );
    }

    #[test]
    fn tick_zero_begins_with_an_empty_journal() {
        // If the endowment postings survived into tick 0, the liveness check
        // could pass on the strength of the endowment alone — the degenerate
        // pass LEDG-08 exists to close.
        let books = Books::new(&shipped()).expect("the shipped endowment sums to the stock");
        assert!(books.journal().is_empty());
        assert_eq!(books.transactions_this_tick(), 0);
    }

    #[test]
    fn an_endowment_that_does_not_sum_to_the_stock_is_refused_at_construction() {
        let mut params = shipped();
        params.money.total_money_cents += 1;

        let expected_endowment = i64::from(params.sim.households)
            * params.household.initial_liquidity_cents
            + i64::from(params.sim.firms) * params.firm.initial_liquidity_cents;

        assert_eq!(
            Books::new(&params).err(),
            Some(BooksError::EndowmentDoesNotSumToStock {
                endowed_cents: expected_endowment,
                opening_cents: params.money.total_money_cents,
            })
        );
    }

    #[test]
    fn the_account_walk_is_households_ascending_then_firm_slots_ascending() {
        // The order is part of the invariant contract: two accounts can be
        // negative at once, and a check that reported an arbitrary one of them
        // would make its own negative test flaky in a way indistinguishable
        // from a real failure. Asserted on the sequence, never on the length.
        let params = shipped();
        let books = Books::new(&params).expect("the shipped endowment sums to the stock");
        let walked: Vec<Account> = books.accounts().collect();

        let mut expected: Vec<Account> = (0..params.sim.households).map(household).collect();
        expected.extend((0..params.sim.firms).map(|slot| {
            firm(
                u16::try_from(slot).expect("the shipped run has at most u16::MAX slots"),
                0,
            )
        }));

        assert_eq!(walked, expected);
        assert_eq!(
            walked.first().copied(),
            Some(household(0)),
            "households are walked first"
        );
        assert_eq!(
            walked.last().copied(),
            Some(firm(
                u16::try_from(params.sim.firms - 1).expect("at most u16::MAX slots"),
                0
            )),
            "and firm slots last, in ascending slot order"
        );

        let mut sorted = walked.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted, walked,
            "the walk agrees with the derived total order src/ids.rs already carries"
        );
        for account in &walked {
            assert!(
                books.cash_of(*account).is_some(),
                "every walked address resolves: {account}"
            );
        }
    }

    #[test]
    fn a_transfer_moves_the_amount_reports_it_and_conserves_the_total() {
        let params = shipped();
        let mut books = Books::new(&params).expect("the shipped endowment sums to the stock");
        let before_payer = books.cash_of(household(0)).expect("household 0 exists");
        let before_payee = books.cash_of(firm(0, 0)).expect("firm slot 0 exists");

        let moved = books
            .transfer(household(0), firm(0, 0), Money::from_cents(700))
            .expect("a household with an endowment can pay 700 cents");

        assert_eq!(moved, Money::from_cents(700));
        assert_eq!(
            books.cash_of(household(0)),
            Some(Money::from_cents(before_payer.cents() - 700))
        );
        assert_eq!(
            books.cash_of(firm(0, 0)),
            Some(Money::from_cents(before_payee.cents() + 700))
        );
        assert_eq!(books.total_money().cents(), params.money.total_money_cents);
        assert_eq!(books.cash_residual_cents(), 0);
        assert_eq!(books.transactions_this_tick(), 1);

        let posting = books.journal().first().copied().expect("one posting");
        assert_eq!(posting.seq, 0);
        assert_eq!(posting.kind, PostingKind::Transfer);
        assert_eq!(posting.debit, household(0));
        assert_eq!(posting.credit, firm(0, 0));
        assert_eq!(posting.debit_cents, 700);
        assert_eq!(posting.credit_cents, 700);
        assert_eq!(posting.cash_residual_cents, 0);
        assert_eq!(
            posting.to_string(),
            "#0 transfer household:0 -> firm:0:0 debit 700c credit 700c good:0 out 0 in 0"
        );
    }

    #[test]
    fn every_refusal_leaves_the_books_exactly_as_it_found_them() {
        let params = shipped();
        let mut books = Books::new(&params).expect("the shipped endowment sums to the stock");
        let untouched = books.clone();
        let endowment = params.household.initial_liquidity_cents;

        let refusals = [
            (
                books.transfer(household(0), firm(0, 0), Money::from_cents(endowment + 1)),
                PostError::Overdraft {
                    account: household(0),
                    amount_cents: endowment + 1,
                    balance_cents: endowment,
                },
            ),
            (
                books.transfer(household(0), firm(0, 0), Money::from_cents(-1)),
                PostError::NegativeAmount { amount_cents: -1 },
            ),
            (
                // A transfer that moves nothing would still count towards the
                // liveness minimum, which is the degenerate pass LEDG-08 exists
                // to close — the same hole the `exchange` sibling below
                // enumerates three cases for. Refused here, and reported by the
                // zero-sum check if one ever reaches the journal.
                books.transfer(household(0), firm(0, 0), Money::ZERO),
                PostError::EmptyTransfer { amount_cents: 0 },
            ),
            (
                // Zero *and* self-dealing: the empty-leg clause is evaluated
                // first, so which refusal is reported is fixed rather than
                // incidental.
                books.transfer(household(0), household(0), Money::ZERO),
                PostError::EmptyTransfer { amount_cents: 0 },
            ),
            (
                books.transfer(household(0), household(0), Money::from_cents(1)),
                PostError::SelfDealing {
                    account: household(0),
                },
            ),
            (
                books.transfer(household(9_999), firm(0, 0), Money::from_cents(1)),
                PostError::UnknownAccount(household(9_999)),
            ),
            (
                // A firm identity from a generation that no longer occupies the
                // slot is a typed miss, never a silent hit on the successor.
                books.transfer(household(0), firm(0, 1), Money::from_cents(1)),
                PostError::UnknownAccount(firm(0, 1)),
            ),
        ];

        for (actual, expected) in refusals {
            assert_eq!(actual, Err(expected));
        }

        assert_eq!(books.total_money(), untouched.total_money());
        assert_eq!(books.cash_of(household(0)), untouched.cash_of(household(0)));
        assert_eq!(books.cash_of(firm(0, 0)), untouched.cash_of(firm(0, 0)));
        assert_eq!(books.cash_residual_cents(), 0);
        assert!(books.journal().is_empty(), "a refusal writes nothing");
        assert_eq!(books.transactions_this_tick(), 0);
    }

    #[test]
    fn ending_a_tick_clears_the_journal_and_the_count_but_not_the_residual() {
        let mut books = Books::new(&shipped()).expect("the shipped endowment sums to the stock");
        books
            .transfer(household(0), firm(0, 0), Money::from_cents(1))
            .expect("a household with an endowment can pay a cent");
        books
            .transfer(household(1), firm(0, 0), Money::from_cents(1))
            .expect("a household with an endowment can pay a cent");
        assert_eq!(books.journal().len(), 2);
        assert_eq!(books.journal()[1].seq, 1);
        assert_eq!(books.transactions_this_tick(), 2);

        let residual_before = books.cash_residual_cents();
        books.end_of_tick();

        assert!(books.journal().is_empty());
        assert_eq!(books.transactions_this_tick(), 0);
        assert_eq!(books.cash_residual_cents(), residual_before);

        books
            .transfer(household(2), firm(0, 0), Money::from_cents(1))
            .expect("a household with an endowment can pay a cent");
        assert_eq!(books.journal()[0].seq, 0, "the sequence restarts each tick");
    }

    #[test]
    fn ending_a_tick_leaves_a_seeded_non_zero_residual_of_either_kind_untouched() {
        // **This test exists because the property above it cannot fail.**
        // `tests/ledger_props.rs::ending_a_tick_leaves_the_residuals_and_the_
        // balances_untouched` asserts the same claim over arbitrary operation
        // sequences and was mutation-checked in plan 02-07: adding
        // `self.cash_residual_cents = 0;` to `end_of_tick` left it green. The
        // reason is structural rather than a defect in the property. On the
        // honest path the books conserve, so both residuals are ALREADY zero at
        // every tick boundary, and zeroing a zero changes nothing observable.
        //
        // Making it observable needs a residual that is not zero, which needs
        // the fault-injection vocabulary — and that vocabulary is visible to
        // this crate's own unit tests only, which is exactly why the version
        // with teeth belongs here and not under `tests/`. Recorded as an
        // unmet truth in `.planning/WINDOWS.md` and discharged by this test.
        //
        // Mutation-verified in plan 02-06, in both build profiles: with
        // `self.cash_residual_cents = 0;` added to `end_of_tick` this test
        // fails and the integration property still passes.
        let mut books = Books::new(&shipped()).expect("the shipped endowment sums to the stock");

        // An over-credited posting: the balances and the journal residual move
        // together, leaving the running cash residual at one cent.
        books.corrupt_recorded_cash(firm(0, 0), household(0), 100, 1);

        // A posting whose two unit legs disagree, leaving the running goods
        // residual at two units. A second, independent residual, because a
        // boundary that resets one and not the other is a different defect.
        books.corrupt_appended_posting(Posting {
            seq: 0,
            kind: PostingKind::Exchange,
            debit: household(0),
            credit: firm(0, 0),
            debit_cents: 500,
            credit_cents: 500,
            good: ONLY_GOOD,
            units_out: 3,
            units_in: 1,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });

        assert_eq!(books.cash_residual_cents(), 1, "the seeded cash residual");
        assert_eq!(books.goods_residual_units(), 2, "the seeded goods residual");
        assert_eq!(books.journal().len(), 2);
        assert_eq!(books.transactions_this_tick(), 2);

        books.end_of_tick();

        // The claim with teeth: a residual the honest path cannot produce
        // survives the boundary unchanged. A tick boundary that reset either
        // one would silently disable conservation from the next tick onward,
        // because the residual is measured against the run's opening stock and
        // is meaningful only cumulatively.
        assert_eq!(
            books.cash_residual_cents(),
            1,
            "the tick boundary reset the cash residual"
        );
        assert_eq!(
            books.goods_residual_units(),
            2,
            "the tick boundary reset the goods residual"
        );

        // What the boundary does reset, asserted in the same test so that a
        // change which preserves the residuals by preserving everything is not
        // mistaken for a pass.
        assert!(books.journal().is_empty());
        assert_eq!(books.transactions_this_tick(), 0);

        // A second boundary does not drift them either: the residuals are not
        // merely "reset once", they are never written here at all.
        books.end_of_tick();
        assert_eq!(books.cash_residual_cents(), 1);
        assert_eq!(books.goods_residual_units(), 2);
    }

    #[test]
    fn the_two_residual_sources_move_apart_when_only_one_of_them_is_told() {
        // **This test exists because the property it mirrors cannot fail.**
        // `tests/ledger_props.rs::posting_residuals_agree_with_the_balance_
        // derived_quantities` asserts the same design over arbitrary operation
        // sequences and is documented there as the property that states it
        // directly. From an integration test it is structurally `0 == 0`.
        //
        // The reason is the same shape as the tick-boundary residual above.
        // `record` derives `cash_delta` as `credit_cents - debit_cents`, and
        // every public operation constructs its posting with ONE value on both
        // cash legs — `transfer` writes `debit_cents: amount.cents(),
        // credit_cents: amount.cents()`, `exchange` the same, `produce` and
        // `consume` write `0`/`0`. `goods_delta` has the same shape. So on any
        // sequence an ordinary caller can produce, both sides of both
        // comparisons are invariantly zero and the assertion is `0 == 0`.
        //
        // **What has teeth is the two sources being able to DISAGREE.** The
        // journal residual is not a restatement of the balances; it is what the
        // postings say, and a posting that says something the balances do not
        // is exactly the leak `check_money` exists to report. So this test
        // appends postings whose legs disagree, touching no balance and no
        // stock, and asserts that the posting-derived residual moves by exactly
        // what the legs say while the balance-derived one does not move at all.
        //
        // Mutation-verified in both build profiles against the single-source
        // collapse the property's own doc comment names: replacing `record`'s
        // residual arithmetic with
        //
        //     let cash_delta = self.total_money().cents()
        //         - self.opening_stock.cents()
        //         - self.cash_residual_cents;
        //
        // fails this test and leaves every property in `tests/ledger_props.rs`
        // green — because that collapse computes the SAME number on the honest
        // path and on every corruption that moves the balances too.
        let mut books = Books::new(&shipped()).expect("the shipped endowment sums to the stock");

        // The two sources, named once. The first is accumulated from the legs
        // of the postings; the second is recomputed from the balance vectors
        // against the configured opening stock. Neither is derived from the
        // other, and that is the whole claim.
        let posting_derived = |books: &Books| books.cash_residual_cents();
        let balance_derived =
            |books: &Books| books.total_money().cents() - books.opening_stock().cents();

        assert_eq!(posting_derived(&books), 0, "the books open conserving");
        assert_eq!(balance_derived(&books), 0);

        // A journal-only over-credit: five hundred cents left the debit account
        // and five hundred and one arrived, according to the posting. No
        // balance moved.
        books.corrupt_appended_posting(Posting {
            seq: 0,
            kind: PostingKind::Transfer,
            debit: household(0),
            credit: firm(0, 0),
            debit_cents: 500,
            credit_cents: 501,
            good: ONLY_GOOD,
            units_out: 0,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });

        assert_eq!(
            posting_derived(&books),
            1,
            "the postings say a cent was created, and the residual is read off \
             the legs of the posting rather than off the balances"
        );
        assert_eq!(
            balance_derived(&books),
            0,
            "no balance moved, so the balance-derived residual is still zero"
        );
        assert_ne!(
            posting_derived(&books),
            balance_derived(&books),
            "the two sources are independent: one can move while the other does \
             not, which is what makes check_money a comparison rather than a \
             tautology"
        );

        // The goods side of the same claim, and it is the same test. A posting
        // whose unit legs disagree moves the goods residual and touches no
        // stock vector, so the two goods sources part company too.
        let stock_before = books.total_stock(ONLY_GOOD);
        books.corrupt_appended_posting(Posting {
            seq: 0,
            kind: PostingKind::Exchange,
            debit: household(0),
            credit: firm(0, 0),
            debit_cents: 400,
            credit_cents: 400,
            good: ONLY_GOOD,
            units_out: 3,
            units_in: 1,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        });

        assert_eq!(
            books.goods_residual_units(),
            2,
            "three units left and one arrived, so the postings say two units \
             are unaccounted for"
        );
        assert_eq!(
            books.produced(ONLY_GOOD) - books.consumed(ONLY_GOOD) - books.total_stock(ONLY_GOOD),
            0,
            "no stock moved, so the identity recomputed from the fields is \
             still zero"
        );
        assert_eq!(books.total_stock(ONLY_GOOD), stock_before);

        // The complementary direction, on the one corruption that moves the
        // balances and the journal TOGETHER: there the two sources are required
        // to agree, and the agreement is on a non-zero number rather than on
        // zero. `corrupt_recorded_cash` drops a cent — a hundred leaves the
        // payer, ninety-nine arrives — so both sources move by -1 from where
        // they were.
        let posting_before = posting_derived(&books);
        let balance_before = balance_derived(&books);
        books.corrupt_recorded_cash(firm(0, 0), household(0), 100, -1);

        assert_eq!(
            posting_derived(&books),
            posting_before - 1,
            "the postings say one more cent is gone"
        );
        assert_eq!(
            balance_derived(&books),
            balance_before - 1,
            "and the balances say the same, because this corruption wrote both"
        );
    }

    #[test]
    fn a_posting_serialises_with_rendered_addresses_and_integer_amounts() {
        // Phase 3 writes this shape into its event stream; it is pinned here so
        // a change to it is a reviewed diff rather than a silent one.
        let posting = Posting {
            seq: 3,
            kind: PostingKind::Transfer,
            debit: household(12),
            credit: firm(3, 1),
            debit_cents: 250,
            credit_cents: 250,
            good: ONLY_GOOD,
            units_out: 0,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        };
        let rendered = toml::to_string(&posting).expect("a posting serialises");
        assert!(rendered.contains("debit = \"household:12\""), "{rendered}");
        assert!(rendered.contains("credit = \"firm:3:1\""), "{rendered}");
        assert!(rendered.contains("kind = \"transfer\""), "{rendered}");
        assert!(rendered.contains("good = 0"), "{rendered}");
    }
}

/// The goods half of the ledger, at unit granularity.
///
/// Named `goods` so that a `books::goods` module-path filter selects exactly
/// these. Every refusal below is asserted against a clone of the books taken
/// before the attempt, because "it returned an error" and "it wrote nothing"
/// are two different claims and only the second is what compute-then-commit
/// promises.
#[cfg(test)]
mod goods {
    use super::*;
    use std::path::Path;

    /// The shipped parameters, loaded through the real deserialisation path so
    /// these tests cannot drift from the configuration the binary runs on.
    fn shipped() -> Params {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
        let (params, _hash) = crate::config::load(&path).expect("the shipped configuration loads");
        params
    }

    fn household(index: u32) -> Account {
        Account::Household(HouseholdId(index))
    }

    fn firm(slot: u16, generation: u32) -> Account {
        Account::Firm(FirmId {
            slot: FirmSlot(slot),
            generation,
        })
    }

    /// The recomputed identity: produced, less consumed, less every unit held.
    /// The side that comes from the fields rather than from the journal.
    fn identity(books: &Books) -> i64 {
        books.produced(ONLY_GOOD) - books.consumed(ONLY_GOOD) - books.total_stock(ONLY_GOOD)
    }

    #[test]
    fn construction_endows_inventory_and_counts_it_into_produced() {
        // Without the count into `produced`, the identity fails on tick 0 by
        // exactly the endowment. This is the test for that specific defect.
        let params = shipped();
        let books = Books::new(&params).expect("the shipped configuration opens the books");
        let per_firm = params.firm.initial_inventory_units;
        let expected = i64::from(params.sim.firms) * per_firm;

        assert_eq!(books.stock_of(firm(0, 0), ONLY_GOOD), Some(per_firm));
        assert_eq!(books.stock_of(household(0), ONLY_GOOD), Some(0));
        assert_eq!(books.total_stock(ONLY_GOOD), expected);
        assert_eq!(books.produced(ONLY_GOOD), expected);
        assert_eq!(books.consumed(ONLY_GOOD), 0);
        assert_eq!(identity(&books), 0, "the identity holds at tick 0");
        assert_eq!(books.goods_residual_units(), 0);
        assert!(books.journal().is_empty(), "tick 0 begins empty");
    }

    #[test]
    fn a_negative_initial_inventory_is_refused_at_construction() {
        // The identity cannot catch this one: a negative endowment gives a
        // negative `produced` against negative stock and balances perfectly.
        let mut params = shipped();
        params.firm.initial_inventory_units = -1;

        assert_eq!(
            Books::new(&params).err(),
            Some(BooksError::InitialInventoryOutOfRange {
                units_per_firm: -1,
                firms: u16::try_from(params.sim.firms).expect("the shipped firm count fits"),
            })
        );
    }

    #[test]
    fn production_raises_both_the_stock_and_the_produced_total() {
        let mut books = Books::new(&shipped()).expect("the shipped configuration opens the books");
        let stock_before = books
            .stock_of(firm(0, 0), ONLY_GOOD)
            .expect("slot 0 exists");
        let produced_before = books.produced(ONLY_GOOD);

        let made = books
            .produce(firm(0, 0), ONLY_GOOD, 40)
            .expect("a firm can produce");

        assert_eq!(made, 40, "the units actually created are reported");
        assert_eq!(
            books.stock_of(firm(0, 0), ONLY_GOOD),
            Some(stock_before + 40)
        );
        assert_eq!(books.produced(ONLY_GOOD), produced_before + 40);
        assert_eq!(identity(&books), 0);
        assert_eq!(books.goods_residual_units(), 0);

        let posting = books.journal().first().copied().expect("one posting");
        assert_eq!(posting.kind, PostingKind::Produce);
        assert_eq!(posting.units_in, 40);
        assert_eq!(posting.units_out, 0);
        assert_eq!(posting.debit_cents, 0);
        assert_eq!(posting.goods_residual_units, 0);
        assert_eq!(
            posting.to_string(),
            "#0 produce firm:0:0 -> firm:0:0 debit 0c credit 0c good:0 out 0 in 40"
        );
    }

    #[test]
    fn consumption_lowers_the_stock_raises_the_consumed_total_and_posts() {
        let mut books = Books::new(&shipped()).expect("the shipped configuration opens the books");
        let held = books
            .stock_of(firm(0, 0), ONLY_GOOD)
            .expect("slot 0 exists");

        let eaten = books
            .consume(firm(0, 0), ONLY_GOOD, 5)
            .expect("a firm holding stock can consume from it");

        assert_eq!(eaten, 5);
        assert_eq!(books.stock_of(firm(0, 0), ONLY_GOOD), Some(held - 5));
        assert_eq!(books.consumed(ONLY_GOOD), 5);
        assert_eq!(identity(&books), 0);
        assert_eq!(books.goods_residual_units(), 0);

        // A real posting, not a bare subtraction: LEDG-09 must be able to name
        // the line, and MKT-06 asks for consumption as a modelled step.
        let posting = books.journal().first().copied().expect("one posting");
        assert_eq!(posting.kind, PostingKind::Consume);
        assert_eq!(posting.units_out, 5);
        assert_eq!(posting.units_in, 0);
    }

    #[test]
    fn consuming_beyond_the_stock_is_refused_and_writes_nothing() {
        let mut books = Books::new(&shipped()).expect("the shipped configuration opens the books");
        let untouched = books.clone();
        let held = books
            .stock_of(firm(0, 0), ONLY_GOOD)
            .expect("slot 0 exists");

        assert_eq!(
            books.consume(firm(0, 0), ONLY_GOOD, held + 1),
            Err(PostError::ShortStock {
                account: firm(0, 0),
                good: ONLY_GOOD,
                units_requested: held + 1,
                units_held: held,
            })
        );

        assert_eq!(books.stock_of(firm(0, 0), ONLY_GOOD), Some(held));
        assert_eq!(books.consumed(ONLY_GOOD), untouched.consumed(ONLY_GOOD));
        assert_eq!(
            books.total_stock(ONLY_GOOD),
            untouched.total_stock(ONLY_GOOD)
        );
        assert!(books.journal().is_empty(), "a refusal writes nothing");
    }

    #[test]
    fn a_completed_exchange_moves_both_and_reports_both() {
        let params = shipped();
        let mut books = Books::new(&params).expect("the shipped configuration opens the books");
        let buyer = household(0);
        let seller = firm(0, 0);
        let buyer_cash = books.cash_of(buyer).expect("household 0 exists");
        let seller_cash = books.cash_of(seller).expect("slot 0 exists");
        let seller_stock = books.stock_of(seller, ONLY_GOOD).expect("slot 0 exists");

        let (paid, received) = books
            .exchange(buyer, seller, ONLY_GOOD, 3, Money::from_cents(315))
            .expect("an endowed household can buy three units");

        assert_eq!(paid, Money::from_cents(315));
        assert_eq!(received, 3);
        assert_eq!(
            books.cash_of(buyer),
            Some(Money::from_cents(buyer_cash.cents() - 315))
        );
        assert_eq!(
            books.cash_of(seller),
            Some(Money::from_cents(seller_cash.cents() + 315))
        );
        assert_eq!(books.stock_of(buyer, ONLY_GOOD), Some(3));
        assert_eq!(
            books.stock_of(seller, ONLY_GOOD),
            Some(seller_stock - 3),
            "the units came out of the seller's inventory"
        );

        // Both conservation properties, from both sources.
        assert_eq!(books.total_money().cents(), params.money.total_money_cents);
        assert_eq!(books.cash_residual_cents(), 0);
        assert_eq!(identity(&books), 0);
        assert_eq!(books.goods_residual_units(), 0);

        // One posting, never two: two could be half-applied.
        assert_eq!(books.journal().len(), 1);
        let posting = books.journal()[0];
        assert_eq!(posting.kind, PostingKind::Exchange);
        assert_eq!(posting.debit, buyer, "the buyer pays, so it is the debit");
        assert_eq!(posting.credit, seller);
        assert_eq!(posting.debit_cents, 315);
        assert_eq!(posting.credit_cents, 315);
        assert_eq!(posting.units_out, 3, "units left the credit account");
        assert_eq!(posting.units_in, 3, "units arrived at the debit account");
    }

    #[test]
    fn every_refused_exchange_moves_neither_cash_nor_units() {
        let params = shipped();
        let mut books = Books::new(&params).expect("the shipped configuration opens the books");
        let untouched = books.clone();
        let buyer = household(0);
        let seller = firm(0, 0);
        let liquidity = params.household.initial_liquidity_cents;
        let seller_stock = books.stock_of(seller, ONLY_GOOD).expect("slot 0 exists");

        let refusals = [
            (
                books.exchange(
                    buyer,
                    seller,
                    ONLY_GOOD,
                    1,
                    Money::from_cents(liquidity + 1),
                ),
                PostError::Overdraft {
                    account: buyer,
                    amount_cents: liquidity + 1,
                    balance_cents: liquidity,
                },
            ),
            (
                books.exchange(
                    buyer,
                    seller,
                    ONLY_GOOD,
                    seller_stock + 1,
                    Money::from_cents(1),
                ),
                PostError::ShortStock {
                    account: seller,
                    good: ONLY_GOOD,
                    units_requested: seller_stock + 1,
                    units_held: seller_stock,
                },
            ),
            (
                books.exchange(buyer, seller, ONLY_GOOD, 1, Money::from_cents(-1)),
                PostError::NegativeAmount { amount_cents: -1 },
            ),
            (
                books.exchange(buyer, seller, ONLY_GOOD, -1, Money::from_cents(1)),
                PostError::NegativeUnits { units: -1 },
            ),
            (
                // An exchange that moves nothing would still count towards the
                // liveness minimum, which is the degenerate pass LEDG-08 exists
                // to close. Refused here, and reported by the zero-sum check if
                // one ever reaches the journal.
                books.exchange(buyer, seller, ONLY_GOOD, 0, Money::from_cents(0)),
                PostError::EmptyExchange {
                    units: 0,
                    amount_cents: 0,
                },
            ),
            (
                books.exchange(buyer, seller, ONLY_GOOD, 2, Money::from_cents(0)),
                PostError::EmptyExchange {
                    units: 2,
                    amount_cents: 0,
                },
            ),
            (
                books.exchange(buyer, seller, ONLY_GOOD, 0, Money::from_cents(50)),
                PostError::EmptyExchange {
                    units: 0,
                    amount_cents: 50,
                },
            ),
            (
                books.exchange(buyer, buyer, ONLY_GOOD, 1, Money::from_cents(1)),
                PostError::SelfDealing { account: buyer },
            ),
            (
                books.exchange(buyer, seller, GoodId(7), 1, Money::from_cents(1)),
                PostError::UnknownGood(GoodId(7)),
            ),
            (
                books.exchange(household(9_999), seller, ONLY_GOOD, 1, Money::from_cents(1)),
                PostError::UnknownAccount(household(9_999)),
            ),
            (
                books.exchange(buyer, firm(0, 1), ONLY_GOOD, 1, Money::from_cents(1)),
                PostError::UnknownAccount(firm(0, 1)),
            ),
        ];

        for (actual, expected) in refusals {
            assert_eq!(actual, Err(expected));
        }

        assert_eq!(books.total_money(), untouched.total_money());
        assert_eq!(books.cash_of(buyer), untouched.cash_of(buyer));
        assert_eq!(books.cash_of(seller), untouched.cash_of(seller));
        assert_eq!(
            books.total_stock(ONLY_GOOD),
            untouched.total_stock(ONLY_GOOD)
        );
        assert_eq!(books.stock_of(seller, ONLY_GOOD), Some(seller_stock));
        assert_eq!(books.stock_of(buyer, ONLY_GOOD), Some(0));
        assert_eq!(books.produced(ONLY_GOOD), untouched.produced(ONLY_GOOD));
        assert_eq!(books.consumed(ONLY_GOOD), untouched.consumed(ONLY_GOOD));
        assert!(books.journal().is_empty(), "a refusal writes nothing");
        assert_eq!(books.transactions_this_tick(), 0);
    }

    #[test]
    fn an_unknown_good_is_refused_rather_than_indexed() {
        let mut books = Books::new(&shipped()).expect("the shipped configuration opens the books");
        let untouched = books.clone();
        let missing = GoodId(7);

        assert_eq!(
            books.produce(firm(0, 0), missing, 1),
            Err(PostError::UnknownGood(missing))
        );
        assert_eq!(
            books.consume(firm(0, 0), missing, 1),
            Err(PostError::UnknownGood(missing))
        );
        assert_eq!(books.stock_of(firm(0, 0), missing), None);

        // Zero, and true rather than a fallback: no unit of a good the books do
        // not carry can ever have been produced, consumed or held.
        assert_eq!(books.total_stock(missing), 0);
        assert_eq!(books.produced(missing), 0);
        assert_eq!(books.consumed(missing), 0);

        assert_eq!(books.produced(ONLY_GOOD), untouched.produced(ONLY_GOOD));
        assert_eq!(
            books.total_stock(ONLY_GOOD),
            untouched.total_stock(ONLY_GOOD)
        );
        assert!(books.journal().is_empty(), "a refusal writes nothing");
    }

    #[test]
    fn a_negative_count_is_refused_by_production_and_by_consumption() {
        let mut books = Books::new(&shipped()).expect("the shipped configuration opens the books");

        assert_eq!(
            books.produce(firm(0, 0), ONLY_GOOD, -1),
            Err(PostError::NegativeUnits { units: -1 })
        );
        assert_eq!(
            books.consume(firm(0, 0), ONLY_GOOD, -1),
            Err(PostError::NegativeUnits { units: -1 })
        );
        assert!(books.journal().is_empty(), "a refusal writes nothing");
    }

    #[test]
    fn the_transaction_count_rises_for_an_exchange_and_not_for_a_production() {
        // The distinction LEDG-08 rests on: a tick in which firms only produced
        // has traded nothing.
        let mut books = Books::new(&shipped()).expect("the shipped configuration opens the books");

        books
            .produce(firm(0, 0), ONLY_GOOD, 10)
            .expect("a firm can produce");
        books
            .consume(firm(0, 0), ONLY_GOOD, 2)
            .expect("a firm holding stock can consume");
        assert_eq!(
            books.transactions_this_tick(),
            0,
            "production and consumption move no money"
        );

        books
            .exchange(
                household(0),
                firm(0, 0),
                ONLY_GOOD,
                1,
                Money::from_cents(105),
            )
            .expect("an endowed household can buy a unit");
        assert_eq!(books.transactions_this_tick(), 1);

        books
            .transfer(household(1), firm(0, 0), Money::from_cents(1))
            .expect("an endowed household can pay a cent");
        assert_eq!(books.transactions_this_tick(), 2);
    }

    #[test]
    fn the_goods_posting_kinds_serialise_under_their_own_names() {
        // Phase 3 writes this shape into its event stream; pinned here so a
        // change to it is a reviewed diff rather than a silent one.
        for (kind, expected) in [
            (PostingKind::Exchange, "exchange"),
            (PostingKind::Produce, "produce"),
            (PostingKind::Consume, "consume"),
        ] {
            let posting = Posting {
                seq: 0,
                kind,
                debit: household(12),
                credit: firm(3, 1),
                debit_cents: 250,
                credit_cents: 250,
                good: ONLY_GOOD,
                units_out: 2,
                units_in: 2,
                cash_residual_cents: 0,
                goods_residual_units: 0,
            };
            let rendered = toml::to_string(&posting).expect("a posting serialises");
            assert!(
                rendered.contains(&format!("kind = \"{expected}\"")),
                "{rendered}"
            );
            assert!(rendered.contains("units_out = 2"), "{rendered}");
            assert!(rendered.contains("units_in = 2"), "{rendered}");
        }
    }
}

/// The books' third quantity (LEDG-06), at unit granularity.
///
/// Named `headcount` so that a `books::headcount` module-path filter selects
/// exactly these. There is no non-negativity test here and that is the point:
/// the count is unsigned, so a negative payroll is not representable and a test
/// for one could not be compiled, let alone made to fail. What *is* worth
/// pinning is that the quantity is genuinely owned — that it round-trips, that
/// it aggregates, that an address outside the arena is refused rather than
/// panicking, and that it is independent of the two conserved quantities.
#[cfg(test)]
mod headcount {
    use super::*;
    use std::path::Path;

    fn shipped() -> Params {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
        let (params, _hash) = crate::config::load(&path).expect("the shipped configuration loads");
        params
    }

    fn books() -> Books {
        Books::new(&shipped()).expect("the shipped endowment sums to the stock")
    }

    #[test]
    fn every_slot_opens_with_an_empty_payroll() {
        // No employment relation exists before Phase 6, so this is the initial
        // condition of the model rather than a configured value.
        let books = books();
        let firms = shipped().sim.firms;

        assert_eq!(books.total_headcount(), 0);
        for slot in 0..firms {
            let slot = u16::try_from(slot).expect("the shipped run has at most u16::MAX slots");
            assert_eq!(books.headcount_of(FirmSlot(slot)), Some(0));
        }
    }

    #[test]
    fn setting_a_count_then_reading_it_back_round_trips() {
        let mut books = books();

        assert_eq!(
            books.set_headcount(FirmSlot(3), 17),
            Some(0),
            "the setter reports the count it replaced"
        );
        assert_eq!(books.headcount_of(FirmSlot(3)), Some(17));

        assert_eq!(books.set_headcount(FirmSlot(3), 4), Some(17));
        assert_eq!(books.headcount_of(FirmSlot(3)), Some(4));

        assert_eq!(
            books.headcount_of(FirmSlot(2)),
            Some(0),
            "writing one slot does not touch its neighbour"
        );
    }

    #[test]
    fn the_total_is_the_sum_of_the_individual_counts() {
        // The aggregate is the accessor the LEDG-06 documentation refers to, so
        // it is asserted against the counts themselves and never against a
        // number it maintained on the side.
        let mut books = books();
        let firms = u16::try_from(shipped().sim.firms).expect("at most u16::MAX slots");

        let mut expected = 0u64;
        for slot in 0..firms {
            let count = u32::from(slot) * 3 + 1;
            books.set_headcount(FirmSlot(slot), count);
            expected += u64::from(count);
        }

        assert_eq!(books.total_headcount(), expected);
        assert_eq!(
            books.total_headcount(),
            (0..firms)
                .map(|slot| u64::from(books.headcount_of(FirmSlot(slot)).expect("the slot exists")))
                .sum::<u64>(),
            "the total is the sum over the slots, read through the same accessor"
        );
    }

    #[test]
    fn a_slot_outside_the_arena_reads_nothing_and_writes_nothing() {
        // A read is a question, and the answer about a slot these books do not
        // hold is "there is none" — not a panic, and not a plausible zero.
        let mut books = books();
        let outside = FirmSlot(u16::MAX);
        let before = books.clone();

        assert_eq!(books.headcount_of(outside), None);
        assert_eq!(books.set_headcount(outside, 9), None);
        assert_eq!(books.headcount_of(outside), None);
        assert_eq!(
            books.total_headcount(),
            before.total_headcount(),
            "a refused write leaves the payrolls exactly as it found them"
        );
    }

    #[test]
    fn the_headcount_is_independent_of_the_two_conserved_quantities() {
        // A headcount has no counterparty and no conservation identity. Moving
        // cash and moving units must therefore leave it alone, and setting it
        // must leave both of them alone.
        let params = shipped();
        let mut books = Books::new(&params).expect("the books open");
        let buyer = Account::Household(HouseholdId(0));
        let seller = Account::Firm(FirmId {
            slot: FirmSlot(0),
            generation: 0,
        });

        books.set_headcount(FirmSlot(0), 11);

        books
            .transfer(buyer, seller, Money::from_cents(250))
            .expect("an endowed household can pay");
        books
            .produce(seller, ONLY_GOOD, 5)
            .expect("a firm produces");
        books
            .exchange(buyer, seller, ONLY_GOOD, 2, Money::from_cents(100))
            .expect("a household buys two units");
        books.consume(buyer, ONLY_GOOD, 2).expect("and eats them");

        assert_eq!(
            books.headcount_of(FirmSlot(0)),
            Some(11),
            "cash and goods operations do not touch a payroll"
        );
        assert_eq!(books.total_headcount(), 11);

        let money_before = books.total_money();
        let stock_before = books.total_stock(ONLY_GOOD);
        books.set_headcount(FirmSlot(0), 0);
        assert_eq!(
            books.total_money(),
            money_before,
            "setting a payroll moves no cash"
        );
        assert_eq!(
            books.total_stock(ONLY_GOOD),
            stock_before,
            "setting a payroll moves no units"
        );
        assert_eq!(books.cash_residual_cents(), 0);
        assert_eq!(books.goods_residual_units(), 0);
    }
}
