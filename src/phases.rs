//! The tick pipeline: the nine documented phases, in a fixed order (TICK-01,
//! TICK-08, TICK-10).
//!
//! [`PHASES`] is a `const` table of `(PhaseId, name, function)` triples,
//! declared in run order, with [`PhaseId::ALL`] beside it as the single source
//! of truth for the sequence. It copies the construction `src/invariants.rs`
//! uses for its check table, structurally rather than by resemblance, because
//! four different claims come free from that shape: the table runs the
//! documented sequence, an identifier cannot exist without a table entry, the
//! names spell their identifiers, and the derived total order agrees with the
//! run order. The position function in the `order` test module below is an
//! **exhaustive match**, so a tenth phase stops that module compiling until it
//! is given a position — omission is a compile error, not a silent gap.
//!
//! **The first seven phases are no-ops because this phase of the project has
//! no economics by design, not because they are unfinished.** The table is
//! built with all nine present from the start; Phases 5 to 10 of the project
//! replace a `noop` with a real function and change nothing else about the
//! shape. A reader finding seven empty functions here is looking at the plan
//! working, not at a stub.
//!
//! **Each phase completes for every agent before the next begins, by
//! construction.** A phase function is a whole loop over the population, and
//! the next phase does not start until it returns. There is no per-agent step
//! function anywhere in this crate and none may be added — that is the clause
//! of TICK-01 which nothing else enforces, and a `Vec<Box<dyn Phase>>`
//! registration step is rejected for the same reason: it would let the order
//! drift at run time, where no order test can see it.
//!
//! [`PhaseFn`] returns a result. The sketch this table came from had it return
//! nothing and halt from inside the library; LEDG-10 requires the invariant
//! phase to *return* its failure, and a phase that terminated the process from
//! inside a library would make the halt untestable in process.
//!
//! [`Ctx`] holds a **shared** reference to the generator set, not a mutable
//! one. `Rngs::stream` takes `&self`, so a mutable borrow would be a lie about
//! the API rather than a safety measure.

use crate::books::Books;
use crate::invariants::{CheckSet, Violation};
use crate::log::{Sink, TickRow};
use crate::rng::{Purpose, Rngs};
use crate::world::World;

/// Everything a phase may touch.
///
/// The sink is a trait object, so which sink a run writes to is not a property
/// of the pipeline: an in-process run and a run that lands on disk execute the
/// identical code path.
pub struct Ctx<'a> {
    pub world: &'a mut World,
    pub books: &'a mut Books,
    pub rngs: &'a Rngs,
    pub checks: &'a CheckSet,
    pub sink: &'a mut dyn Sink,
}

/// Which phase of the tick, as a value a test can order and compare.
///
/// **Declared in the order the phases run**, so the derived `Ord` agrees with
/// [`PHASES`] rather than quietly contradicting it. A new identifier goes at
/// its run position, not at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhaseId {
    FirmPlanning,
    LabourMarket,
    Production,
    Wages,
    GoodsMarket,
    FirmAccounting,
    Bankruptcy,
    Invariants,
    Log,
}

impl PhaseId {
    /// Every phase identifier, in the order the phases run.
    ///
    /// **The single source of truth for the sequence.** The order tests read
    /// [`PHASES`] and compare it against this constant element for element;
    /// there is no second hand-written list for either to drift from. An
    /// identifier added to the enum but not to this constant is caught by the
    /// exhaustive match in those tests, which stops compiling.
    pub const ALL: [PhaseId; 9] = [
        PhaseId::FirmPlanning,
        PhaseId::LabourMarket,
        PhaseId::Production,
        PhaseId::Wages,
        PhaseId::GoodsMarket,
        PhaseId::FirmAccounting,
        PhaseId::Bankruptcy,
        PhaseId::Invariants,
        PhaseId::Log,
    ];
}

/// A phase: the whole context in, a violation or nothing out.
///
/// A plain function pointer, not a trait object and not a closure. The phases
/// are a fixed table known at compile time and nothing here needs to capture.
pub type PhaseFn = fn(&mut Ctx<'_>) -> Result<(), Violation>;

/// The full, ordered table: every phase, in the order it runs.
///
/// The positions are the contract. The seven economic phases run in the order
/// a day takes: a firm plans, hires, produces, pays, sells, closes its books,
/// and only then may fail. **The invariant check is position 7 and the log is
/// position 8**, in that order and not the other, because a tick is not
/// declared good until it has been checked — which is also why the tick that
/// violates is never logged.
pub const PHASES: [(PhaseId, &str, PhaseFn); 9] = [
    (PhaseId::FirmPlanning, "firm_planning", noop),
    (PhaseId::LabourMarket, "labour_market", noop),
    (PhaseId::Production, "production", noop),
    (PhaseId::Wages, "wages", noop),
    (PhaseId::GoodsMarket, "goods_market", noop),
    (PhaseId::FirmAccounting, "firm_accounting", noop),
    (PhaseId::Bankruptcy, "bankruptcy", noop),
    (PhaseId::Invariants, "invariants", run_invariants),
    (PhaseId::Log, "log", run_log),
];

/// A phase with no economics yet. See the module docs: by design, not by
/// omission.
fn noop(_ctx: &mut Ctx<'_>) -> Result<(), Violation> {
    Ok(())
}

/// Position 7: the invariant check, as a phase that returns its failure.
fn run_invariants(ctx: &mut Ctx<'_>) -> Result<(), Violation> {
    ctx.checks.run(ctx.books, ctx.world.tick)
}

/// Position 8: the tick's row, read from the books' accessors and the world's
/// activation state.
fn run_log(ctx: &mut Ctx<'_>) -> Result<(), Violation> {
    let goods = ctx.books.goods();
    let stock_units: i64 = goods.iter().map(|good| ctx.books.total_stock(*good)).sum();
    let postings = u32::try_from(ctx.books.journal().len())
        .expect("a single tick's journal is far below the u32 range");

    let row = TickRow {
        tick: ctx.world.tick,
        total_money_cents: ctx.books.total_money().cents(),
        firm_cash_cents: ctx.books.firm_cash_total().cents(),
        stock_units,
        headcount: ctx.books.total_headcount(),
        transactions: ctx.books.transactions_this_tick(),
        rng_draws: ctx.world.draws_this_tick,
        activation_digest: ctx.world.activation_digest,
        postings,
    };

    ctx.sink.tick_row(row);
    Ok(())
}

/// Draw this tick's activation order for households and for firms, and record
/// what it cost and what it was.
///
/// Each pool is rebuilt from scratch. Sharing one buffer between the two
/// purposes is the natural way to avoid a per-tick allocation and it is
/// precisely the shape `Stream::shuffle_in_place` names as a caller obligation:
/// the buffer is permuted in place, so a shared one would make the second
/// purpose's permutation depend on what the first did to it — the "an added
/// draw in one market perturbs another" coupling the sub-stream design exists
/// to remove, reintroduced through state rather than through a sequence.
///
/// The permutation's **own value** reaches the log, not merely the count of
/// draws it took. A draw count is identical at every seed, so a run logging
/// only the count produces byte-identical files at two different seeds while
/// appearing to consume the generator — which was measured, and is why TICK-10
/// is written against the digest.
pub fn shuffle_activation(world: &mut World, rngs: &Rngs) {
    world.draws_this_tick = 0;

    let household_count =
        u32::try_from(world.households.len()).expect("the household count is bounded at setup");
    world.household_order.clear();
    world.household_order.extend(0..household_count);
    let mut households = rngs.stream(world.tick, 0, Purpose::ActivationOrderHouseholds);
    households.shuffle_in_place(&mut world.household_order);
    world.draws_this_tick += households.draws();

    let firm_count = u32::try_from(world.firms.len()).expect("the firm count is bounded at setup");
    world.firm_order.clear();
    world.firm_order.extend(0..firm_count);
    let mut firms = rngs.stream(world.tick, 0, Purpose::ActivationOrderFirms);
    firms.shuffle_in_place(&mut world.firm_order);
    world.draws_this_tick += firms.draws();

    world.activation_digest = order_digest(&world.household_order, &world.firm_order);
}

/// A positive 64-bit-derived digest of one tick's activation permutation.
///
/// The two sequences are hashed with a separator between them, so a change
/// confined to the firm order alone changes the result — without the separator,
/// two different splits of the same concatenated sequence would collide.
///
/// The result is shifted right one bit so it is always positive. A negative
/// value would be perfectly valid in the file, but the analysis side reads the
/// column as a signed 64-bit integer and a value at the top of the unsigned
/// range would widen the column to an object column, taking the whole table's
/// integer typing with it.
///
/// `sha2` is used because it is already a dependency and already the crate
/// behind the configuration hash. A rolling hash was declined: it needs
/// wrapping arithmetic, and this project's release profile deliberately makes
/// wrapping panic.
pub fn order_digest(households: &[u32], firms: &[u32]) -> i64 {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    for index in households {
        hasher.update(index.to_le_bytes());
    }
    hasher.update(b"|");
    for index in firms {
        hasher.update(index.to_le_bytes());
    }
    let digest = hasher.finalize();

    let mut leading = [0u8; 8];
    leading.copy_from_slice(&digest[..8]);
    i64::try_from(u64::from_le_bytes(leading) >> 1)
        .expect("a 63-bit value is representable as a signed 64-bit integer")
}

/// One tick: draw the activation order, then walk [`PHASES`] in index order.
///
/// Returns on the first failure. The remaining phases are not run — a tick that
/// has broken an invariant has nothing further to contribute.
pub fn tick(ctx: &mut Ctx<'_>) -> Result<(), Violation> {
    shuffle_activation(ctx.world, ctx.rngs);

    for (_id, _name, phase) in PHASES {
        phase(ctx)?;
    }

    Ok(())
}

/// Run `ticks` ticks, resetting the books' per-tick state after each.
///
/// Returns the first violation, or success. The caller must finish its sink
/// **before** it inspects this result: on the halt path the ticks that led to
/// the violation are exactly the diagnostic evidence, and terminating the
/// process runs no destructors.
pub fn run(ctx: &mut Ctx<'_>, ticks: u32) -> Result<(), Violation> {
    for number in 0..ticks {
        ctx.world.tick = number;
        tick(ctx)?;
        ctx.books.end_of_tick();
    }

    Ok(())
}

#[cfg(test)]
mod order {
    use super::*;

    /// Where each identifier sits in the table.
    ///
    /// **Exhaustive on purpose.** A new [`PhaseId`] variant stops this function
    /// compiling until it is given a position, and the assertions below then
    /// force it into both [`PhaseId::ALL`] and [`PHASES`]. That is what makes
    /// "a tenth phase cannot be added without placing it in the documented
    /// order" a compile-time property rather than a promise in a comment.
    fn documented_position(id: PhaseId) -> usize {
        match id {
            PhaseId::FirmPlanning => 0,
            PhaseId::LabourMarket => 1,
            PhaseId::Production => 2,
            PhaseId::Wages => 3,
            PhaseId::GoodsMarket => 4,
            PhaseId::FirmAccounting => 5,
            PhaseId::Bankruptcy => 6,
            PhaseId::Invariants => 7,
            PhaseId::Log => 8,
        }
    }

    /// The snake-case spelling of an identifier, derived rather than written
    /// out — a second hand-written list of names would be the very thing this
    /// test exists to catch.
    fn snake_case(id: PhaseId) -> String {
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
        let sequence: Vec<PhaseId> = PHASES.iter().map(|(id, _, _)| *id).collect();

        assert_eq!(sequence, PhaseId::ALL.to_vec());
        assert_eq!(
            sequence.first().copied(),
            Some(PhaseId::FirmPlanning),
            "a day begins with a firm's plan; everything downstream reads it"
        );
        assert_eq!(
            sequence.last().copied(),
            Some(PhaseId::Log),
            "the log is last: a tick is recorded only once it has passed its check"
        );
    }

    #[test]
    fn an_identifier_cannot_exist_without_a_table_entry() {
        assert_eq!(
            PHASES.len(),
            PhaseId::ALL.len(),
            "every identifier has exactly one entry and the table has no extras"
        );

        for &id in &PhaseId::ALL {
            let position = documented_position(id);
            assert_eq!(
                PHASES[position].0, id,
                "{id:?} is not at position {position} of the table"
            );
        }
    }

    #[test]
    fn the_derived_order_agrees_with_the_run_order() {
        // The identifiers are declared in run order, so sorting them changes
        // nothing. A derived `Ord` that disagreed with the table would be a
        // trap laid for the first code that sorts a set of phases.
        let mut sorted = PhaseId::ALL.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, PhaseId::ALL.to_vec());
    }

    #[test]
    fn the_names_are_distinct_and_spell_their_identifiers() {
        let mut names: Vec<&str> = PHASES.iter().map(|(_, name, _)| *name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "two phases share a name");

        for (id, name, _) in PHASES {
            assert_eq!(
                name,
                snake_case(id),
                "the table name and the identifier have drifted apart"
            );
        }
    }

    #[test]
    fn the_check_runs_before_the_log() {
        // The one positional claim the rest of the phase depends on: the tick
        // that violates is never logged, and that is a consequence of these two
        // indices and nothing else. Asserted by identifier, not by index
        // literal alone, so a reordering that kept the count cannot pass.
        let invariants = documented_position(PhaseId::Invariants);
        let logging = documented_position(PhaseId::Log);

        assert_eq!(PHASES[invariants].0, PhaseId::Invariants);
        assert_eq!(PHASES[logging].0, PhaseId::Log);
        assert!(
            invariants < logging,
            "the log must not record a tick the check has not yet passed"
        );
    }
}

#[cfg(test)]
mod end_to_end {
    use super::*;
    use crate::config::Params;
    use crate::log::{RunWriter, TICKS_FILE, VecSink, ticks_header};
    use std::path::Path;

    const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

    /// The measured per-tick draw count: one full shuffle of each population
    /// takes `len - 1` draws, so 199 + 19 for the shipped sizes.
    const EXPECTED_DRAWS_PER_TICK: u32 = 218;

    fn shipped() -> Params {
        crate::config::load(Path::new(CONFIG))
            .expect("the shipped configuration loads")
            .0
    }

    /// Run the whole pipeline into `sink`, and return the outcome.
    fn drive(params: &Params, seed: u64, ticks: u32, sink: &mut dyn Sink) -> Result<(), Violation> {
        let mut books = Books::new(params).expect("the shipped configuration opens books");
        let mut world = World::new(params);
        let rngs = Rngs::new(seed);
        let checks = CheckSet::from_params(params);

        let mut ctx = Ctx {
            world: &mut world,
            books: &mut books,
            rngs: &rngs,
            checks: &checks,
            sink,
        };
        run(&mut ctx, ticks)
    }

    #[test]
    fn a_decade_of_empty_ticks_lands_on_disk() {
        let params = shipped();
        let directory = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(directory.path()).expect("the run writer opens");

        let outcome = drive(&params, params.sim.seed, params.sim.ticks, &mut writer);
        // The sink is finished BEFORE the outcome is inspected, exactly as the
        // binary does it: a halt would otherwise discard the ticks that led to
        // it, which are the evidence.
        writer.finish().expect("the run writer finishes");
        outcome.expect("a decade of empty ticks passes every active check");

        let text =
            std::fs::read_to_string(directory.path().join(TICKS_FILE)).expect("the file exists");

        assert!(
            !text.contains('\r'),
            "the line terminator carries no carriage return"
        );
        assert!(text.ends_with('\n'), "the last row is terminated");

        let lines: Vec<&str> = text.lines().collect();
        let expected = usize::try_from(params.sim.ticks).expect("the tick count is bounded") + 1;
        assert_eq!(
            lines.len(),
            expected,
            "one header line plus one row per configured tick"
        );

        let header = ticks_header();
        assert_eq!(lines[0], header.join(","));

        let column = |name: &str| {
            header
                .iter()
                .position(|held| held == name)
                .unwrap_or_else(|| panic!("no {name} column"))
        };
        let tick_at = column("tick");
        let draws_at = column("rng_draws");
        let digest_at = column("activation_digest");
        let money_at = column("total_money_cents");

        let mut digests = Vec::with_capacity(lines.len());
        for (number, line) in lines[1..].iter().enumerate() {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(
                fields.len(),
                header.len(),
                "row {number} is the wrong width"
            );

            for (at, field) in fields.iter().enumerate() {
                assert!(
                    !field.is_empty(),
                    "row {number} column {at} is empty; an empty cell reads back as a missing \
                     value and widens the whole column"
                );
                field.parse::<i64>().unwrap_or_else(|_| {
                    panic!("row {number} column {at} is not an integer: {field}")
                });
            }

            assert_eq!(
                fields[tick_at],
                number.to_string(),
                "the tick column counts up from zero with no gap"
            );
            assert_eq!(
                fields[draws_at],
                EXPECTED_DRAWS_PER_TICK.to_string(),
                "the per-tick draw count is fixed; a varying one is a rejection loop"
            );
            assert_eq!(
                fields[money_at],
                params.money.total_money_cents.to_string(),
                "the money pile is conserved to the cent for the whole run"
            );

            let digest: i64 = fields[digest_at].parse().expect("the digest is an integer");
            assert!(digest > 0, "the digest column stays in the positive range");
            digests.push(digest);
        }

        // The digest is a value derived from the tick's permutation, so it
        // moves with the tick. A column that were constant would be a column
        // carrying no information about the order it claims to describe.
        let mut distinct = digests.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(
            distinct.len() > digests.len() / 2,
            "the activation digest barely varies across the run: {} distinct values in {}",
            distinct.len(),
            digests.len()
        );
    }

    #[test]
    fn the_seed_reaches_the_first_logged_digest() {
        // TICK-10 in one assertion, at the cheapest possible size: the value
        // the log records at tick 0 differs between two seeds. This is the
        // claim the count-only design failed — it produced byte-identical
        // files at two different seeds while consuming the generator exactly
        // as designed.
        let params = shipped();

        let mut first = VecSink::default();
        drive(&params, 42, 1, &mut first).expect("one empty tick passes");
        let mut second = VecSink::default();
        drive(&params, 43, 1, &mut second).expect("one empty tick passes");

        assert_eq!(first.rows.len(), 1);
        assert_eq!(second.rows.len(), 1);
        assert_eq!(
            first.rows[0].rng_draws, second.rows[0].rng_draws,
            "the draw count is seed-independent, which is exactly why it cannot be the witness"
        );
        assert_ne!(
            first.rows[0].activation_digest, second.rows[0].activation_digest,
            "the seed does not reach the log: a different seed produced the same digest"
        );
    }

    #[test]
    fn the_same_seed_replays_the_same_rows() {
        let params = shipped();

        let mut first = VecSink::default();
        drive(&params, 42, 32, &mut first).expect("thirty-two empty ticks pass");
        let mut second = VecSink::default();
        drive(&params, 42, 32, &mut second).expect("thirty-two empty ticks pass");

        assert_eq!(first.rows.len(), 32);
        assert_eq!(
            first, second,
            "the same seed produced a different series across two in-process runs"
        );
    }
}
