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
//! **Both comma-separated tables write their header eagerly, at
//! construction**, and the reason differs slightly between them. A run halted
//! by the invariant phase writes zero tick rows, since the check is pipeline
//! position 7 and the log is position 8; and *every* run at this phase writes
//! zero provenance rows, because nothing decides anything yet. Either way a
//! lazily-written header leaves a **zero-byte** file — the writer emits its own
//! header only on the first serialised row — and a zero-byte comma-separated
//! file is not an openable table on the analysis side, which raises rather than
//! returning an empty frame.
//!
//! The obvious spelling emits the header **twice**: the writer emits its own on
//! the first serialised row in addition to the one written explicitly, and the
//! duplicate then reads back as a row of text, widening every column with it.
//! Both writers are therefore built with automatic headers off.
//!
//! The header is **derived from the serialisation**, not typed out: one
//! exemplar row is serialised through a throwaway header-enabled writer and its
//! first line becomes the column list. [`header_of`] is the single mechanism
//! and [`ticks_header`] and [`provenance_header`] both go through it, so there
//! is exactly one source of column names in this module — a later schema
//! emitter cannot disagree with the file it describes, and a renamed field
//! cannot leave a stale hand-written header behind.
//!
//! **One consequence handed forward to Phase 4.** A header-only table reads
//! back with every column typed as an object rather than as an integer, so the
//! harness's dtype assertion must be conditional on a non-empty frame, or must
//! read the dtype from the generated schema — which is one of the reasons that
//! schema carries dtypes at all.
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

/// The decision-provenance table, relative to the run directory.
pub const PROVENANCE_FILE: &str = "provenance.csv";

/// The run's own record, relative to the run directory.
///
/// **The single quarantined file.** It is excluded from the determinism diff
/// and is the only file in a run directory that may carry a wall clock. That
/// exclusion is a permission for a start time, not a licence to put the
/// environment beside the logs.
pub const RUN_META_FILE: &str = "run_meta.json";

/// The wire-format label carried by the run record and, from plan 03-04, by the
/// generated schema.
///
/// **Spelled without a decimal point, deliberately.** The float-confinement
/// guard in `tests/numeric_det.rs` reads whole lines and is blind to both
/// comments and string literals — Phase 1 recorded that *"a heuristic that
/// skips comments is one someone later widens to skip a string"* and reworded
/// `src/rng.rs` rather than loosening the test. A dotted version string here
/// fails that guard, which was reproduced at a cost of one build; the precedent
/// is to reword the constant, not to add this module to the allowlist. If a
/// dotted version is ever wanted in the emitted record, build it from integer
/// constants and concatenate.
///
/// **A `const`, not a configuration key** — see `config/PROVENANCE.md` § 4,
/// which carries the `GRADE: PROJECT` row CORE-10's carve-out is conditional
/// on. Being code rather than configuration does not make it free to change:
/// bumping it invalidates the committed schema and the committed golden run,
/// which is exactly why the change should be a deliberate source-level act.
pub const SCHEMA_VERSION: &str = "v1";

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

/// Which decision a provenance row describes (TICK-07).
///
/// **A fixed enumeration, not a string.** This is what makes TICK-07's "never
/// free text" a property of the type rather than of a reviewer's discipline: a
/// caller cannot write a decision name that is not one of these, and the
/// analysis side gets a closed vocabulary it can count over. Variants are
/// appended as the phases that make the decisions arrive; the serialised form
/// is the snake-cased variant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// A firm set its posted price. Phase 9.
    Price,
    /// A firm set its wage offer. Phase 6.
    Wage,
    /// A firm changed its labour demand. Phase 6.
    Hire,
}

impl Decision {
    /// Every decision, for a test that must cover the whole vocabulary.
    pub const ALL: [Decision; 3] = [Decision::Price, Decision::Wage, Decision::Hire];
}

/// Which branch of a decision rule actually fired (TICK-07).
///
/// **The highest-value column in the table, and it costs nothing.** When prices
/// spiral in Phase 9, a frequency count over this column localises the defect to
/// one branch in one query — which is not recoverable from the inputs and the
/// outcome alone, because two branches can produce the same number.
///
/// A fixed enumeration for the same reason [`Decision`] is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// The upward branch fired.
    Raised,
    /// The downward branch fired.
    Lowered,
    /// Neither branch fired and the previous value stood.
    Held,
    /// A bound bound: the rule's own arithmetic was overridden by a floor or a
    /// ceiling, so the outcome describes the bound rather than the branch.
    Bounded,
}

impl Rule {
    /// Every branch, for a test that must cover the whole vocabulary.
    pub const ALL: [Rule; 4] = [Rule::Raised, Rule::Lowered, Rule::Held, Rule::Bounded];
}

/// One row of the decision-provenance table (TICK-07): what an agent decided,
/// what it decided from, and which branch of the rule produced it.
///
/// **Seven flat columns, none of them optional and none of them free text.**
/// The two enumerated columns are enumerations of the type system; every other
/// column is an integer.
///
/// *No optional column*, on exactly the terms [`TickRow`] sets out: an absent
/// value writes an empty cell, an empty cell reads back as a missing value, and
/// one missing value widens an otherwise-integer column to a fractional one —
/// the degradation the whole integer-cents decision exists to prevent. A value
/// that can genuinely be absent later gets a documented sentinel integer.
///
/// The agent is a **rendered address**, the same string the event stream and a
/// serialised posting carry, so the three files join on one spelling.
///
/// **This phase writes zero rows**, by definition: nothing decides anything
/// yet. The table is present and schema-complete anyway, because provenance
/// added retroactively never covers the early history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRow {
    pub tick: u32,
    pub agent: String,
    pub decision: Decision,
    pub input_a: i64,
    pub input_b: i64,
    pub outcome: i64,
    pub rule: Rule,
}

/// The row the provenance header is derived from. Never written to a run's
/// file.
fn provenance_exemplar() -> ProvenanceRow {
    ProvenanceRow {
        tick: 0,
        agent: String::new(),
        decision: Decision::Price,
        input_a: 0,
        input_b: 0,
        outcome: 0,
        rule: Rule::Held,
    }
}

/// The column names of [`ProvenanceRow`], in file order.
///
/// Derived through [`header_of`] — the same single mechanism [`ticks_header`]
/// uses. Two independent header sources is the defect this shares a function to
/// prevent: a renamed field would otherwise leave one of them stale.
///
/// # Panics
///
/// See [`header_of`].
pub fn provenance_header() -> Vec<String> {
    header_of(provenance_exemplar())
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

    /// Record one decision's provenance.
    fn provenance(&mut self, row: ProvenanceRow);

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

    fn provenance(&mut self, _row: ProvenanceRow) {}

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
    pub provenance: Vec<ProvenanceRow>,
}

impl Sink for VecSink {
    fn tick_row(&mut self, row: TickRow) {
        self.rows.push(row);
    }

    fn event(&mut self, event: Event) {
        self.events.push(event);
    }

    fn provenance(&mut self, row: ProvenanceRow) {
        self.provenance.push(row);
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
    /// The decision-provenance table. Its header is written at construction —
    /// this phase writes zero rows, and a lazily-written header would leave a
    /// zero-byte file the analysis side refuses to open.
    provenance: csv::Writer<File>,
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

        let mut provenance = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(File::create(dir.join(PROVENANCE_FILE))?);
        provenance
            .write_record(provenance_header())
            .map_err(io::Error::other)?;
        provenance.flush()?;

        Ok(RunWriter {
            ticks,
            events,
            provenance,
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

    fn provenance(&mut self, row: ProvenanceRow) {
        if self.first_error.is_some() {
            return;
        }
        if let Err(error) = self.provenance.serialize(row) {
            self.first_error = Some(io::Error::other(error));
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        let flushed = self
            .ticks
            .flush()
            .and_then(|()| self.events.flush())
            .and_then(|()| self.provenance.flush());
        match self.first_error.take() {
            Some(error) => Err(error),
            None => flushed,
        }
    }
}

// ---------------------------------------------------------------------------
// The generated schema (TICK-02).
// ---------------------------------------------------------------------------

/// The generated schema, relative to the repository root.
///
/// Generated, committed, and compared against the generator by a test that
/// **never writes** — the same shape as this repository's other
/// generated-and-committed artifact, `clippy.toml`. Regeneration is a
/// deliberate operator act with a reviewable diff, not something a test
/// performs; a test that regenerated and then compared would be comparing the
/// generator with itself and would pass forever.
pub const SCHEMA_FILE: &str = "schema/schema.json";

/// The command that regenerates [`SCHEMA_FILE`].
///
/// Named here once so that the drift test's failure message points at something
/// a reader can actually run, rather than at a description of one.
pub const SCHEMA_REGEN_COMMAND: &str =
    "cargo run --locked --quiet -- --dump-schema > schema/schema.json";

/// What [`first_difference`] reports for a text that has no such line.
const NO_SUCH_LINE: &str = "<no such line>";

/// The short type name of a parsed value, in the vocabulary the analysis side
/// names its column types with.
///
/// An integral number is a 64-bit integer and any other number is the
/// fractional name; a shape this classifier does not understand is reported as
/// an explicit unsupported marker rather than as a guess. **The marker is the
/// point.** A silent fallback would let a shape the Python side cannot read
/// pass as though it were understood, and its absence from the committed
/// artifact is asserted rather than assumed.
fn value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(number) if number.is_i64() || number.is_u64() => "int64",
        serde_json::Value::Number(_) => "float64",
        serde_json::Value::Null => "null",
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => "UNSUPPORTED",
    }
}

/// The keys of a serialised record, in the order **the emitted text** carries
/// them.
///
/// Read from the text rather than from the parsed value on purpose: the parsed
/// value's backing map is ordered by key, so reading the order from it would
/// report every record alphabetically — and declaration order is the contract
/// the analysis side reads. Depth is tracked so that only top-level keys are
/// collected; nothing in the stream nests today, and a record that started to
/// would surface as an unsupported type rather than as a silently flattened
/// one.
///
/// A key carrying an escape is carried through raw, so it then fails to resolve
/// in the parsed record and is reported as unsupported. Nothing here is
/// permitted to guess.
fn keys_in_text_order(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut depth = 0usize;
    let mut expect_key = false;
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        match character {
            '"' => {
                let mut name = String::new();
                while let Some(inner) = chars.next() {
                    if inner == '\\' {
                        name.push(inner);
                        if let Some(escaped) = chars.next() {
                            name.push(escaped);
                        }
                        continue;
                    }
                    if inner == '"' {
                        break;
                    }
                    name.push(inner);
                }
                if expect_key && depth == 1 {
                    keys.push(name);
                }
                expect_key = false;
            }
            '{' => {
                depth += 1;
                expect_key = depth == 1;
            }
            '[' => {
                depth += 1;
                expect_key = false;
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                expect_key = false;
            }
            ',' => expect_key = depth == 1,
            ':' => expect_key = false,
            _ => {}
        }
    }
    keys
}

/// The ordered field-name-and-type pairs of a serialisable value, read out of
/// the bytes the writer actually produces.
///
/// **This is the whole trick, and it is why no schema-derive crate is in the
/// manifest.** The names come from the emitted text and the types from parsing
/// that same text, so there is no second description of the types anywhere and
/// the schema cannot disagree with the file. A field rendered by a custom
/// serialiser is reported as whatever it actually became — an address that
/// renders to `household:12` is typed as a string, which is what it is. A
/// derive macro is a second, independent description that cannot see
/// `#[serde(serialize_with = …)]`, and this project uses one on both address
/// fields of a serialised posting: it declares them structured objects where
/// the writer emits short strings. A generated file and a generator wrong in
/// the same way agree with each other forever, and the drift test never fires.
///
/// # Panics
///
/// If the value does not serialise as a flat record, or if the emitted text and
/// the record parsed back from it disagree on how many fields there are. Both
/// are defects in the record type rather than runtime conditions.
fn json_fields<T: Serialize>(value: &T) -> Vec<(String, &'static str)> {
    let text = serde_json::to_string(value).expect("a log record serialises");
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("what the writer emitted parses back");
    let object = parsed
        .as_object()
        .expect("a log record serialises as an object");

    let names = keys_in_text_order(&text);
    assert_eq!(
        names.len(),
        object.len(),
        "the emitted text and the record parsed back from it disagree on the field count: {text}"
    );
    names
        .into_iter()
        .map(|name| {
            let kind = object.get(&name).map_or("UNSUPPORTED", value_kind);
            (name, kind)
        })
        .collect()
}

/// The ordered column-name-and-type pairs of a comma-separated row type.
///
/// The names come from the header [`header_of`] derives — the header the
/// comma-separated writer itself emits — and the types from the same
/// serialisation. Column order **is** the contract for the tick file; a
/// generator that sorted its output alphabetically would not record it at all.
///
/// # Panics
///
/// If the two derivations disagree on the column list. They read one serde
/// implementation through two writers, so a disagreement is a defect in this
/// module rather than in the row type.
fn csv_columns<T: Serialize>(exemplar: &T) -> Vec<(String, &'static str)> {
    let names = header_of(exemplar);
    let fields = json_fields(exemplar);
    assert_eq!(
        names.len(),
        fields.len(),
        "the emitted header and the serialised record disagree on the column count"
    );
    names
        .into_iter()
        .zip(fields)
        .map(|(column, (field, kind))| {
            assert_eq!(
                column, field,
                "the emitted header and the serialised record disagree on column order"
            );
            (column, kind)
        })
        .collect()
}

/// One exemplar per event variant, in declaration order. Never written to a
/// run's file.
///
/// The values are arbitrary but distinguishable; only the **shape** of each
/// record reaches the schema.
fn event_exemplars() -> Vec<Event> {
    vec![
        Event::Hire {
            tick: 0,
            firm: "firm:0:0".to_owned(),
            household: "household:0".to_owned(),
            wage_cents: 0,
        },
        Event::Fire {
            tick: 0,
            firm: "firm:0:0".to_owned(),
            household: "household:0".to_owned(),
        },
        Event::Dividend {
            tick: 0,
            firm: "firm:0:0".to_owned(),
            household: "household:0".to_owned(),
            amount_cents: 0,
        },
        Event::Bankruptcy {
            tick: 0,
            firm: "firm:0:0".to_owned(),
            residual_cents: 0,
        },
        Event::Endowment {
            tick: 0,
            account: "household:0".to_owned(),
            cash_cents: 0,
            units: 0,
        },
    ]
}

/// The tag a variant carries, read out of the emitted record rather than
/// matched on the variant.
///
/// # Panics
///
/// If a record carries no tag. The enumeration is externally tagged, so that is
/// a change to the wire shape rather than a runtime condition.
fn event_tag(event: &Event) -> String {
    let text = serde_json::to_string(event).expect("an event serialises");
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("what the writer emitted parses back");
    parsed
        .get("event")
        .and_then(serde_json::Value::as_str)
        .expect("every event record carries its tag")
        .to_owned()
}

/// Append `value` as a quoted name.
///
/// # Panics
///
/// If the name would need escaping. Every name here is a Rust identifier or a
/// fixed file name; one that needed escaping would be a wire-shape change that
/// should be seen rather than silently encoded.
fn push_quoted(out: &mut String, value: &str) {
    assert!(
        !value.contains('"') && !value.contains('\\'),
        "a schema name that would need escaping: {value:?}"
    );
    out.push('"');
    out.push_str(value);
    out.push('"');
}

/// One field, on **one line**, in the pretty-printed spelling with a single
/// space after each colon.
///
/// One type name per line is load bearing twice over. The reviewer's diff of a
/// schema change reads field by field, which is the same reason the run record
/// is pretty-printed; and the build's negative check over a fractional type
/// name is a bare substring `grep` precisely because this text is hand-composed
/// — a pattern that also pinned the key name and the spacing would pass
/// vacuously the moment either drifted.
fn push_field(out: &mut String, indent: &str, name: &str, kind: &str, more: bool) {
    out.push_str(indent);
    out.push_str("{ \"name\": ");
    push_quoted(out, name);
    out.push_str(", \"dtype\": ");
    push_quoted(out, kind);
    out.push_str(" }");
    if more {
        out.push(',');
    }
    out.push('\n');
}

/// Append one comma-separated table's ordered column list.
fn push_table(out: &mut String, file: &str, columns: &[(String, &'static str)]) {
    out.push_str("  ");
    push_quoted(out, file);
    out.push_str(": [\n");
    for (at, (name, kind)) in columns.iter().enumerate() {
        push_field(out, "    ", name, kind, at + 1 < columns.len());
    }
    out.push_str("  ],\n");
}

/// The wire format this binary writes, read out of the writers themselves
/// (TICK-02).
///
/// The contract between this binary and the analysis harness across the disk
/// boundary: the two sides share no code, so this file is the only thing that
/// crosses it. It records the tick series' columns in order and every one as an
/// integer (TICK-03), the provenance table's seven columns — which is what lets
/// a downstream reader assert a type on a table that is legitimately empty
/// (TICK-07) — and every event variant's fields in declaration order with the
/// tag first.
///
/// **The text is composed here, in a fixed order, rather than through a map
/// type**, so the ordering is a property of this function and not of a
/// container someone could later swap.
///
/// Deterministic: two calls in one process return byte-identical text.
///
/// # Panics
///
/// See [`json_fields`], [`csv_columns`] and [`push_quoted`]. Every panic here
/// is a wire-shape defect surfaced at the first call rather than at the first
/// row of a long run.
pub fn schema_json() -> String {
    let mut out = String::new();
    out.push_str("{\n  ");
    push_quoted(&mut out, "schema_version");
    out.push_str(": ");
    push_quoted(&mut out, SCHEMA_VERSION);
    out.push_str(",\n");

    push_table(&mut out, TICKS_FILE, &csv_columns(&HEADER_EXEMPLAR));
    push_table(
        &mut out,
        PROVENANCE_FILE,
        &csv_columns(&provenance_exemplar()),
    );

    out.push_str("  ");
    push_quoted(&mut out, EVENTS_FILE);
    out.push_str(": [\n");
    let events = event_exemplars();
    for (at, event) in events.iter().enumerate() {
        out.push_str("    {\n      ");
        push_quoted(&mut out, "event");
        out.push_str(": ");
        push_quoted(&mut out, &event_tag(event));
        out.push_str(",\n      ");
        push_quoted(&mut out, "fields");
        out.push_str(": [\n");
        let fields = json_fields(event);
        for (n, (name, kind)) in fields.iter().enumerate() {
            push_field(&mut out, "        ", name, kind, n + 1 < fields.len());
        }
        out.push_str("      ]\n    }");
        if at + 1 < events.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

/// The first line at which two texts differ: the line number counting from one,
/// then that line from each side. `None` when the two texts are identical.
///
/// The raw equality assertion over a schema prints a multi-kilobyte
/// single-line escaped blob that nobody reads. A line number and two lines is a
/// diagnostic someone can act on.
///
/// Split **including** the terminator, so that two texts differing only in a
/// trailing newline are reported as differing rather than as equal. A line that
/// exists on one side only is reported as [`NO_SUCH_LINE`].
pub fn first_difference(left: &str, right: &str) -> Option<(usize, String, String)> {
    let mut lefts = left.split_inclusive('\n');
    let mut rights = right.split_inclusive('\n');
    let mut number = 0usize;
    loop {
        number += 1;
        match (lefts.next(), rights.next()) {
            (None, None) => return None,
            (left_line, right_line) if left_line == right_line => {}
            (left_line, right_line) => {
                return Some((
                    number,
                    left_line.map_or_else(|| NO_SUCH_LINE.to_owned(), str::to_owned),
                    right_line.map_or_else(|| NO_SUCH_LINE.to_owned(), str::to_owned),
                ));
            }
        }
    }
}

/// The generated schema (TICK-02).
///
/// Named `schema` so that `cargo test --lib log::schema` — the command this
/// plan's verification uses — reaches this module and not an empty set.
#[cfg(test)]
mod schema {
    use super::*;

    fn generated() -> serde_json::Value {
        serde_json::from_str(&schema_json()).expect("the generated schema is valid JSON")
    }

    /// The ordered `(name, type)` pairs the generated schema lists for one
    /// comma-separated table.
    fn listed(schema: &serde_json::Value, file: &str) -> Vec<(String, String)> {
        schema[file]
            .as_array()
            .unwrap_or_else(|| panic!("the schema lists {file}"))
            .iter()
            .map(|entry| {
                (
                    entry["name"]
                        .as_str()
                        .expect("an entry carries a name")
                        .to_owned(),
                    entry["dtype"]
                        .as_str()
                        .expect("an entry carries a type")
                        .to_owned(),
                )
            })
            .collect()
    }

    /// The ordered `(name, type)` pairs listed for one event variant.
    fn variant(schema: &serde_json::Value, tag: &str) -> Vec<(String, String)> {
        let entry = schema[EVENTS_FILE]
            .as_array()
            .expect("the schema lists the event stream")
            .iter()
            .find(|entry| entry["event"].as_str() == Some(tag))
            .unwrap_or_else(|| panic!("the schema lists the {tag} variant"));
        entry["fields"]
            .as_array()
            .expect("a variant carries its fields")
            .iter()
            .map(|field| {
                (
                    field["name"]
                        .as_str()
                        .expect("a field carries a name")
                        .to_owned(),
                    field["dtype"]
                        .as_str()
                        .expect("a field carries a type")
                        .to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn two_calls_in_one_process_return_identical_bytes() {
        assert_eq!(schema_json(), schema_json());
    }

    #[test]
    fn the_key_order_comes_from_the_text_and_not_from_the_parsed_value() {
        // The load-bearing property of the whole design, and the one a
        // container would quietly take away: the record parsed back from the
        // emitted text has a key-ordered backing map, so a reader that took its
        // order from THERE would report every record alphabetically. These
        // fields are declared in an order no sort produces.
        #[derive(Serialize)]
        struct OutOfOrder {
            zebra: i64,
            alpha: String,
            middle: bool,
        }

        let fields = json_fields(&OutOfOrder {
            zebra: 1,
            alpha: "a".to_owned(),
            middle: true,
        });
        assert_eq!(
            fields,
            vec![
                ("zebra".to_owned(), "int64"),
                ("alpha".to_owned(), "string"),
                ("middle".to_owned(), "bool"),
            ],
            "the field order was taken from the parsed value rather than from the emitted text"
        );
    }

    #[test]
    fn a_custom_serialised_address_is_typed_as_the_string_it_becomes() {
        // THE measurement that rejects a schema-derive crate, asserted here as
        // a property of this generator rather than left in the research. A
        // derive is a second description that cannot see
        // `#[serde(serialize_with = …)]`; compiled against this exact type it
        // declares both address fields structured objects, while the writer
        // emits `household:12`. Reading the type out of the emitted text
        // reports what the field actually became.
        use crate::books::{Posting, PostingKind};
        use crate::ids::{Account, FirmId, FirmSlot, GoodId, HouseholdId};

        let posting = Posting {
            seq: 0,
            kind: PostingKind::Transfer,
            debit: Account::Household(HouseholdId(12)),
            credit: Account::Firm(FirmId {
                slot: FirmSlot(3),
                generation: 0,
            }),
            debit_cents: 100,
            credit_cents: 100,
            good: GoodId(0),
            units_out: 0,
            units_in: 0,
            cash_residual_cents: 0,
            goods_residual_units: 0,
        };

        let text = serde_json::to_string(&posting).expect("a posting serialises");
        assert!(
            text.contains("\"debit\":\"household:12\""),
            "the writer no longer renders an address as a short string: {text}"
        );

        let fields = json_fields(&posting);
        for name in ["debit", "credit"] {
            let (_, kind) = fields
                .iter()
                .find(|(field, _)| field == name)
                .unwrap_or_else(|| panic!("a posting carries a {name} field"));
            assert_eq!(
                *kind, "string",
                "{name} was typed as something other than the string it renders as"
            );
        }
    }

    #[test]
    fn every_tick_column_is_an_integer_in_file_order() {
        let schema = generated();
        let listed = listed(&schema, TICKS_FILE);

        let names: Vec<String> = listed.iter().map(|(name, _)| name.clone()).collect();
        assert_eq!(
            names,
            ticks_header(),
            "the schema and the header the writer emits disagree on the tick columns"
        );
        for (name, kind) in &listed {
            assert_eq!(kind, "int64", "the tick column {name} is not an integer");
        }
    }

    #[test]
    fn the_provenance_table_carries_seven_columns_with_their_declared_types() {
        let schema = generated();
        let listed = listed(&schema, PROVENANCE_FILE);
        assert_eq!(
            listed.len(),
            7,
            "the provenance table lost or gained a column"
        );

        // Written out ONCE, here. This is the contract Phase 4 reads a type
        // from for a table that is legitimately empty (TICK-07).
        assert_eq!(
            listed,
            vec![
                ("tick".to_owned(), "int64".to_owned()),
                ("agent".to_owned(), "string".to_owned()),
                ("decision".to_owned(), "string".to_owned()),
                ("input_a".to_owned(), "int64".to_owned()),
                ("input_b".to_owned(), "int64".to_owned()),
                ("outcome".to_owned(), "int64".to_owned()),
                ("rule".to_owned(), "string".to_owned()),
            ]
        );
    }

    #[test]
    fn every_variant_lists_its_fields_in_declaration_order_with_the_tag_first() {
        let schema = generated();

        // Written out ONCE, here, and compared against the generated text: a
        // expectation rebuilt from the type would agree with any reordering the
        // type underwent. This is the same line the writer emits, read field by
        // field.
        assert_eq!(
            variant(&schema, "hire"),
            vec![
                ("event".to_owned(), "string".to_owned()),
                ("tick".to_owned(), "int64".to_owned()),
                ("firm".to_owned(), "string".to_owned()),
                ("household".to_owned(), "string".to_owned()),
                ("wage_cents".to_owned(), "int64".to_owned()),
            ]
        );
        assert_eq!(
            variant(&schema, "endowment"),
            vec![
                ("event".to_owned(), "string".to_owned()),
                ("tick".to_owned(), "int64".to_owned()),
                ("account".to_owned(), "string".to_owned()),
                ("cash_cents".to_owned(), "int64".to_owned()),
                ("units".to_owned(), "int64".to_owned()),
            ]
        );

        // And the tag is first for every variant, checked against the bytes the
        // writer produces rather than against the list above.
        for event in event_exemplars() {
            let tag = event_tag(&event);
            let line = serde_json::to_string(&event).expect("an event serialises");
            assert!(
                line.starts_with(&format!("{{\"event\":\"{tag}\",")),
                "the tag is not the first field of {line}"
            );
            let listed = variant(&schema, &tag);
            assert_eq!(
                listed[0].0, "event",
                "the schema lists {tag} without its tag"
            );
            assert_eq!(
                listed.len(),
                json_fields(&event).len(),
                "the schema and the emitted record disagree on the field count for {tag}"
            );
        }
    }

    #[test]
    fn nothing_is_unsupported_and_no_type_is_fractional() {
        let text = schema_json();
        assert!(
            !text.contains("UNSUPPORTED"),
            "a field serialised to a shape the classifier does not understand"
        );
        assert!(
            !text.contains("float"),
            "a fractional type reached one of the two tabular files"
        );
        assert!(!text.contains("null"), "an absent value reached the schema");
    }

    #[test]
    fn exactly_one_type_name_per_line_in_the_pretty_printed_spelling() {
        // The build's fractional-type check is a bare substring grep over this
        // hand-composed text, so the spelling is pinned here rather than there.
        let text = schema_json();
        assert!(
            !text.contains("\"dtype\":\""),
            "a type was emitted in the compact spelling, without the space after the colon"
        );
        for line in text.lines() {
            assert!(
                line.matches("\"dtype\"").count() <= 1,
                "more than one type name on one line: {line}"
            );
        }
        assert!(
            text.contains("{ \"name\": \"tick\", \"dtype\": \"int64\" },"),
            "the one-field-per-line spelling changed: {text}"
        );
    }

    #[test]
    fn the_classifier_reports_an_unsupported_shape_rather_than_guessing() {
        use serde_json::json;

        assert_eq!(value_kind(&json!("household:12")), "string");
        assert_eq!(value_kind(&json!(true)), "bool");
        assert_eq!(value_kind(&json!(-7)), "int64");
        assert_eq!(value_kind(&json!(u64::MAX)), "int64");
        assert_eq!(value_kind(&json!(null)), "null");
        assert_eq!(value_kind(&json!([1, 2])), "UNSUPPORTED");
        assert_eq!(value_kind(&json!({ "a": 1 })), "UNSUPPORTED");
    }

    #[test]
    fn the_difference_helper_reports_the_first_differing_line() {
        let left = "one\ntwo\nthree\n";
        let right = "one\nTWO\nthree\n";
        assert_eq!(
            first_difference(left, right),
            Some((2, "two\n".to_owned(), "TWO\n".to_owned()))
        );

        // A line present on one side only, and a difference that is only a
        // trailing terminator — both are differences, and a helper that split
        // on lines alone would miss the second.
        assert_eq!(
            first_difference("one\n", "one\ntwo\n"),
            Some((2, NO_SUCH_LINE.to_owned(), "two\n".to_owned()))
        );
        assert_eq!(
            first_difference("one\n", "one"),
            Some((1, "one\n".to_owned(), "one".to_owned()))
        );
    }

    #[test]
    fn identical_texts_have_no_difference() {
        assert_eq!(first_difference(&schema_json(), &schema_json()), None);
        assert_eq!(first_difference("", ""), None);
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

/// The decision-provenance table (TICK-07).
///
/// Named `provenance` so that `cargo test --lib log::provenance` reaches this
/// module and not an empty set.
#[cfg(test)]
mod provenance {
    use super::*;

    fn a_row(tick: u32) -> ProvenanceRow {
        ProvenanceRow {
            tick,
            agent: "firm:3:0".to_owned(),
            decision: Decision::Price,
            input_a: 1_200,
            input_b: 47,
            outcome: 1_250,
            rule: Rule::Raised,
        }
    }

    fn text_of(directory: &tempfile::TempDir) -> String {
        std::fs::read_to_string(directory.path().join(PROVENANCE_FILE))
            .expect("the provenance file exists")
    }

    #[test]
    fn the_header_is_the_declared_column_order() {
        // Written out ONCE, here, precisely because `provenance_header`
        // derives it: if both sides derived it, the test would compare the
        // function with itself and pass however the columns were renamed. This
        // list is the contract plan 03-04 freezes.
        assert_eq!(
            provenance_header(),
            vec![
                "tick", "agent", "decision", "input_a", "input_b", "outcome", "rule",
            ]
        );
    }

    #[test]
    fn the_header_comes_from_the_same_derivation_the_tick_file_uses() {
        // Two independent header sources is the defect this asserts against:
        // the provenance header must be what `header_of` produces from the row
        // type, exactly as the tick header is.
        assert_eq!(provenance_header(), header_of(provenance_exemplar()));
        assert_eq!(ticks_header(), header_of(HEADER_EXEMPLAR));
        assert_ne!(
            provenance_header(),
            ticks_header(),
            "two different tables must not share a column list"
        );
    }

    #[test]
    fn a_writer_that_received_no_row_still_leaves_a_full_header() {
        // The default outcome at this phase, and the one the measured defect
        // lives in: without the eager header this file is ZERO BYTES, the
        // analysis side raises rather than returning an empty frame, and a hash
        // comparison against another empty file compares the digest of the
        // empty string with itself.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(directory.path()).expect("the run writer opens");
        writer.finish().expect("the run writer finishes");

        let text = text_of(&directory);
        assert_eq!(
            text,
            format!("{}\n", provenance_header().join(",")),
            "a zero-row run must leave a header, not a zero-byte file"
        );
        assert_eq!(text.lines().count(), 1, "exactly one line: {text:?}");
    }

    #[test]
    fn the_header_is_written_exactly_once() {
        // The double-header defect is silent: the writer emits its OWN header
        // on the first serialised row, so a file with a hand-written header as
        // well still opens — and the duplicate reads back as a row of text,
        // widening every column with it.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(directory.path()).expect("the run writer opens");
        writer.provenance(a_row(0));
        writer.provenance(a_row(1));
        writer.finish().expect("the run writer finishes");

        let text = text_of(&directory);
        let lines: Vec<&str> = text.lines().collect();
        let header = provenance_header().join(",");
        assert_eq!(lines.len(), 3, "header plus two rows: {text:?}");
        assert_eq!(lines[0], header);
        assert_ne!(
            lines[1], header,
            "the header was emitted a second time, and the duplicate reads back as a row of text"
        );
        assert_eq!(
            text.matches(&header).count(),
            1,
            "the header string appears more than once: {text:?}"
        );
    }

    #[test]
    fn one_row_writes_a_header_line_then_a_data_line() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(directory.path()).expect("the run writer opens");
        writer.provenance(a_row(7));
        writer.finish().expect("the run writer finishes");

        let text = text_of(&directory);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "header then one row: {text:?}");
        assert_eq!(lines[0], provenance_header().join(","));
        assert_eq!(lines[1], "7,firm:3:0,price,1200,47,1250,raised");
        assert!(
            !text.contains('\r'),
            "the line terminator carries no carriage return"
        );
    }

    #[test]
    fn no_cell_is_ever_empty() {
        // An empty cell is what an optional column writes, and one missing
        // value widens an otherwise-integer column to a fractional one.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let mut writer = RunWriter::new(directory.path()).expect("the run writer opens");
        writer.provenance(a_row(0));
        writer.finish().expect("the run writer finishes");

        let text = text_of(&directory);
        assert!(
            !text.contains(",,"),
            "an empty cell reached the file: {text:?}"
        );
        for line in text.lines() {
            let cells: Vec<&str> = line.split(',').collect();
            assert_eq!(cells.len(), 7, "a row of the wrong width: {line}");
            for (at, cell) in cells.iter().enumerate() {
                assert!(!cell.is_empty(), "column {at} is empty in {line:?}");
            }
        }
    }

    #[test]
    fn the_decision_and_rule_columns_are_closed_vocabularies() {
        // TICK-07's "never free text" is a property of the two enumerations,
        // and this asserts the wire form each of their variants takes. A
        // caller cannot write a token that is not in one of these two lists,
        // because there is no constructor that would accept one.
        let decisions: Vec<String> = Decision::ALL
            .iter()
            .map(|d| serde_json::to_string(d).expect("a decision serialises"))
            .collect();
        assert_eq!(
            decisions,
            vec!["\"price\"", "\"wage\"", "\"hire\""],
            "the decision vocabulary changed shape"
        );

        let rules: Vec<String> = Rule::ALL
            .iter()
            .map(|r| serde_json::to_string(r).expect("a rule serialises"))
            .collect();
        assert_eq!(
            rules,
            vec!["\"raised\"", "\"lowered\"", "\"held\"", "\"bounded\""],
            "the rule vocabulary changed shape"
        );
    }

    #[test]
    fn the_vector_sink_keeps_the_rows_in_order() {
        let mut sink = VecSink::default();
        sink.provenance(a_row(0));
        sink.provenance(a_row(1));
        sink.finish().expect("the vector sink finishes");
        assert_eq!(sink.provenance, vec![a_row(0), a_row(1)]);
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
