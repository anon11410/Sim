//! The log seam: where a run's per-tick series leaves the process (TICK-03).
//!
//! One trait, three implementations, one row type.
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
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The per-tick series, relative to the run directory.
pub const TICKS_FILE: &str = "ticks.csv";

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

/// The column names of [`TickRow`], in file order, read out of the
/// serialisation itself.
///
/// Derived rather than written out, for the reason the module docs give: a
/// hand-typed list is a second source of truth that a field rename leaves
/// stale, and the file would then disagree with the schema that claims to
/// describe it.
///
/// # Panics
///
/// If [`TickRow`] cannot be serialised as a flat record. That is a defect in
/// the row type — a nested or sequence-valued field — and it is caught here, on
/// any call, rather than on the first tick of a long run.
pub fn ticks_header() -> Vec<String> {
    let mut probe = csv::Writer::from_writer(Vec::new());
    probe
        .serialize(HEADER_EXEMPLAR)
        .expect("TickRow is flat and serialises as one record");
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

    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A sink that keeps the rows, so a test can assert against the records rather
/// than against a re-parsed file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VecSink {
    pub rows: Vec<TickRow>,
}

impl Sink for VecSink {
    fn tick_row(&mut self, row: TickRow) {
        self.rows.push(row);
    }

    fn finish(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The disk writer: one run directory, one per-tick series inside it.
pub struct RunWriter {
    ticks: csv::Writer<File>,
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
        Ok(RunWriter {
            ticks,
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

    fn finish(&mut self) -> io::Result<()> {
        let flushed = self.ticks.flush();
        match self.first_error.take() {
            Some(error) => Err(error),
            None => flushed,
        }
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
