//! The wire format as a committed contract, and the two files it describes
//! (TICK-02, TICK-03, TICK-07).
//!
//! Three claims, each about bytes rather than about intent.
//!
//! **The drift test never writes.** It reads the committed `schema/schema.json`
//! and compares the generator against it. Regeneration is an operator action —
//! the command is named in the failure message and in
//! [`sim::log::SCHEMA_REGEN_COMMAND`] — because a test that regenerated and
//! then compared would be comparing the generator with itself and would pass
//! however far the wire format drifted. This is the same discipline this
//! repository already applies to `clippy.toml`.
//!
//! **A drift test that has never been seen to fail is indistinguishable from
//! one comparing a file to itself.** `tests/schema_drift_negative.sh` perturbs
//! the committed artifact by a column swap, watches this test fail, restores
//! under a `trap` and watches it pass again. It is a build step, beside the
//! lint gate, for the reason that step's own comment gives.
//!
//! **The two shape tests assert on the files a real run leaves behind**, not on
//! the row types. The provenance test in particular asserts the *absence* of
//! rows: after a run that decides nothing the table must still be one full
//! header line, because the measured default is a zero-byte file that the
//! analysis side refuses to open rather than reading as an empty frame.

use std::path::Path;

use sim::books::Books;
use sim::config::Params;
use sim::invariants::CheckSet;
use sim::log::{
    PROVENANCE_FILE, RunWriter, SCHEMA_FILE, SCHEMA_REGEN_COMMAND, Sink, TICKS_FILE,
    first_difference, provenance_header, schema_json, ticks_header,
};
use sim::phases::Ctx;
use sim::rng::Rngs;
use sim::world::World;

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

/// The committed artifact. Spelled absolutely so the test does not depend on
/// the working directory a test harness happens to choose.
const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/schema.json");

/// Run the shipped configuration to completion into `directory`, and return the
/// parameters it ran with.
///
/// The whole pipeline through the real disk writer: the shape claims below are
/// then claims about an artefact a run actually produced.
fn run_into(directory: &Path) -> Params {
    let (params, _hash) =
        sim::config::load(Path::new(CONFIG)).expect("the shipped configuration loads");

    let mut writer = RunWriter::new(directory).expect("the run writer opens");
    let mut books = Books::new(&params).expect("the shipped configuration opens books");
    let mut world = World::new(&params);
    let rngs = Rngs::new(params.sim.seed);
    let checks = CheckSet::from_params(&params);
    {
        let mut ctx = Ctx {
            world: &mut world,
            books: &mut books,
            rngs: &rngs,
            checks: &checks,
            sink: &mut writer,
        };
        sim::phases::run(&mut ctx, params.sim.ticks).expect("the shipped configuration runs clean");
    }
    // Before anything is read back: the comma-separated writer's drop-time
    // flush discards its error, so a truncated file would otherwise be read as
    // though it were the whole run.
    writer.finish().expect("the run writer finishes");

    params
}

#[test]
fn schema_matches_the_committed_file() {
    assert!(
        COMMITTED.ends_with(SCHEMA_FILE),
        "this test reads {COMMITTED}, which is not the artifact the library names ({SCHEMA_FILE})"
    );

    // READ ONLY. This test must never write the file it is comparing against.
    let committed = std::fs::read_to_string(COMMITTED)
        .unwrap_or_else(|error| panic!("{COMMITTED} is committed and readable: {error}"));
    let generated = schema_json();

    if let Some((line, generated_line, committed_line)) = first_difference(&generated, &committed) {
        panic!(
            "schema drift at line {line}\n  \
             generated: {generated_line:?}\n  \
             committed: {committed_line:?}\n\
             The wire format and the committed contract have parted company. Regenerate \
             deliberately and review the diff:\n    {SCHEMA_REGEN_COMMAND}"
        );
    }
}

#[test]
fn ticks_csv_is_flat_and_integer_only() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let params = run_into(directory.path());

    let bytes = std::fs::read(directory.path().join(TICKS_FILE)).expect("the run left a tick file");
    let text = String::from_utf8(bytes).expect("the tick file is text");

    // The terminator, asserted on the raw text: a carriage return would reach
    // the analysis side as part of the last column's value.
    assert!(
        !text.contains('\r'),
        "a carriage return reached the tick file"
    );
    assert!(text.ends_with('\n'), "the last row is terminated");

    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .expect("the tick file has a header")
        .split(',')
        .collect();
    assert_eq!(
        header,
        ticks_header(),
        "the file's header and the header the writer derives have parted company"
    );

    // Money is named with a cents suffix, asserted as a property of the header
    // rather than as a repetition of it: a column naming money without the
    // suffix is a column someone will one day write a decimal into, and a
    // decimal column degrades the conservation audit from exact integer
    // equality to a tolerance check.
    let money = ["money", "cash", "price", "wage", "cost", "income"];
    let mut suffixed = 0;
    for column in &header {
        if column.ends_with("_cents") {
            suffixed += 1;
            continue;
        }
        for word in money {
            assert!(
                !column.contains(word),
                "the column {column} carries money without the _cents suffix"
            );
        }
    }
    assert!(suffixed >= 2, "no money column survives in {header:?}");

    let mut rows = 0;
    for (number, line) in lines.enumerate() {
        rows += 1;
        let cells: Vec<&str> = line.split(',').collect();
        assert_eq!(
            cells.len(),
            header.len(),
            "row {number} is {} cells wide, not {}: {line}",
            cells.len(),
            header.len()
        );
        for (at, cell) in cells.iter().enumerate() {
            // An empty cell is what an optional column writes, and one missing
            // value widens an otherwise-integer column to a fractional one.
            assert!(
                !cell.is_empty(),
                "row {number} column {} is empty: {line}",
                header[at]
            );
            assert!(
                cell.parse::<i64>().is_ok(),
                "row {number} column {} is not an integer: {cell:?}",
                header[at]
            );
        }
    }

    // The row count comes from the configuration, not from the file.
    assert_eq!(
        rows, params.sim.ticks as usize,
        "the run wrote a row count the configuration does not account for"
    );
}

#[test]
fn provenance_has_a_header_even_with_no_rows() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    run_into(directory.path());

    let path = directory.path().join(PROVENANCE_FILE);
    assert!(path.is_file(), "the run left no provenance table behind");

    let text = std::fs::read_to_string(&path).expect("the provenance table is readable");

    // The measured defect this exists to catch: a lazily-written header leaves
    // a ZERO-BYTE file, which the analysis side raises on rather than reading
    // as an empty frame — and two zero-byte files hash equal, so a determinism
    // comparison over them certifies nothing.
    assert!(
        !text.is_empty(),
        "a run that decided nothing left a zero-byte provenance table"
    );

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "expected the header and no rows; a second line means the header was written twice, \
         and the duplicate reads back as a row of text: {text:?}"
    );

    // The full seven-column header, spelled out ONCE here. Deriving both sides
    // would compare the derivation with itself and pass however the columns
    // were renamed.
    assert_eq!(
        lines[0], "tick,agent,decision,input_a,input_b,outcome,rule",
        "the provenance header changed shape"
    );
    assert_eq!(
        lines[0].split(',').count(),
        7,
        "the provenance table lost or gained a column"
    );
    assert_eq!(
        lines[0],
        provenance_header().join(","),
        "the file and the header the writer derives have parted company"
    );
    assert!(
        !text.contains('\r'),
        "a carriage return reached the provenance table"
    );
}
