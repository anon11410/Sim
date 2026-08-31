//! The books: the one place a cent exists, and the one place a cent moves
//! (LEDG-01, LEDG-02, LEDG-03, LEDG-04, LEDG-07, LEDG-09).
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

/// What a posting records.
///
/// Plan 02-03 appends `Exchange`, `Produce` and `Consume`. New variants are
/// **appended**, never inserted or reordered: the serialised form below is the
/// wire shape Phase 3 writes into its event stream, and a renamed or reordered
/// variant is a trajectory-visible change to a committed log rather than a
/// refactor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostingKind {
    /// Opening endowment. Its counterparty is outside the books by definition,
    /// so its debit leg carries no amount. Not a transaction: see
    /// [`Books::transactions_this_tick`].
    Endow,
    /// Cash moved from one account in these books to another.
    Transfer,
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
/// **The units leg is two amounts for the same reason.** Both are zero for
/// every posting this phase produces; plan 02-03 gives them values.
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
    /// Units that left `debit`.
    pub units_out: i64,
    /// Units that arrived at `credit`.
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
}

/// Where a resolved account's balance lives.
///
/// Private, and the only way to reach a balance vector index. Constructing one
/// requires passing [`Books::resolve`], which is what bounds-checks the index
/// and compares a firm's generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CashSlot {
    Household(usize),
    Firm(usize),
}

/// Every cent in the simulation, plus this tick's journal.
///
/// All fields are private and there is exactly one constructor. No accessor
/// returns a mutable reference to a balance or to a balance vector: that would
/// hand a caller the mutation point [`Books::transfer`] exists to monopolise,
/// and no search for a setter *name* would find a getter shaped that way.
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
    /// This tick's postings. Cleared, not reallocated, by
    /// [`Books::end_of_tick`], so the capacity is reused.
    journal: Vec<Posting>,
    next_seq: u32,
    /// Cents posted so far, less the opening stock. Maintained incrementally by
    /// the recorder and never reset by [`Books::end_of_tick`] — it measures the
    /// whole run against its opening stock and is meaningful only cumulatively.
    cash_residual_cents: i64,
    /// The goods identity's running residual, maintained the same way. Zero for
    /// every posting this phase produces; plan 02-03 gives it values.
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
    ///    outside the books by definition.
    /// 4. The running residual must have returned to zero. If it has not, the
    ///    endowment does not sum to the stock and construction fails with both
    ///    numbers. This is a construction-time check and is a *different* check
    ///    from the per-tick one; both are needed.
    /// 5. The journal is cleared and the sequence and transaction counters are
    ///    reset, so **tick 0 begins with an empty journal.**
    ///
    /// Step 5 closes the subtlest trap in this phase. If the endowment postings
    /// survived into tick 0's journal, the liveness check (LEDG-08) could pass
    /// on the strength of the endowment alone — exactly the degenerate pass it
    /// exists to close. Phase 3 therefore reads opening balances from the
    /// accessors below rather than from an endowment event.
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

        let mut books = Books {
            opening_stock: Money::from_cents(opening_cents),
            household_cash: vec![Money::ZERO; households],
            firm_cash: vec![Money::ZERO; firms],
            firm_generation: vec![0; firms],
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

    /// The cash `account` holds, or `None` if it names no account in these
    /// books — including a firm identity whose generation no longer occupies
    /// its slot.
    pub fn cash_of(&self, account: Account) -> Option<Money> {
        self.resolve(account).map(|slot| self.cash_at(slot))
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

    /// This tick's postings, in the order they were recorded.
    pub fn journal(&self) -> &[Posting] {
        &self.journal
    }

    /// How many cash transactions this tick has recorded.
    ///
    /// Counts [`PostingKind::Transfer`] postings and nothing else. An endowment
    /// is not a transaction, and in plan 02-03 neither is production nor
    /// consumption. That counting rule is what makes LEDG-08 mean "money
    /// changed hands" rather than "something happened".
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
        let goods_delta = draft.units_out.saturating_sub(draft.units_in);
        self.cash_residual_cents = self.cash_residual_cents.saturating_add(cash_delta);
        self.goods_residual_units = self.goods_residual_units.saturating_add(goods_delta);

        if draft.kind == PostingKind::Transfer {
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
    fn resolve(&self, account: Account) -> Option<CashSlot> {
        match account {
            Account::Household(household) => {
                let index = household.0 as usize;
                (index < self.household_cash.len()).then_some(CashSlot::Household(index))
            }
            Account::Firm(firm) => {
                let index = firm.slot.0 as usize;
                let generation = *self.firm_generation.get(index)?;
                (generation == firm.generation).then_some(CashSlot::Firm(index))
            }
        }
    }

    /// Read the balance at an already-resolved slot.
    fn cash_at(&self, slot: CashSlot) -> Money {
        match slot {
            CashSlot::Household(index) => self.household_cash[index],
            CashSlot::Firm(index) => self.firm_cash[index],
        }
    }

    /// Write the balance at an already-resolved slot.
    ///
    /// The index came from [`Books::resolve`], which bounds-checked it, and both
    /// vectors are fixed length for the life of the books — so this is an
    /// assignment and nothing more, which is what makes the commit step of
    /// [`Books::transfer`] infallible.
    fn write_cash(&mut self, slot: CashSlot, value: Money) {
        match slot {
            CashSlot::Household(index) => self.household_cash[index] = value,
            CashSlot::Firm(index) => self.firm_cash[index] = value,
        }
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
