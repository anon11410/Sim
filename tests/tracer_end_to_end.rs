//! End-to-end check of the BUILT BINARY, not of each layer in isolation.
//!
//! The binary is reached through `env!("CARGO_BIN_EXE_sim")`, so this exercises
//! the real artefact: argument parsing, config read, hash, seed resolution,
//! sub-stream draw and money construction, in one process.
//!
//! The library is reached through `use sim::…` (CORE-08), which is what lets
//! case (a) recompute the binary's draw independently and compare.

use std::path::{Path, PathBuf};
use std::process::Command;

use sim::config::Params;
use sim::money::Money;
use sim::rng::{Purpose, Rngs};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

/// A distinct, per-test `--out` directory.
fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sim-tracer-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Invoke the built binary and return its stdout, asserting a clean exit.
fn run(seed: u64, out: &Path) -> String {
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

    String::from_utf8(output.stdout).expect("stdout was not valid UTF-8")
}

/// Pull `key=value` out of the single tracer line.
fn field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("no `{key}=` field in tracer line: {line}"))
}

#[test]
fn runs_end_to_end() {
    let stdout = run(7, &out_dir("end-to-end"));

    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "expected exactly one line, got {stdout:?}");
    let line = lines[0];
    assert!(line.starts_with("tracer "), "unexpected prefix: {line}");

    // The effective seed is the override, not the config's own seed.
    assert_eq!(field(line, "effective_seed"), "7");

    let hash = field(line, "config_sha256");
    assert_eq!(hash.len(), 64, "digest is not 64 hex characters: {hash}");
    assert!(
        hash.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "digest is not lowercase hex: {hash}",
    );

    // Recompute the binary's work through the library surface. This is the
    // half that proves `src/main.rs` holds no simulation logic: every value it
    // printed is reachable from `tests/` through `use sim::…`.
    let (params, library_hash): (Params, String) =
        sim::config::load(Path::new(CONFIG)).expect("library failed to load the same config");
    assert_eq!(hash, library_hash, "binary and library disagree on the config hash");

    let mut probe = Rngs::new(7).stream(0, 0, Purpose::TracerProbe);
    let draw = probe.below(1_000_000);
    assert_eq!(probe.draws(), 1, "below() must take exactly one draw");
    assert_eq!(field(line, "draw"), draw.to_string(), "binary and library disagree on the draw");

    let money = Money::from_cents(params.money.total_money_cents) + Money::ZERO;
    assert_eq!(field(line, "money_cents"), money.cents().to_string());
}

#[test]
fn same_seed_is_reproducible() {
    let first = run(7, &out_dir("repro-a"));
    let second = run(7, &out_dir("repro-b"));

    assert_eq!(
        first, second,
        "the same seed produced different output across two runs",
    );
}

#[test]
fn different_seed_changes_the_draw() {
    let seven = run(7, &out_dir("seed-7"));
    let eight = run(8, &out_dir("seed-8"));

    assert_ne!(
        field(&seven, "draw"),
        field(&eight, "draw"),
        "the draw did not change with the seed — the RNG may be constant",
    );
}
