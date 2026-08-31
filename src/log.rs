//! The log seam: where a run's records leave the process (TICK-03, TICK-04,
//! TICK-07).
//!
//! One trait, three implementations, three record types: the per-tick series,
//! the event stream, and the decision-provenance table.
//!
//! [`Sink`] is the seam. [`NullSink`] is for in-process runs that write
//! nothing, [`VecSink`] lets a unit test assert against the records themselves
//! rather than against a re-parsed file, and [`RunWriter`] is the disk writer.
//! A caller holds `&mut dyn Sink`, so which of the three a run uses is not a
//! property of the pipeline.
//!
//! **[`Sink::finish`] is named `finish` and not `flush`, and it returns an
//! error.** It is called exactly once, and its error must be propagated: the
//! underlying comma-separated writer does flush on drop but *discards* the
//! error, so a full disk would silently truncate a run and every other check
//! would still pass. Process termination runs no destructors at all, so on the
//! halt path there is no drop to rely on either. `finish` is the only place
//! either failure is caught, and the caller must call it before it inspects
//! the run's outcome.
//!
//! **The header is written eagerly, at construction.** A run that writes zero
//! rows — which is what a run halted by the invariant phase produces, since
//! the check is pipeline position 7 and the log is position 8 — would
//! otherwise leave a zero-byte file, and a zero-byte file is not an openable
//! table on the analysis side. The header is what keeps a halted run's
//! artefact openable.
//!
//! The header is **derived from the serialisation**, not typed out: one
//! exemplar row is serialised through a throwaway header-enabled writer and its
//! first line becomes [`ticks_header`]. One source for the column names means a
//! later schema emitter cannot disagree with the file it describes, and a
//! renamed field cannot leave a stale hand-written header behind.
//!
//! **Nothing in the event stream nests, and that is a decision rather than a
//! simplification** (resolving 03-RESEARCH.md Open Question 4). A nested record
//! would be serialised through the serialisation library's own value type,
//! whose backing map is ordered by key, so its fields would appear
//! alphabetically while every top-level field appears in declaration order —
//! one file carrying two orderings, a schema that can record only one of them,
//! and a dictionary-valued column on the analysis side. So there is **no
//! posting record in the event stream at this phase.** The journal is cleared
//! each tick and a full journal dump is an opt-in flag a later phase may add;
//! when it does, it flattens.
//!
//! **A correction to this project's own documentation, measured rather than
//! assumed.** `CLAUDE.md` and `research/STACK.md` both state that the
//! serialisation library sorts map keys, giving byte-identical output. That is
//! true only of its own value type, whose backing map is ordered. A
//! **hashed-map field** on a serialised struct goes through the map
//! serialisation path and keeps the map's own iteration order — measured as
//! five different orderings in five consecutive runs of one binary. Nothing
//! here uses one, and the `clippy.toml` type ban plus `tests/lints.sh` check 4a
//! (which catches a type *alias* the lint cannot see) is what keeps it that
//! way; the reason is now measured rather than inherited.
//!
//! **A violating tick is never logged, and no violation record is written from
//! this module** (resolving 03-RESEARCH.md Open Question 3). Two reasons. The
//! series stays a series of ticks that *passed* their check, which is what
//! makes a row in it mean something. And this is the one module that
//! legitimately holds a filesystem path — rendering a halt message here would
//! put the environment next to the message, which is exactly what the
//! halt-message guard exists to keep apart. The eager header keeps the halted
//! run openable; the halt message on standard error and the run record carry
//! the evidence.

use std::fs::File;
use std::io;
use std::io::{BufWriter, Write as _};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The per-tick series, relative to the run directory.
pub const TICKS_FILE: &str = "ticks.csv";

/// The event stream, relative to the run directory. One record per line.
pub const EVENTS_FILE: &str = "events.jsonl";

/// One row of the per-tick series.
///
/// **Flat, integer-only, and with no optional field.** All three are load
/// bearing rather than stylistic.
///
/// *Flat*, because the comma-separated writer refuses a nested struct or a
/// sequence at **runtime**, not at compile time: a nested field compiles and
/// fails only when the first row is written.
///
/// *Integer-only*, and money named with a cents suffix, because a decimal
/// string makes the analysis side infer a fractional column — which would
/// degrade the conservation audit from exact integer equality to a tolerance
/// check, the one degradation the whole integer-cents decision exists to
/// prevent.
///
/// *No optional field*, because an absent value writes an empty cell, an empty
/// cell reads back as a missing value, and one missing value widens the whole
/// column. If a value can ever be absent, it gets a sentinel integer and a line
/// in the schema — never an option.
///
/// The field order **is** the column order of the file, and from plan 03-04 it
/// is frozen into a committed schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickRow {
    pub tick: u32,
    pub total_money_cents: i64,
    pub firm_cash_cents: i64,
    pub stock_units: i64,
    pub headcount: u64,
    pub transactions: u32,
    pub rng_draws: u32,
    pub activation_digest: i64,
    pub postings: u32,
}

/// The row the header is derived from. Never written to a run's file.
const HEADER_EXEMPLAR: TickRow = TickRow {
    tick: 0,
    total_money_cents: 0,
    firm_cash_cents: 0,
    stock_units: 0,
    headcount: 0,
    transactions: 0,
    rng_draws: 0,
    activation_digest: 0,
    postings: 0,
};

/// The column names of a comma-separated row type, in file order, read out of
/// the serialisation itself.
///
/// **The single derivation in this module**, so that every table's header has
/// one source. Derived rather than written out, for the reason the module docs
/// give: a hand-typed list is a second source of truth that a field rename
/// leaves stale, and the file would then disagree with the schema that claims
/// to describe it.
///
/// # Panics
///
/// If `exemplar` cannot be serialised as a flat record. That is a defect in the
/// row type — a nested or sequence-valued field — and it is caught here, on any
/// call, rather than on the first row of a long run.
fn header_of<T: Serialize>(exemplar: T) -> Vec<String> {
    let mut probe = csv::Writer::from_writer(Vec::new());
    probe
        .serialize(exemplar)
        .expect("a row type is flat and serialises as one record");
    probe.flush().expect("writing to a vector cannot fail");
    let bytes = probe.into_inner().expect("writing to a vector cannot fail");
    let text = String::from_utf8(bytes).expect("column names are valid text");
    text.lines()
        .next()
        .expect("a serialised record has a header line")
        .split(',')
        .map(str::to_owned)
        .collect()
}

/// The column names of [`TickRow`], in file order.
///
/// # Panics
///
/// See [`header_of`].
pub fn ticks_header() -> Vec<String> {
    header_of(HEADER_EXEMPLAR)
}

/// One record of the event stream (TICK-04).
///
/// **Externally tagged and snake-cased**: the tag field is emitted first, then
/// the variant's own fields in declaration order, on one line —
/// `{"event":"hire","tick":0,"firm":"firm:3:0",…}`.
///
/// **Every variant is flat, and every field is an integer or a rendered
/// address.** See the module docs for why nothing nests. Money fields carry a
/// cents suffix, and no field may ever hold a fractional value: this writer
/// maps both not-a-number and infinity to a null, so a runaway quantity and an
/// invalid one become indistinguishable in the file, irreversibly. A rate that
/// must be logged is logged as an integer on one of `src/numeric.rs`'s scales.
///
/// **An agent is named by its rendered address**, never by a bare index — the
/// form `src/ids.rs` owns and `src/books.rs` already writes into a serialised
/// posting. One spelling of an agent across the whole file is what lets a
/// Python-side join, or a `grep`, address a household in the event stream and
/// in the endowment rows with the same string.
///
/// **Variants are appended, never renamed or reordered.** Four of the five —
/// hire, fire, dividend and bankruptcy — have no call site yet and are declared
/// now so that Phases 6, 8 and 10 add one rather than reopen a wire shape that
/// plan 03-04 freezes into a committed schema and 03-06 into a committed golden
/// run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// A firm took on a household at a wage. Phase 6.
    Hire {
        tick: u32,
        firm: String,
        household: String,
        wage_cents: i64,
    },
    /// A firm let a household go. Phase 6.
    Fire {
        tick: u32,
        firm: String,
        household: String,
    },
    /// A firm paid a household its share of profit. Phase 8.
    Dividend {
        tick: u32,
        firm: String,
        household: String,
        amount_cents: i64,
    },
    /// A firm failed, and what it still owed when it did. Phase 10.
    Bankruptcy {
        tick: u32,
        firm: String,
        residual_cents: i64,
    },
    /// The opening endowment, one record per account, emitted at run setup
    /// before the first tick.
    ///
    /// **Read from the ledger's accessors, never from a posting.**
    /// `Books::new` clears the endowment postings before tick 0 precisely so
    /// the liveness check cannot be satisfied by them, and its own
    /// documentation instructs this phase to read the opening balances from
    /// the accessors instead.
    ///
    /// This is not a workaround for an otherwise-empty file. Phase 4's
    /// conservation audit is defined as a replay **from the initial
    /// endowment**; without these records there is nothing to replay from, and
    /// the sum of their cash fields is the quantity the whole audit is anchored
    /// to.
    Endowment {
        tick: u32,
        account: String,
        cash_cents: i64,
        units: i64,
    },
}

/// One [`Event::Endowment`] per account the ledger holds, in the ledger's own
/// documented walk order: households by ascending index, then firm slots by
/// ascending slot.
///
/// The order is reused rather than re-derived because `Books::accounts`
/// documents it as part of the invariant contract — so the event file's order
/// is the order every other part of the system would produce if it sorted.
///
/// `units` is the account's holding of the single good these books carry. Phase
/// 5, which adds a second good, has to revisit this field: one record per
/// account with a summed holding stops meaning anything the moment two goods
/// exist.
///
/// # Panics
///
/// If an account the ledger enumerated does not resolve in the same ledger.
/// That is a defect in the ledger's own accessors rather than a runtime
/// condition, and a silent zero here would put a fabricated opening balance
/// into the file Phase 4 replays from.
pub fn endowment_events(books: &crate::books::Books, tick: u32) -> Vec<Event> {
    books
        .accounts()
        .map(|account| {
            let cash_cents = books
                .cash_of(account)
                .expect("an account the ledger enumerated resolves in that ledger")
                .cents();
            let units: i64 = books
                .goods()
                .iter()
                .map(|good| {
                    books
                        .stock_of(account, *good)
                        .expect("a good the ledger carries resolves for an enumerated account")
                })
                .sum();
            Event::Endowment {
                tick,
                account: account.to_string(),
                cash_cents,
                units,
            }
        })
        .collect()
}

/// Where a run's records go.
///
/// The row method takes no result: an implementation that can fail records its
/// **first** failure and reports it from [`Sink::finish`]. Returning a result
/// per row would put an error path on every tick of the pipeline for a failure
/// that is, in practice, terminal for the whole run; keeping the first error
/// rather than the last is what makes the reported one the attributable one.
pub trait Sink {
    /// Record one tick's row.
    fn tick_row(&mut self, row: TickRow);

    /// Record one event.
    fn event(&mut self, event: Event);

    /// Flush everything and report the first failure, if there was one.
    ///
    /// Called exactly once, by the owner of the sink, **before** the run's
    /// outcome is inspected.
    fn finish(&mut self) -> io::Result<()>;
}

/// A sink that writes nothing. For in-process runs with no artefact.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullSink;

impl Sink for NullSink {
    fn tick_row(&mut self, _row: TickRow) {}

    fn event(&mut self, _event: Event) {}

    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A sink that keeps the rows, so a test can assert against the records rather
/// than against a re-parsed file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VecSink {
    pub rows: Vec<TickRow>,
    pub events: Vec<Event>,
}

impl Sink for VecSink {
    fn tick_row(&mut self, row: TickRow) {
        self.rows.push(row);
    }

    fn event(&mut self, event: Event) {
        self.events.push(event);
    }

    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The disk writer: one run directory, one file per stream inside it.
pub struct RunWriter {
    ticks: csv::Writer<File>,
    /// The event stream. Buffered, and flushed by [`Sink::finish`] — process
    /// termination runs no destructors, so on the halt path there is no drop to
    /// rely on.
    events: BufWriter<File>,
    /// The first failure seen, held until [`Sink::finish`] reports it.
    first_error: Option<io::Error>,
}

impl RunWriter {
    /// Create the run directory and open the per-tick series, header written.
    ///
    /// The directory path is the operator's; it is joined only with the fixed
    /// literal file name above and never with anything derived from
    /// configuration content (threat T-03-04).
    ///
    /// The writer is built with automatic headers **off** and the header
    /// written explicitly. The obvious spelling emits it twice — the
    /// comma-separated writer emits its own header on the first serialised row
    /// — and the second header then reads back as a row of text, turning every
    /// column into text with it.
    pub fn new(dir: &Path) -> io::Result<RunWriter> {
        std::fs::create_dir_all(dir)?;
        let file = File::create(dir.join(TICKS_FILE))?;
        let mut ticks = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        ticks
            .write_record(ticks_header())
            .map_err(io::Error::other)?;
        ticks.flush()?;

        let events = BufWriter::new(File::create(dir.join(EVENTS_FILE))?);

        Ok(RunWriter {
            ticks,
            events,
            first_error: None,
        })
    }
}

impl Sink for RunWriter {
    fn tick_row(&mut self, row: TickRow) {
        if self.first_error.is_some() {
            return;
        }
        if let Err(error) = self.ticks.serialize(row) {
            self.first_error = Some(io::Error::other(error));
        }
    }

    /// One serialised record per line.
    ///
    /// Written through the serialisation library rather than formatted by
    /// hand: one unescaped quotation mark inside a rendered address would
    /// silently corrupt the stream, and the library's number formatting is
    /// shortest-round-trip and deterministic.
    fn event(&mut self, event: Event) {
        if self.first_error.is_some() {
            return;
        }
        let written = serde_json::to_writer(&mut self.events, &event)
            .map_err(io::Error::other)
            .and_then(|()| self.events.write_all(b"\n"));
        if let Err(error) = written {
            self.first_error = Some(error);
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        let flushed = self.ticks.flush().and_then(|()| self.events.flush());
        match self.first_error.take() {
            Some(error) => Err(error),
            None => flushed,
        }
    }
}

/// The event stream's wire shape (TICK-04).
///
/// Named `events` so that `cargo test --lib log::events` — the command the
/// phase validation map maps the requirement to — reaches this module and not
/// an empty set.
#[cfg(test)]
mod events {
    use super::*;

    /// The five variants, each with a distinguishable value in every field so
    /// that a field swapped for its neighbour cannot round-trip unnoticed.
    fn one_of_each() -> Vec<Event> {
        vec![
            Event::Hire {
                tick: 11,
                firm: "firm:3:0".to_owned(),
                household: "household:12".to_owned(),
                wage_cents: 6_300,
            },
            Event::Fire {
                tick: 12,
                firm: "firm:4:1".to_owned(),
                household: "household:13".to_owned(),
            },
            Event::Dividend {
                tick: 13,
                firm: "firm:5:0".to_owned(),
                household: "household:14".to_owned(),
                amount_cents: 250,
            },
            Event::Bankruptcy {
                tick: 14,
                firm: "firm:6:2".to_owned(),
                residual_cents: -70,
            },
            Event::Endowment {
                tick: 0,
                account: "household:7".to_owned(),
                cash_cents: 8_000,
                units: 3,
            },
        ]
    }

    /// The tag a variant is expected to carry, spelled out here rather than
    /// derived, because a derived expectation would compare the serialisation
    /// with itself and pass however the variants were renamed.
    fn expected_tag(event: &Event) -> &'static str {
        match event {
            Event::Hire { .. } => "hire",
            Event::Fire { .. } => "fire",
            Event::Dividend { .. } => "dividend",
            Event::Bankruptcy { .. } => "bankruptcy",
            Event::Endowment { .. } => "endowment",
        }
    }

    /// Serialise, parse back, and assert both that the value survives and that
    /// the emitted line carries the tag naming the variant.
    fn round_trip(event: Event) {
        let line = serde_json::to_string(&event).expect("an event serialises");
        let parsed: Event = serde_json::from_str(&line).expect("an event parses back");
        assert_eq!(parsed, event, "the record did not survive the round trip");

        let tag = expected_tag(&event);
        assert!(
            line.starts_with(&format!("{{\"event\":\"{tag}\",")),
            "the tag is emitted first and names the variant: {line}"
        );
        assert!(!line.contains('\n'), "one record is one line: {line}");
    }

    #[test]
    fn a_hire_record_round_trips() {
        round_trip(one_of_each().remove(0));
    }

    #[test]
    fn a_fire_record_round_trips() {
        round_trip(one_of_each().remove(1));
    }

    #[test]
    fn a_dividend_record_round_trips() {
        round_trip(one_of_each().remove(2));
    }

    #[test]
    fn a_bankruptcy_record_round_trips() {
        round_trip(one_of_each().remove(3));
    }

    #[test]
    fn an_endowment_record_round_trips() {
        round_trip(one_of_each().remove(4));
    }

    #[test]
    fn the_tag_comes_first_then_the_declared_fields() {
        // Written out ONCE, here, and compared against the emitted bytes. This
        // is the byte shape plan 03-04 freezes into a committed schema; a test
        // that rebuilt the expectation from the type would agree with any
        // reordering the type underwent.
        let line = serde_json::to_string(&one_of_each().remove(0)).expect("an event serialises");
        assert_eq!(
            line,
            "{\"event\":\"hire\",\"tick\":11,\"firm\":\"firm:3:0\",\
             \"household\":\"household:12\",\"wage_cents\":6300}"
        );
    }

    #[test]
    fn an_address_renders_as_a_short_stable_string() {
        // The ledger, not this module, owns the wire shape of an address: an
        // account rendered here must be the identical string a serialised
        // posting carries. Compared against the `Display` form rather than
        // against a literal, so the two cannot drift apart.
        use crate::ids::{Account, FirmId, FirmSlot, HouseholdId};

        let household = Account::Household(HouseholdId(12));
        let firm = Account::Firm(FirmId {
            slot: FirmSlot(3),
            generation: 0,
        });
        assert_eq!(household.to_string(), "household:12");
        assert_eq!(firm.to_string(), "firm:3:0");

        let line = serde_json::to_string(&Event::Endowment {
            tick: 0,
            account: household.to_string(),
            cash_cents: 1,
            units: 0,
        })
        .expect("an event serialises");
        assert!(
            line.contains("\"account\":\"household:12\""),
            "an address is a string, not a structural encoding: {line}"
        );
    }

    #[test]
    fn nothing_in_the_stream_nests() {
        // The measured defect this guards: a nested record serialises through
        // the library's own key-ordered value type, so its fields appear
        // alphabetically while every top-level field keeps declaration order —
        // one file carrying two orderings, and a dictionary-valued column on
        // the analysis side.
        for event in one_of_each() {
            let line = serde_json::to_string(&event).expect("an event serialises");
            assert!(
                !line.contains(":{") && !line.contains(":["),
                "a variant nests a record or a sequence: {line}"
            );
        }
    }

    #[test]
    fn no_variant_can_carry_a_fractional_value() {
        // This writer maps both not-a-number and infinity to a null, so the two
        // are indistinguishable once written. Asserted as a property of the
        // emitted bytes, which is the thing the analysis side actually reads.
        for event in one_of_each() {
            let line = serde_json::to_string(&event).expect("an event serialises");
            assert!(
                !line.contains("null") && !line.contains("NaN") && !line.contains("Infinity"),
                "a fractional or absent value reached the stream: {line}"
            );
            assert!(
                !line.contains(".0") && !line.contains(".5"),
                "a decimal point reached the stream: {line}"
            );
        }
    }

    #[test]
    fn the_vector_sink_keeps_the_events_in_order() {
        let mut sink = VecSink::default();
        for event in one_of_each() {
            sink.event(event);
        }
        sink.finish().expect("the vector sink finishes");
        assert_eq!(sink.events, one_of_each());
    }
}

/// The opening endowment: the event stream's content at a phase with no
/// economics (TICK-04).
///
/// Named `endowment` so that `cargo test --lib log::endowment` reaches it.
#[cfg(test)]
mod endowment {
    use super::*;
    use crate::books::Books;
    use crate::config::Params;

    const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

    fn shipped() -> Params {
        crate::config::load(Path::new(CONFIG))
            .expect("the shipped configuration loads")
            .0
    }

    fn cash_of(event: &Event) -> i64 {
        match event {
            Event::Endowment { cash_cents, .. } => *cash_cents,
            other => panic!("not an endowment record: {other:?}"),
        }
    }

    fn account_of(event: &Event) -> &str {
        match event {
            Event::Endowment { account, .. } => account,
            other => panic!("not an endowment record: {other:?}"),
        }
    }

    #[test]
    fn one_record_per_account_in_the_ledgers_documented_walk_order() {
        let params = shipped();
        let books = Books::new(&params).expect("the shipped configuration opens books");
        let records = endowment_events(&books, 0);

        // The expected count comes from the CONFIGURATION, not from the same
        // iterator the records were built from: households plus firm slots.
        let expected = params.sim.households as usize + params.sim.firms as usize;
        assert_eq!(
            records.len(),
            expected,
            "one record per account: {} households plus {} firm slots",
            params.sim.households,
            params.sim.firms
        );

        // The walk order `Books::accounts` documents, stated independently
        // rather than re-derived from that iterator: households by ascending
        // index first, then firm slots by ascending slot.
        let last_household = params.sim.households as usize - 1;
        assert_eq!(account_of(&records[0]), "household:0");
        assert_eq!(
            account_of(&records[last_household]),
            format!("household:{last_household}")
        );
        assert_eq!(account_of(&records[last_household + 1]), "firm:0:0");
        assert_eq!(
            account_of(records.last().expect("a non-empty walk")),
            format!("firm:{}:0", params.sim.firms - 1)
        );

        for record in &records {
            let account = account_of(record);
            assert!(
                account.starts_with("household:") || account.starts_with("firm:"),
                "an account rendered as something other than an address: {account}"
            );
        }
    }

    #[test]
    fn the_cash_fields_sum_to_the_configured_money_stock() {
        // The two sides are genuinely independent: the records are built from
        // the ledger's accessors, and the expectation is read from the
        // configuration file. A comparison against `books.total_money()` would
        // compare the ledger with itself.
        let params = shipped();
        let books = Books::new(&params).expect("the shipped configuration opens books");
        let records = endowment_events(&books, 0);

        let total: i64 = records.iter().map(cash_of).sum();
        assert_eq!(
            total, params.money.total_money_cents,
            "the opening endowment does not sum to the configured money stock; \
             records built from anything other than the ledger accessors would \
             miss exactly this"
        );
    }

    #[test]
    fn the_endowment_is_read_from_the_accessors_not_from_the_journal() {
        // `Books::new` clears the endowment postings before tick 0 so the
        // liveness check cannot pass on them. This asserts that fact and then
        // asserts the records exist anyway — which is only possible if they
        // came from the accessors.
        let params = shipped();
        let books = Books::new(&params).expect("the shipped configuration opens books");

        assert!(
            books.journal().is_empty(),
            "the ledger opens with an empty journal; there is no posting to read an \
             endowment from"
        );
        assert!(!endowment_events(&books, 0).is_empty());
    }

    #[test]
    fn a_run_that_executed_no_tick_still_leaves_a_non_empty_event_file() {
        // The measured default outcome of an economics-free pipeline is a
        // zero-byte event file, and two zero-byte files hash equal — a
        // cross-process comparison over them compares the digest of the empty
        // string with itself and certifies nothing.
        let params = shipped();
        let directory = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(directory.path()).expect("the run writer opens");

        let mut books = Books::new(&params).expect("the shipped configuration opens books");
        let mut world = crate::world::World::new(&params);
        let rngs = crate::rng::Rngs::new(params.sim.seed);
        let checks = crate::invariants::CheckSet::from_params(&params);
        {
            let mut ctx = crate::phases::Ctx {
                world: &mut world,
                books: &mut books,
                rngs: &rngs,
                checks: &checks,
                sink: &mut writer,
            };
            crate::phases::run(&mut ctx, 0).expect("zero ticks pass");
        }
        writer.finish().expect("the run writer finishes");

        let text = std::fs::read_to_string(directory.path().join(EVENTS_FILE))
            .expect("the event file exists");
        assert!(
            !text.is_empty(),
            "a run that executed no tick left a zero-byte event file"
        );

        let lines: Vec<&str> = text.lines().collect();
        let expected = params.sim.households as usize + params.sim.firms as usize;
        assert_eq!(lines.len(), expected, "one endowment record per account");
        for line in &lines {
            assert!(
                line.starts_with("{\""),
                "a record without a leading brace: {line}"
            );
            assert!(
                line.contains("\"event\":\"endowment\""),
                "a record that is not an endowment: {line}"
            );
        }
        assert!(text.ends_with('\n'), "the last record is terminated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_row(tick: u32) -> TickRow {
        TickRow {
            tick,
            total_money_cents: 2_000_000,
            firm_cash_cents: 1_000_000,
            stock_units: 400,
            headcount: 0,
            transactions: 0,
            rng_draws: 218,
            activation_digest: 123_456_789,
            postings: 0,
        }
    }

    #[test]
    fn the_header_is_the_declared_column_order() {
        // The expectation is written out ONCE, here, precisely because
        // `ticks_header` derives it: if both sides derived it, the test would
        // compare the function with itself and pass however the columns were
        // renamed. This list is the contract plan 03-04 freezes.
        assert_eq!(
            ticks_header(),
            vec![
                "tick",
                "total_money_cents",
                "firm_cash_cents",
                "stock_units",
                "headcount",
                "transactions",
                "rng_draws",
                "activation_digest",
                "postings",
            ]
        );
    }

    #[test]
    fn money_columns_carry_the_cents_suffix() {
        // The claim the analysis side depends on, asserted as a property of
        // the header rather than as a repetition of the list above: a column
        // naming money without the suffix is a column someone will one day
        // write a decimal into.
        for name in ["total_money_cents", "firm_cash_cents"] {
            assert!(
                ticks_header().iter().any(|column| column == name),
                "{name} is missing from the header"
            );
        }
    }

    #[test]
    fn a_run_that_wrote_no_row_still_leaves_an_openable_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(dir.path()).expect("the writer opens");
        writer.finish().expect("the writer finishes");

        let text = std::fs::read_to_string(dir.path().join(TICKS_FILE)).expect("the file exists");
        assert_eq!(
            text,
            format!("{}\n", ticks_header().join(",")),
            "a zero-row run must leave a header, not a zero-byte file"
        );
    }

    #[test]
    fn the_header_is_written_once() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(dir.path()).expect("the writer opens");
        writer.tick_row(a_row(0));
        writer.tick_row(a_row(1));
        writer.finish().expect("the writer finishes");

        let text = std::fs::read_to_string(dir.path().join(TICKS_FILE)).expect("the file exists");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "header plus two rows: {text:?}");
        let header = ticks_header().join(",");
        assert_eq!(lines[0], header);
        assert_ne!(
            lines[1], header,
            "the header was emitted a second time, and the duplicate reads back as a row of text"
        );
        assert!(
            !text.contains('\r'),
            "the line terminator carries no carriage return"
        );
    }

    #[test]
    fn the_vector_sink_keeps_the_rows_in_order() {
        let mut sink = VecSink::default();
        sink.tick_row(a_row(0));
        sink.tick_row(a_row(1));
        sink.finish().expect("the vector sink finishes");

        assert_eq!(sink.rows, vec![a_row(0), a_row(1)]);
    }

    #[test]
    fn the_null_sink_writes_nothing_and_finishes_clean() {
        let mut sink = NullSink;
        sink.tick_row(a_row(0));
        sink.finish().expect("the null sink finishes");
    }
}
