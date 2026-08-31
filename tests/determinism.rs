//! Reproducibility asserted on BYTES, in one process and across two (TICK-09,
//! TICK-10).
//!
//! **Why this file exists beside `tests/tracer_end_to_end.rs`.** That file
//! asserts at the COLUMN level: it reads a named column out of the first data
//! row and compares it. This file asserts at the FILE-BYTE level, and across a
//! process boundary. Neither subsumes the other and the overlap is deliberate.
//! A column comparison says *which* column stopped depending on the seed, which
//! a byte comparison cannot; a byte comparison covers every column, including
//! ones nobody thought to name, and a cross-process comparison sees global
//! state, environment leakage and allocator-order effects that an in-process
//! comparison is blind to by construction. The cheap smoke test has existed
//! since Phase 1 and stays where it is.
//!
//! **The mutation this file's value rests on.** `different_seed_differs` is the
//! test that closes the vacuous-reproducibility trap, and it was measured
//! failing before it was measured passing. Built exactly as ROADMAP criterion 3
//! originally prescribed — an activation shuffle consuming 218 draws a tick,
//! logged as a `rng_draws` COUNT — 3,650 ticks at seed 42 and at seed 43
//! produced BYTE-IDENTICAL tick files (`cmp` returned 0). The generator was
//! consumed and nothing observable depended on it. The repair was to log a
//! seed-sensitive VALUE, `activation_digest`, and the two runs then differ at
//! tick 0. Blanking that column makes this test red again; that is what gives
//! it teeth. Recorded here rather than only in a summary, because the next
//! reader's question about `different_seed_differs` is "has this ever failed?"
//!
//! **Every comparison asserts its inputs are non-empty first.** This is not
//! defensive habit. Against the naive build both `ticks.csv` and `events.jsonl`
//! were zero bytes, the comparison hashed the empty string twice, got
//! `e3b0c442…` on both sides, and passed while certifying nothing at all. Every
//! read in this file goes through [`read_nonempty`] so that a comparison over
//! an empty file cannot be written by accident.

use std::path::Path;

use sim::books::Books;
use sim::config::Params;
use sim::invariants::CheckSet;
use sim::log::{EVENTS_FILE, PROVENANCE_FILE, RunWriter, Sink, TICKS_FILE};
use sim::phases::Ctx;
use sim::rng::Rngs;
use sim::world::World;

/// The shipped configuration, spelled absolutely so no test here depends on the
/// working directory a harness happens to choose.
const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

/// The seed the same-seed claims are made at.
const SEED: u64 = 42;

/// A second seed, for the claim that the seed reaches a written byte.
const OTHER_SEED: u64 = 43;

/// Read a file, asserting it has content BEFORE any caller can compare it.
///
/// The single read path in this file, so that "the inputs to a comparison were
/// non-empty" is a structural property rather than a clause someone has to
/// remember. Two empty files hash equal, and a test that hashes them is green
/// and worthless — measured, not imagined: see the module docs.
fn read_nonempty(path: &Path) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap_or_else(|error| {
        panic!(
            "{} was expected in the run directory: {error}",
            display_name(path)
        )
    });
    assert!(
        !bytes.is_empty(),
        "{} is empty — comparing it to another empty file proves nothing",
        display_name(path)
    );
    bytes
}

/// The file's own name, for a message that names the artefact and not the
/// temporary directory it happened to land in.
fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || "the file".to_owned(),
        |name| name.to_string_lossy().into(),
    )
}

/// The SHA-256 of some bytes, as lowercase hex.
///
/// The library's own hash, reused rather than reimplemented. It is named for
/// its first caller — the configuration digest in the run record — but it is a
/// plain byte digest and needs no second hashing dependency here.
fn digest_of(bytes: &[u8]) -> String {
    sim::config::config_hash(bytes)
}

/// Run the shipped configuration to completion into `directory`, through the
/// library, at `seed`.
///
/// The same sequence `src/main.rs` performs, and deliberately so: the sink is a
/// trait object, so an in-process run and a run driven by the binary execute
/// the identical pipeline. The writer is FINISHED before anything is read back
/// — the comma-separated writer's drop-time flush discards its error, so a
/// truncated file would otherwise be read as though it were a whole run.
fn run_in_process(directory: &Path, seed: u64) -> Params {
    let (params, _hash) =
        sim::config::load(Path::new(CONFIG)).expect("the shipped configuration loads");

    let mut writer = RunWriter::new(directory).expect("the run writer opens");
    let mut books = Books::new(&params).expect("the shipped configuration opens books");
    let mut world = World::new(&params);
    let rngs = Rngs::new(seed);
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
    writer.finish().expect("the run writer finishes");

    params
}

/// Invoke the BUILT BINARY at `seed`, into `out`, asserting a clean exit.
///
/// Resolved through the process-testing crate rather than by assembling a path
/// into the target directory by hand: a hand-built path silently tests whatever
/// binary was there last, which on a failing build is the previous one.
fn run_binary(seed: u64, out: &Path) {
    assert_cmd::Command::cargo_bin("sim")
        .expect("the sim binary is built for this test run")
        .args(["--config", CONFIG])
        .args(["--seed", &seed.to_string()])
        .arg("--out")
        .arg(out)
        .assert()
        .success();
}

/// One column of the FIRST DATA ROW of a run's tick file, addressed by name out
/// of the header.
///
/// By name, never by position: a column that moved would otherwise silently
/// change what the caller claims to be asserting.
fn first_row_column(directory: &Path, column: &str) -> String {
    let text = String::from_utf8(read_nonempty(&directory.join(TICKS_FILE)))
        .expect("the tick file is text");
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .expect("the tick file has a header")
        .split(',')
        .collect();
    let at = header
        .iter()
        .position(|name| *name == column)
        .unwrap_or_else(|| panic!("no {column} column in {header:?}"));
    lines
        .next()
        .expect("the tick file has at least one data row")
        .split(',')
        .nth(at)
        .unwrap_or_else(|| panic!("the first data row is narrower than the header"))
        .to_owned()
}

// ---------------------------------------------------------------------------
// TICK-09: the same seed reproduces, in one process.
// ---------------------------------------------------------------------------

/// Two library runs at one seed, into two directories, byte for byte.
///
/// The narrowest of the three claims and the one that localises a failure
/// furthest: a failure here is inside the library — something read an unseeded
/// source, or iterated an unordered container — with no process boundary,
/// no argument parsing and no environment in the way.
#[test]
fn same_seed_identical_in_process() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let first = root.path().join("first");
    let second = root.path().join("second");

    run_in_process(&first, SEED);
    run_in_process(&second, SEED);

    // The three files the LIBRARY writes. The run record is the binary's, and
    // is the one file excluded from the diff; `the_exclusion_is_enforced_not_documented`
    // is where that exclusion is enforced, against a directory the binary wrote.
    let mut compared = 0;
    for name in [TICKS_FILE, EVENTS_FILE, PROVENANCE_FILE] {
        let left = read_nonempty(&first.join(name));
        let right = read_nonempty(&second.join(name));
        assert_eq!(
            digest_of(&left),
            digest_of(&right),
            "{name} differs between two in-process runs at seed {SEED}",
        );
        compared += 1;
    }
    assert_eq!(
        compared, 3,
        "three files were expected to be compared; a comparison that ran over \
         fewer would be a narrower claim than this test's name makes",
    );
}

// ---------------------------------------------------------------------------
// TICK-09: the same seed reproduces, across a process boundary.
// ---------------------------------------------------------------------------

/// Two invocations of the built binary at one seed, byte for byte.
///
/// A DIFFERENT claim from the in-process one, not a more expensive spelling of
/// it. Two processes have different address-space layouts, different allocator
/// histories, different process identifiers and a different environment block.
/// A failure here with the in-process test still green points at exactly that
/// class: global state, an environment read, an allocator-order effect.
#[test]
fn two_processes_at_one_seed_write_identical_bytes() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let mut digests: Vec<String> = Vec::new();

    for run in ["first", "second"] {
        let out = root.path().join(run);
        run_binary(SEED, &out);
        for name in [TICKS_FILE, EVENTS_FILE] {
            digests.push(digest_of(&read_nonempty(&out.join(name))));
        }
    }

    assert_eq!(
        digests.len(),
        4,
        "two runs of two files were expected to produce four digests",
    );
    assert_eq!(
        &digests[0..2],
        &digests[2..4],
        "two invocations of the binary at seed {SEED} wrote different bytes",
    );
}

// ---------------------------------------------------------------------------
// TICK-10: a different seed reaches a written byte.
// ---------------------------------------------------------------------------

/// Two seeds, one binary, and a tick file that must differ.
///
/// **Asserted on the tick file specifically, and that is not a weakening.** At
/// this phase `events.jsonl` carries only the opening endowment, which is read
/// from the ledger's accessors and is seed-independent — so the two seeds
/// produce IDENTICAL event streams, and an assertion that every diffed file
/// differs would be red against a correct simulation. Measured: `cmp` on the
/// two event streams returns equal. When a later phase makes an event depend on
/// a draw, this test gets stronger on its own; it does not need loosening now
/// to be honest.
///
/// **The difference is localised as well as detected.** A byte comparison that
/// fails says only that some byte moved. Reading the two columns by name says
/// which one stopped depending on the seed — and the pair below is the whole
/// lesson of the vacuous-reproducibility trap in two assertions: the draw COUNT
/// is identical at both seeds, and it was a run logging only that count which
/// produced byte-identical files while appearing to consume the generator.
#[test]
fn different_seed_differs() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let one = root.path().join("seed-one");
    let other = root.path().join("seed-other");

    run_binary(SEED, &one);
    run_binary(OTHER_SEED, &other);

    let left = read_nonempty(&one.join(TICKS_FILE));
    let right = read_nonempty(&other.join(TICKS_FILE));
    assert_ne!(
        digest_of(&left),
        digest_of(&right),
        "seeds {SEED} and {OTHER_SEED} wrote identical tick files — the generator \
         is consumed but nothing it touches reaches a diffed byte. The repair is \
         a seed-sensitive column, not a weaker assertion here",
    );

    assert_eq!(
        first_row_column(&one, "rng_draws"),
        first_row_column(&other, "rng_draws"),
        "the per-tick draw count is fixed-draw and therefore seed-independent; a \
         difference here is a fixed-draw-sampling violation, not reproducibility news",
    );
    assert_ne!(
        first_row_column(&one, "activation_digest"),
        first_row_column(&other, "activation_digest"),
        "the activation digest did not change with the seed at tick 0 — this is \
         the column the whole different-seed claim rests on",
    );
}
