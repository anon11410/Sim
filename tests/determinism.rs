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
//!
//! **The diffed set is enumerated from the run directory, never listed here.**
//! A hand-written list of files to compare silently stops covering a file a
//! later phase adds, and the test that used it stays green while the coverage
//! shrinks. [`the_exclusion_is_enforced_not_documented`] reads the directory,
//! subtracts [`EXCLUDED_FROM_DIFF`], and asserts the count it actually diffed —
//! so a new file is compared automatically or excluded deliberately, and cannot
//! be skipped by omission.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;
use sim::books::Books;
use sim::config::Params;
use sim::invariants::CheckSet;
use sim::log::{
    Decision, EVENTS_FILE, PROVENANCE_FILE, RUN_META_FILE, Rule, RunWriter, Sink, TICKS_FILE,
    provenance_header, schema_json, ticks_header,
};
use sim::phases::Ctx;
use sim::rng::Rngs;
use sim::world::World;

/// The shipped configuration, spelled absolutely so no test here depends on the
/// working directory a harness happens to choose.
const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

/// This repository's root, as the compiler knew it.
///
/// Known to the test and therefore searchable: it is the path the configuration
/// was read from, and it may not appear in anything a run writes.
const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The seed the same-seed claims are made at.
const SEED: u64 = 42;

/// A second seed, for the claim that the seed reaches a written byte.
const OTHER_SEED: u64 = 43;

/// A decade of daily ticks.
///
/// **A literal, deliberately, and the only one in this file.** TICK-08 is a
/// claim about a decade. A test that read the run length out of the same
/// configuration it is exercising would certify whatever that file happened to
/// say — including a shortened run someone left behind while debugging — and
/// would report a green decade for a run of eleven ticks. The literal is
/// compared *against* the configuration, so a deliberate change to the run
/// length fails here and gets reconsidered rather than absorbed.
const DECADE_TICKS: u32 = 3650;

/// The files a run directory is not diffed on.
///
/// **One entry, one place.** The run record is the single quarantined file: it
/// is the only one permitted a wall clock, and it carries the compiler string,
/// which differs between two machines that must still agree on the economy. A
/// future phase that adds a file to the run directory declares its exclusion
/// here or nowhere — clause 3 of [`the_exclusion_is_enforced_not_documented`]
/// diffs everything this list does not name, so an undeclared file is compared
/// rather than skipped.
///
/// Spelled from the library's own constant rather than as a literal, so a
/// renamed file cannot leave this list pointing at a name nothing writes.
const EXCLUDED_FROM_DIFF: [&str; 1] = [RUN_META_FILE];

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

// ---------------------------------------------------------------------------
// TICK-05, TICK-06: the exclusion, and what a diffed file may contain.
// ---------------------------------------------------------------------------

/// The parameters the shipped configuration asks for.
fn configured() -> Params {
    sim::config::load(Path::new(CONFIG))
        .expect("the shipped configuration loads")
        .0
}

/// The names of everything in a run directory, in a set with a defined order.
///
/// A `BTreeSet`, never a hashed one — the project bans hashed iteration for the
/// reason this file exists, and a set whose order varies would make a failure
/// message report a different file each run.
fn entries(directory: &Path) -> BTreeSet<String> {
    std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("the run directory is readable: {error}"))
        .map(|entry| {
            entry
                .expect("a directory entry reads")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// Invoke the built binary and return the PROCESS IDENTIFIER it ran under.
///
/// Spawned directly rather than through the process-testing crate's assertion
/// builder for one reason: that builder does not surface the child's
/// identifier, and clause 4 of the exclusion test needs it. The binary is the
/// same one — `CARGO_BIN_EXE_sim` is set by cargo to the artefact built for
/// this test run, so this is not a path assembled by hand into the target
/// directory.
fn spawn_binary(seed: u64, out: &Path) -> u32 {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_sim"))
        .args(["--config", CONFIG])
        .args(["--seed", &seed.to_string()])
        .arg("--out")
        .arg(out)
        .spawn()
        .expect("the sim binary spawns");
    let identifier = child.id();
    let status = child.wait().expect("the sim binary is waited on");
    assert!(status.success(), "the binary exited {:?}", status.code());
    identifier
}

/// Every distinct run of ASCII letters in `text`.
fn alphabetic_words(text: &str) -> BTreeSet<String> {
    let mut words = BTreeSet::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_alphabetic() {
            current.push(character);
        } else if !current.is_empty() {
            words.insert(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.insert(current);
    }
    words
}

/// Every word the wire format itself declares.
///
/// **Derived from the GENERATOR, not from a run's output.** A vocabulary read
/// out of the artefact under test would contain whatever leaked into it and
/// would permit exactly the thing it is meant to catch. `schema_json` is the
/// committed contract's source, and `tests/log_schema.rs` holds it to the
/// committed file, so this vocabulary cannot drift from the format either.
///
/// **The two closed value vocabularies are added, and are not optional.** The
/// schema declares the provenance table's `decision` and `rule` COLUMNS; the
/// names those columns hold are declared by the enumerations instead. This
/// phase writes no provenance rows, so omitting them costs nothing today and
/// would make this clause fire on the first correct row Phase 6 writes — a
/// guard that goes red on correct output is one the next reader loosens, and
/// loosening it is how the clause stops catching a host name.
fn wire_vocabulary() -> BTreeSet<String> {
    let mut words = alphabetic_words(&schema_json());
    for decision in Decision::ALL {
        words.append(&mut alphabetic_words(&rendered(&decision)));
    }
    for rule in Rule::ALL {
        words.append(&mut alphabetic_words(&rendered(&rule)));
    }
    words
}

/// A value as the writers spell it.
fn rendered<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("a closed vocabulary serialises")
}

/// A calendar date, as a byte shape.
const DATE_SHAPE: &str = "dddd-dd-dd";

/// A clock reading, as a byte shape.
const CLOCK_SHAPE: &str = "dd:dd:dd";

/// The first run of bytes in `text` matching `shape`, where `d` stands for any
/// ASCII digit and every other character stands for itself.
///
/// Shapes rather than substrings, because a wall clock has no fixed value to
/// search for. Both shapes are impossible in a comma-separated series of
/// integers — a minus sign only ever follows a comma or a line start, and a
/// colon in the event stream only ever follows a letter — so a match is a
/// finding rather than a coincidence.
fn shape_match(text: &str, shape: &str) -> Option<String> {
    let characters: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = shape.chars().collect();
    if characters.len() < pattern.len() {
        return None;
    }
    for start in 0..=characters.len() - pattern.len() {
        let window = &characters[start..start + pattern.len()];
        let matched = window.iter().zip(pattern.iter()).all(|(seen, wanted)| {
            if *wanted == 'd' {
                seen.is_ascii_digit()
            } else {
                seen == wanted
            }
        });
        if matched {
            return Some(window.iter().collect());
        }
    }
    None
}

/// A run's own record, parsed.
fn run_meta(out: &Path) -> Value {
    serde_json::from_slice(&read_nonempty(&out.join(RUN_META_FILE)))
        .expect("the run record is JSON")
}

/// Two runs at one seed, one exclusion, and four claims that are four different
/// claims.
///
/// The clauses are numbered in the test body. None is redundant, and the test
/// was watched failing on clause 3 against the naive build, where both the tick
/// file and the event stream were zero bytes.
///
/// **On the process identifier and the wall clock.** The two runs below are two
/// separate processes, started one after the other, writing into two
/// differently named directories. Every diffed file is asserted byte-equal
/// across them — so a process identifier, a per-run directory name or a clock
/// reading cannot have reached a diffed byte, because all three differed
/// between the two runs. The identifiers are asserted to differ so that this
/// reasoning is not vacuous.
///
/// **A substring search for the identifier's digits is deliberately refused.**
/// It is the obvious spelling and it is wrong: measured on a real 3,650-tick
/// `ticks.csv`, **42.0%** of all five-digit numbers occur somewhere in it by
/// coincidence (4.9% of six-digit, 0.47% of seven-digit), and this host's
/// `pid_max` is 32768, so every identifier here is at most five digits. Such a
/// search would be red against a correct simulation roughly two runs in five,
/// and what it would be measuring is digit coincidence, not information
/// disclosure. The clauses below cover the same ground soundly: byte-equality
/// across two processes for anything that varies per run, a closed vocabulary
/// for anything alphabetic, path separators and known paths for anything
/// path-shaped, and two byte shapes for anything timestamp-shaped.
#[test]
fn the_exclusion_is_enforced_not_documented() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let first = root.path().join("first");
    let second = root.path().join("second");
    let first_identifier = spawn_binary(SEED, &first);
    let second_identifier = spawn_binary(SEED, &second);

    let vocabulary = wire_vocabulary();

    // The scanners, checked against a fabricated leak BEFORE they are trusted
    // on the real files. A scanner that matches nothing passes over everything,
    // which is the shape of failure this whole file is about.
    assert!(shape_match("started 2026-08-31 here", DATE_SHAPE).is_some());
    assert!(shape_match("0,2000000,1000000,3300", DATE_SHAPE).is_none());
    assert!(shape_match("at 14:07:33 today", CLOCK_SHAPE).is_some());
    assert!(shape_match("household:12,firm:3:0", CLOCK_SHAPE).is_none());
    assert!(alphabetic_words("/tmp/.tmpQ1z/first").contains("tmp"));
    // The closed value vocabularies really did reach the set: without them the
    // clause below fires on the first correct provenance row a later phase
    // writes, and the repair someone reaches for is to delete the clause.
    assert!(
        vocabulary.contains("price") && vocabulary.contains("held"),
        "the decision and rule vocabularies are missing from the wire vocabulary",
    );
    assert!(
        !vocabulary.contains("tmp") && !vocabulary.contains("hostname"),
        "the wire vocabulary already admits a word a leak would carry, so \
         clause 4a would pass over that leak",
    );

    // 1. The two runs produced the same set of files.
    let files = entries(&first);
    assert_eq!(
        files,
        entries(&second),
        "two runs at one seed produced different sets of files",
    );

    // 2. The excluded file must EXIST. Excluding a file that was never written
    //    is a vacuous exclusion: it enforces nothing, and it is indistinguishable
    //    from an exclusion that is doing its job.
    for excluded in EXCLUDED_FROM_DIFF {
        assert!(
            files.contains(excluded),
            "{excluded} is excluded from the diff but was never written — a \
             vacuous exclusion enforces nothing",
        );
    }

    // 3. Every OTHER file, ENUMERATED FROM THE DIRECTORY.
    let mut diffed = 0usize;
    let mut words_seen = 0usize;
    for name in &files {
        if EXCLUDED_FROM_DIFF.contains(&name.as_str()) {
            continue;
        }
        let left = read_nonempty(&first.join(name));
        let right = read_nonempty(&second.join(name));
        assert_eq!(
            digest_of(&left),
            digest_of(&right),
            "{name} differs between two runs at seed {SEED}",
        );

        // 4. Nothing in a diffed file may come from the environment.
        let text = String::from_utf8(left).expect("a diffed file is text");

        // 4a. Every word is one the wire format declares. A host name, a user
        //     name, a path component or a month name is not.
        let words = alphabetic_words(&text);
        assert!(
            !words.is_empty(),
            "{name} carries no words at all, so the vocabulary clause below \
             would pass over it without looking at anything",
        );
        for word in &words {
            assert!(
                vocabulary.contains(word),
                "{name} carries the word {word:?}, which the wire format does \
                 not declare — a run's files carry the model's own vocabulary \
                 and nothing from the machine it ran on (TICK-06)",
            );
        }
        words_seen += words.len();

        // 4b. Nothing path-shaped, whether or not it is spelled in letters.
        assert!(
            !text.contains('/') && !text.contains('\\'),
            "{name} carries a path separator",
        );
        for known in [
            root.path().to_str().expect("the temporary path is text"),
            REPO_ROOT,
        ] {
            assert!(!text.contains(known), "{name} carries the path {known}");
        }

        // 4c. Nothing timestamp-shaped. The tick number is the only clock.
        for shape in [DATE_SHAPE, CLOCK_SHAPE] {
            assert!(
                shape_match(&text, shape).is_none(),
                "{name} carries {:?}, which is shaped like a wall-clock reading",
                shape_match(&text, shape).unwrap_or_default(),
            );
        }

        diffed += 1;
    }

    assert!(
        words_seen > 0,
        "the vocabulary clause examined no words across any file",
    );
    assert_eq!(
        diffed,
        files.len() - EXCLUDED_FROM_DIFF.len(),
        "the number of files diffed is not the directory's contents minus the \
         exclusion list — a file was skipped without being declared",
    );
    assert!(
        diffed >= 3,
        "only {diffed} files were diffed; a run that silently stopped writing \
         one would otherwise leave this test green",
    );

    // The premise clause 4's reasoning about process identity rests on.
    assert_ne!(
        first_identifier, second_identifier,
        "the two runs shared a process identifier, so the byte equality above \
         says nothing about whether one reached a diffed file",
    );
}

// ---------------------------------------------------------------------------
// TICK-03: the run directory is well formed.
// ---------------------------------------------------------------------------

/// One header line, one row per configured tick, no carriage return, no empty
/// field, and every field an integer.
///
/// The shape claims are made against the files a real run left behind rather
/// than against the row types, because the row type is not what the analysis
/// side reads.
#[test]
fn the_run_directory_is_well_formed() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let out = root.path().join("run");
    run_binary(SEED, &out);
    let params = configured();

    let files = entries(&out);
    for name in [TICKS_FILE, EVENTS_FILE, PROVENANCE_FILE, RUN_META_FILE] {
        assert!(files.contains(name), "{name} is missing from the run");
    }

    // The tick series.
    let text =
        String::from_utf8(read_nonempty(&out.join(TICKS_FILE))).expect("the tick file is text");
    assert!(
        !text.contains('\r'),
        "a carriage return reached the tick file",
    );
    assert!(text.ends_with('\n'), "the last row is terminated");

    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .expect("the tick file has a header")
        .split(',')
        .map(str::to_owned)
        .collect();
    // Against the library's own header, so a column that appeared without being
    // declared fails here rather than becoming part of the contract by writing
    // itself into the file. This is also what closes the one gap the timestamp
    // shapes cannot see: a clock recorded as a bare integer would need a column,
    // and a column that is not declared cannot exist.
    assert_eq!(
        header,
        ticks_header(),
        "the tick file's header is not the one the library declares",
    );

    let mut rows = 0u32;
    for (index, line) in lines.enumerate() {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(
            fields.len(),
            header.len(),
            "row {index} is not as wide as the header",
        );
        for (column, field) in header.iter().zip(fields.iter()) {
            assert!(!field.is_empty(), "row {index} has an empty {column} field");
            assert!(
                field.parse::<i64>().is_ok(),
                "row {index}'s {column} field {field:?} is not an integer",
            );
        }
        rows += 1;
    }
    assert_eq!(
        rows, params.sim.ticks,
        "one header line plus one row per configured tick",
    );

    // The decision-provenance table: a run that decided nothing still leaves a
    // full header line, because the measured default is a zero-byte file the
    // analysis side refuses to open.
    let provenance = String::from_utf8(read_nonempty(&out.join(PROVENANCE_FILE)))
        .expect("the provenance table is text");
    assert!(
        !provenance.contains('\r'),
        "a carriage return reached the provenance table",
    );
    let provenance_lines: Vec<&str> = provenance.lines().collect();
    assert_eq!(
        provenance_lines.len(),
        1,
        "this phase decides nothing, so the table is one header line",
    );
    assert_eq!(
        provenance_lines[0]
            .split(',')
            .map(str::to_owned)
            .collect::<Vec<String>>(),
        provenance_header(),
        "the provenance header is not the one the library declares",
    );

    // The event stream: one record per line, each a tagged object.
    let events =
        String::from_utf8(read_nonempty(&out.join(EVENTS_FILE))).expect("the event stream is text");
    assert!(
        !events.contains('\r'),
        "a carriage return reached the event stream",
    );
    assert!(events.ends_with('\n'), "the last record is terminated");
    let mut records = 0usize;
    for (index, line) in events.lines().enumerate() {
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("record {index} is not JSON: {error}"));
        assert!(
            value.get("event").and_then(Value::as_str).is_some(),
            "record {index} carries no event tag",
        );
        records += 1;
    }
    assert!(records > 0, "the event stream carries no records");
}

// ---------------------------------------------------------------------------
// TICK-08: a decade of empty ticks actually runs.
// ---------------------------------------------------------------------------

/// The configured decade, through the built binary, leaving a complete run
/// directory.
///
/// **The clean exit is the invariant claim.** A violation exits one and prints
/// to standard error; a run that completes is a run in which every active check
/// passed on every one of its ticks. There is nothing further to assert about
/// the invariants here that the exit code does not already carry.
#[test]
fn the_empty_decade_runs() {
    let params = configured();
    assert_eq!(
        params.sim.ticks, DECADE_TICKS,
        "the shipped configuration no longer runs a decade of daily ticks, so \
         this test would certify a shorter run under TICK-08's name",
    );

    let root = tempfile::tempdir().expect("a temporary directory");
    let out = root.path().join("run");
    run_binary(SEED, &out);

    let declared: BTreeSet<String> = [TICKS_FILE, EVENTS_FILE, PROVENANCE_FILE, RUN_META_FILE]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        entries(&out),
        declared,
        "the run directory is not the set of files the library and the binary \
         declare — a file appeared or went missing",
    );

    let text =
        String::from_utf8(read_nonempty(&out.join(TICKS_FILE))).expect("the tick file is text");
    assert_eq!(
        text.lines().count(),
        usize::try_from(DECADE_TICKS).expect("a decade of ticks fits an index") + 1,
        "the decade did not leave one header line plus one row per day",
    );

    let meta = run_meta(&out);
    assert_eq!(
        meta.get("ticks_completed").and_then(Value::as_u64),
        Some(u64::from(DECADE_TICKS)),
        "the run record does not report a completed decade",
    );
    assert_eq!(
        meta.get("exit").and_then(Value::as_str),
        Some("ok"),
        "the run record does not report a clean finish",
    );
}

// ---------------------------------------------------------------------------
// TICK-04: the replay origin agrees with the series it explains.
// ---------------------------------------------------------------------------

/// The endowment records sum, in cents, to the tick series' money column.
///
/// **Two independently produced numbers, neither of them a literal.** One comes
/// out of the event stream, the other out of the tick series; a literal would
/// only assert that someone once wrote the same number twice. This is the
/// origin row Phase 4's conservation replay is anchored to, checked here rather
/// than assumed there.
#[test]
fn endowment_events_sum_to_the_money_stock() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let out = root.path().join("run");
    run_binary(SEED, &out);

    let events =
        String::from_utf8(read_nonempty(&out.join(EVENTS_FILE))).expect("the event stream is text");
    let mut records = 0usize;
    let mut cents: i64 = 0;
    for (index, line) in events.lines().enumerate() {
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("record {index} is not JSON: {error}"));
        if value.get("event").and_then(Value::as_str) != Some("endowment") {
            continue;
        }
        let cash = value
            .get("cash_cents")
            .and_then(Value::as_i64)
            .unwrap_or_else(|| panic!("endowment record {index} carries no cash field"));
        cents = cents
            .checked_add(cash)
            .expect("the endowment sum does not overflow");
        records += 1;
    }
    assert!(
        records > 0,
        "no endowment record was found — the sum would be zero, and a zero \
         summed from nothing is the vacuous half of this comparison",
    );

    let money: i64 = first_row_column(&out, "total_money_cents")
        .parse()
        .expect("the money column is an integer");
    assert!(
        money > 0,
        "the money stock is zero, so this comparison is 0 == 0"
    );
    assert_eq!(
        cents, money,
        "the {records} endowment records sum to {cents} cents, while the tick \
         series opens at {money} — Phase 4's replay origin does not agree with \
         the series it is meant to explain",
    );
}

// ---------------------------------------------------------------------------
// TICK-05: the run record, and nothing the exclusion is not a licence for.
// ---------------------------------------------------------------------------

/// The word in `key` that a run record may not carry, if there is one.
fn forbidden_word(key: &str) -> Option<&'static str> {
    const FORBIDDEN: [&str; 8] = [
        "duration", "elapsed", "host", "pid", "process", "path", "dir", "user",
    ];
    let lowered = key.to_lowercase();
    FORBIDDEN.into_iter().find(|word| lowered.contains(word))
}

/// The seed, the configuration digest and the compiler — present, non-empty,
/// and accompanied by nothing else.
///
/// **The exclusion from the diff is a permission for a wall clock, not a
/// general environment allowance** for a file that ships beside the logs. The
/// compiler string legitimately carries a release date; that is the one dated
/// value in a run directory and it is why this file is the excluded one.
#[test]
fn run_meta_carries_the_three_fields() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let out = root.path().join("run");
    run_binary(SEED, &out);

    // The predicate, checked against a fabricated field before it is trusted.
    assert_eq!(forbidden_word("duration_ms"), Some("duration"));
    assert_eq!(forbidden_word("hostname"), Some("host"));
    assert_eq!(forbidden_word("seed"), None);

    let meta = run_meta(&out);
    let object = meta.as_object().expect("the run record is a JSON object");

    // The value that runs must be the value recorded: this run was given the
    // seed by an override, and the record must show the override.
    assert_eq!(
        object.get("seed").and_then(Value::as_u64),
        Some(SEED),
        "the recorded seed is not the seed the run was given",
    );

    // Independently produced on both sides: the record's digest against a
    // digest this test takes of the same file.
    let recorded = object
        .get("config_sha256")
        .and_then(Value::as_str)
        .expect("the run record carries a configuration digest");
    assert!(!recorded.is_empty(), "the configuration digest is empty");
    assert_eq!(
        recorded,
        digest_of(&read_nonempty(Path::new(CONFIG))),
        "the recorded digest is not the digest of the configuration that ran",
    );

    let compiler = object
        .get("rustc")
        .and_then(Value::as_str)
        .expect("the run record carries a compiler string");
    assert!(!compiler.is_empty(), "the compiler string is empty");

    for key in object.keys() {
        assert!(
            forbidden_word(key).is_none(),
            "the run record carries a {key} field — the diff exclusion is a \
             permission for a wall clock, not for the environment",
        );
    }
    for (key, value) in object {
        if let Some(text) = value.as_str() {
            assert!(
                !text.contains('/') && !text.contains('\\'),
                "the {key} field carries a path",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// ROADMAP Phase 3 criterion 6: the PROCESS halts, non-zero, naming tick 0.
// ---------------------------------------------------------------------------

/// The shipped liveness leaf, in its off state.
///
/// Anchored between two line terminators so it matches a whole leaf and not a
/// mention of one inside the comment block above it.
const LIVENESS_OFF: &str = "\nliveness_enabled = false\n";

/// The same leaf, on.
const LIVENESS_ON: &str = "\nliveness_enabled = true\n";

/// The built binary, run against the shipped configuration with one leaf moved,
/// exits one and says why on standard error.
///
/// **The process-level half of Phase 2's halt claim.** Phase 2 could prove only
/// that a tick loop aborts at the right tick, because the phase table and the
/// binary's loop are this phase's. The criterion is recorded against this phase
/// so it falls between neither.
///
/// **Why tick 0, and why this needs no fault injection.** `Books::new` clears
/// the endowment postings at construction, precisely so the liveness check
/// cannot be satisfied by them. Tick 0's journal is therefore empty,
/// `transactions_this_tick()` is zero, and the check fires on the first tick.
/// That is Phase 2's construction doing the work: the liveness violation is the
/// one violation reachable through the ledger's public API, so nothing here
/// goes near the corruption vocabulary the lint gate keeps out of this
/// directory.
///
/// **Why the override is a file and not an environment variable.** An
/// environment variable is an input present in neither the committed
/// configuration nor the run's own record, so a run configured that way cannot
/// be reproduced from the repository. This is a project prohibition, not a
/// preference, and `tests/lints.sh` is not what enforces it here — the plan's
/// own check greps this file for one.
///
/// **Why the override is textual and not a re-serialisation.** Round-tripping
/// the parsed parameters through the serialiser works and strips every comment,
/// and the comments carry the source grades `tests/provenance.rs` makes
/// load-bearing. The count assertion below is the point of the exercise: with a
/// reworded configuration the substitution would be a silent no-op, the binary
/// would complete its decade, this test would pass — and it would be passing as
/// a second copy of criterion 1 while claiming to prove criterion 6.
#[test]
fn the_binary_halts_on_a_liveness_violation_at_tick_zero() {
    let root = tempfile::tempdir().expect("a temporary directory");
    let shipped = String::from_utf8(read_nonempty(Path::new(CONFIG)))
        .expect("the shipped configuration is text");

    assert_eq!(
        shipped.matches(LIVENESS_OFF).count(),
        1,
        "expected exactly one liveness_enabled leaf to override; the shipped \
         configuration was reworded, and this substitution would have been a \
         silent no-op that left the gate off",
    );
    let overridden = shipped.replace(LIVENESS_OFF, LIVENESS_ON);
    assert_eq!(
        overridden.matches(LIVENESS_ON).count(),
        1,
        "the override did not put the leaf back exactly once",
    );
    assert!(
        !overridden.contains(LIVENESS_OFF),
        "the leaf is still off after the override",
    );

    // The substitution moved ONE leaf and nothing else. The grade comments are
    // what distinguish a textual override from a re-serialisation, and
    // `tests/provenance.rs` makes them load-bearing.
    assert_eq!(
        overridden.lines().count(),
        shipped.lines().count(),
        "the override changed the shape of the file, not one leaf in it",
    );
    assert_eq!(
        overridden.matches("# GRADE:").count(),
        shipped.matches("# GRADE:").count(),
        "the override lost a source grade — it was a re-serialisation, not a \
         textual substitution",
    );

    let config = root.path().join("liveness_on.toml");
    std::fs::write(&config, &overridden).expect("the overridden configuration writes");
    let out = root.path().join("run");

    let assertion = assert_cmd::Command::cargo_bin("sim")
        .expect("the sim binary is built for this test run")
        .args([
            "--config",
            config.to_str().expect("the temporary path is text"),
        ])
        .arg("--out")
        .arg(&out)
        .assert()
        .failure()
        .code(1);

    let stderr =
        String::from_utf8(assertion.get_output().stderr.clone()).expect("the halt message is text");
    assert!(
        stderr.contains("tick 0"),
        "the halt does not name tick 0: {stderr}",
    );
    assert!(
        stderr.contains("liveness"),
        "the halt does not name the check that fired: {stderr}",
    );

    // TICK-06 at the MESSAGE level. The source guard over the modules that
    // render this string is the static half; neither half is sufficient alone.
    // Both the configuration and the output directory live under this root, so
    // one search covers the two paths this run was given.
    let temporary = root.path().to_str().expect("the temporary path is text");
    assert!(
        !stderr.contains(temporary),
        "the halt message carries a path: {stderr}",
    );
    assert!(
        !stderr.contains(REPO_ROOT),
        "the halt message carries this repository's path: {stderr}",
    );

    // A halted run is self-describing: its own record says how far it got and
    // that it ended in a violation.
    let meta = run_meta(&out);
    assert_eq!(
        meta.get("ticks_completed").and_then(Value::as_u64),
        Some(0),
        "the run record does not report a halt before the first tick completed",
    );
    assert_eq!(
        meta.get("exit").and_then(Value::as_str),
        Some("violation"),
        "the run record does not report a violation",
    );

    // What the eager header buys: a halted run leaves an OPENABLE tick file
    // rather than a zero-byte one the analysis side refuses.
    let text =
        String::from_utf8(read_nonempty(&out.join(TICKS_FILE))).expect("the tick file is text");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "the halted run wrote {} lines; the failing tick is aborted at the \
         invariant phase, which runs before the log phase, so no row is due",
        lines.len(),
    );
    assert_eq!(
        lines[0]
            .split(',')
            .map(str::to_owned)
            .collect::<Vec<String>>(),
        ticks_header(),
        "the halted run's one line is not the header",
    );
}
