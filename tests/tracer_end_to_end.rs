//! End-to-end check of the BUILT BINARY, not of each layer in isolation.
//!
//! The binary is reached through `env!("CARGO_BIN_EXE_sim")`, so this exercises
//! the real artefact: argument parsing, config read, seed resolution, the whole
//! nine-phase tick pipeline, and the run directory it leaves behind — in one
//! process.
//!
//! **What the first three tests now prove, and why they still exist.** Until
//! Phase 3 they parsed the Phase 1 tracer's single stdout line. The binary no
//! longer prints one, so they were PORTED rather than deleted: they are not
//! obsolete in intent, and the different-seed one is the direct ancestor of
//! TICK-10.
//!
//! They are deliberately kept at the COLUMN level. Plan 03-05 asserts the
//! file-byte-level and cross-process claims with the process-testing crate; the
//! overlap is intentional rather than duplication. These three are the cheap,
//! direct, binary-level smoke test that has existed since Phase 1, and reading
//! a named column is what makes a failure here say *which* column stopped
//! depending on the seed — something a byte comparison cannot.
//!
//! The library is still reached through `use sim::…` (CORE-08): the expected
//! row count is read from the shipped configuration rather than written out, so
//! a change to the configured run length cannot leave this file asserting a
//! stale number.

use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

/// A distinct, per-test `--out` directory.
fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sim-tracer-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Invoke the built binary against the shipped config, asserting a clean exit.
fn run(seed: u64, out: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_sim"))
        .args(["--config", CONFIG])
        .args(["--seed", &seed.to_string()])
        .arg("--out")
        .arg(out)
        .output()
        .expect("failed to spawn the sim binary");

    assert!(
        output.status.success(),
        "binary exited {:?}; stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );

    // The run is the artefact; a clean run says nothing on standard output.
    // This is the runtime half of "the Phase 1 tracer line is gone" — a rewrite
    // that had been additive rather than a replacement would fail here.
    assert!(
        output.stdout.is_empty(),
        "expected no standard output, got {:?}",
        String::from_utf8_lossy(&output.stdout),
    );
}

/// The raw bytes of the per-tick series a run left behind.
fn ticks_bytes(out: &Path) -> Vec<u8> {
    std::fs::read(out.join("ticks.csv")).expect("the run left a tick file behind")
}

/// One column of the FIRST DATA ROW, addressed by name out of the header.
///
/// By name, never by position: a column that moved would otherwise silently
/// change what this file claims to be asserting.
fn first_row_column(out: &Path, name: &str) -> String {
    let text = String::from_utf8(ticks_bytes(out)).expect("the tick file is text");
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .expect("the tick file has a header")
        .split(',')
        .collect();
    let at = header
        .iter()
        .position(|column| *column == name)
        .unwrap_or_else(|| panic!("no {name} column in {header:?}"));
    lines
        .next()
        .expect("the tick file has at least one data row")
        .split(',')
        .nth(at)
        .unwrap_or_else(|| panic!("the first data row is narrower than the header"))
        .to_owned()
}

/// The run length the shipped configuration asks for, read through the library.
fn configured_ticks() -> u32 {
    sim::config::load(Path::new(CONFIG))
        .expect("the library loads the same config the binary does")
        .0
        .sim
        .ticks
}

#[test]
fn runs_end_to_end() {
    let out = out_dir("end-to-end");
    run(7, &out);

    let text = String::from_utf8(ticks_bytes(&out)).expect("the tick file is text");
    let lines: Vec<&str> = text.lines().collect();
    let expected = usize::try_from(configured_ticks()).expect("the tick count is bounded") + 1;

    assert_eq!(
        lines.len(),
        expected,
        "one header line plus one row per configured tick"
    );
    assert!(
        lines[0].starts_with("tick,"),
        "the header is missing or the tick column moved: {}",
        lines[0]
    );
    assert!(
        !text.contains('\r'),
        "the line terminator carries no carriage return"
    );
}

#[test]
fn same_seed_is_reproducible() {
    let first = out_dir("repro-a");
    let second = out_dir("repro-b");
    run(7, &first);
    run(7, &second);

    assert_eq!(
        ticks_bytes(&first),
        ticks_bytes(&second),
        "the same seed produced different tick files across two runs",
    );
}

#[test]
fn different_seed_changes_the_activation_digest() {
    let seven = out_dir("seed-7");
    let eight = out_dir("seed-8");
    run(7, &seven);
    run(8, &eight);

    // The draw count is IDENTICAL at both seeds, and that is exactly why a run
    // logging only the count produced byte-identical files at two seeds while
    // appearing to consume the generator. Asserted here so the next reader can
    // see what the digest column is for.
    assert_eq!(
        first_row_column(&seven, "rng_draws"),
        first_row_column(&eight, "rng_draws"),
        "the per-tick draw count is fixed-draw and therefore seed-independent",
    );
    assert_ne!(
        first_row_column(&seven, "activation_digest"),
        first_row_column(&eight, "activation_digest"),
        "the activation digest did not change with the seed — nothing the seed \
         touches reaches a diffed byte",
    );
}

// ---------------------------------------------------------------------------
// CORE-02 / D-10: the release profile cannot silently wrap.
//
// `Money`'s operators route through `checked_add` and so panic regardless of
// profile — but every raw `i64` on the behaviour path (goods units, headcounts,
// tick counters) is unprotected without `[profile.release] overflow-checks`.
// A default release build was verified to wrap `i64::MAX - 1 + 6` silently.
//
// These two cases are the pair. The panicking one observes that overflow is
// detected; the adjacent non-panicking one is what distinguishes "overflow
// detection works" from "all addition panics".
//
// WHAT THIS PAIR DOES NOT PROVE, and why the name says so. Under a plain
// `cargo test` it carries no information about `[profile.release]` at all: the
// `test` profile inherits `dev`, where `overflow-checks` is already on by
// default, so deleting the setting from Cargo.toml leaves both cases green in
// the debug run. Verified by deleting it: the debug suite stayed at 5 passed.
// They are informative only in the `--release` pass, where the `bench` profile
// inherits `release`. The setting ITSELF is asserted where it belongs, as a
// fact about the manifest, by `tests/toolchain.sh` check 4b.
//
// Both operands go through `std::hint::black_box` so the expression is not
// const-evaluated: without it rustc rejects the overflow at compile time and
// the runtime check is never exercised.
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "overflow")]
fn raw_i64_overflow_panics_when_overflow_checks_are_on() {
    let lhs = std::hint::black_box(i64::MAX - 1);
    let rhs = std::hint::black_box(2i64);
    let _ = std::hint::black_box(lhs + rhs);
}

#[test]
fn raw_i64_at_the_maximum_does_not_panic() {
    let lhs = std::hint::black_box(i64::MAX - 1);
    let rhs = std::hint::black_box(1i64);
    assert_eq!(
        lhs + rhs,
        i64::MAX,
        "one step below the edge must not panic"
    );
}

// ---------------------------------------------------------------------------
// The two HELD-OUT overflow sites (01-UAT.md test 4).
//
// The pair above overflows a raw `i64` at ONE site, in the test function's own
// body. `01-VERIFICATION.md` abstained on the backstop truth "`overflow-checks`
// applies to every arithmetic site" for exactly the right reason: one site is
// evidence about that site, not a universal quantifier over all of them.
//
// These two close the gap at the two places where the check could plausibly
// NOT follow the call site, because `-C overflow-checks` is applied when MIR is
// built rather than when it is codegen'd:
//
//   (a) across an `#[inline(always)]` call boundary — the addition is written in
//       one function and executed inside another;
//   (b) inside a generic — the addition is written once, against `T: Add`, and
//       monomorphised at `i64` by the caller.
//
// Both are still same-crate, so what they establish is that the profile setting
// reaches inlined and monomorphised MIR in THIS crate. That is the claim the
// project actually depends on: goods units, headcounts and tick counters are
// raw `i64` and will be added inside small helpers and generic code, not only
// in straight-line function bodies.
//
// The same caveat as the pair above applies unchanged: under a plain
// `cargo test` these carry no information about `[profile.release]`, because the
// `test` profile inherits `dev` where `overflow-checks` is on by default. They
// are informative in the `--release` pass. CI runs both profiles.
// ---------------------------------------------------------------------------

/// (a) The addition lives here; the panic must occur in the caller's frame.
#[inline(always)]
fn add_across_an_inline_boundary(lhs: i64, rhs: i64) -> i64 {
    lhs + rhs
}

/// (b) The addition is written once against `T: Add` and monomorphised at `i64`.
fn add_in_a_generic<T: std::ops::Add<Output = T>>(lhs: T, rhs: T) -> T {
    lhs + rhs
}

#[test]
#[should_panic(expected = "overflow")]
fn raw_i64_overflow_panics_across_an_inline_boundary() {
    let lhs = std::hint::black_box(i64::MAX - 1);
    let rhs = std::hint::black_box(2i64);
    let _ = std::hint::black_box(add_across_an_inline_boundary(lhs, rhs));
}

#[test]
#[should_panic(expected = "overflow")]
fn raw_i64_overflow_panics_inside_a_generic() {
    let lhs = std::hint::black_box(i64::MAX - 1);
    let rhs = std::hint::black_box(2i64);
    let _ = std::hint::black_box(add_in_a_generic(lhs, rhs));
}

#[test]
fn the_held_out_sites_do_not_panic_one_step_below_the_edge() {
    // The negative half of each pair: distinguishes "overflow is detected at
    // these sites" from "these sites panic on any addition".
    let lhs = std::hint::black_box(i64::MAX - 1);
    let rhs = std::hint::black_box(1i64);
    assert_eq!(add_across_an_inline_boundary(lhs, rhs), i64::MAX);
    assert_eq!(add_in_a_generic(lhs, rhs), i64::MAX);
}
