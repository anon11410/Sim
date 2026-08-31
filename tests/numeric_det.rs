//! D-11 / D-12 at the library surface: the fractional power is bit-identical,
//! the bit count is load-bearing, the crossing rounds and saturates as
//! specified — and the float domain really is one module wide.
//!
//! The last test is the module-level half of the float ban. Plan `01-07`'s
//! clippy wall enforces the method-level half (which *methods* may be called);
//! this file enforces which *files* may name the type at all. Neither is
//! sufficient alone: the lint catches the accidental call, this catches the
//! spread.

use std::fs;
use std::path::{Path, PathBuf};

use sim::numeric::{POW_FRAC_BITS, demand_to_units, pow_frac, pow_frac_det};

/// A deterministic sweep of the model's plausible input range.
fn swept_inputs() -> Vec<f64> {
    (1..=20_000).map(|step| step as f64 * 0.01).collect()
}

#[test]
fn pow_frac_is_bit_identical_across_many_invocations() {
    // Compare raw bit patterns, never values: a comparison on values could
    // hide a difference that a later serialisation would expose.
    let first = pow_frac(1.5, 0.9).to_bits();
    for _ in 0..100_000 {
        assert_eq!(pow_frac(1.5, 0.9).to_bits(), first);
    }
}

#[test]
fn pow_frac_matches_repeated_square_roots_at_negative_powers_of_two() {
    for x in [0.01f64, 1.0, 2.0, 199.99] {
        assert_eq!(pow_frac(x, 0.5).to_bits(), x.sqrt().to_bits(), "x = {x}");
        assert_eq!(
            pow_frac(x, 0.25).to_bits(),
            x.sqrt().sqrt().to_bits(),
            "x = {x}"
        );
    }
}

#[test]
fn bit_count_is_load_bearing() {
    let swept = swept_inputs();

    let differs = swept
        .iter()
        .any(|&x| pow_frac_det(x, 0.9, 20) != pow_frac_det(x, 0.9, 40));
    assert!(
        differs,
        "20 and 40 bits agree everywhere on the swept range"
    );

    for &x in &swept {
        let coarse = pow_frac_det(x, 0.9, 40);
        let fine = pow_frac_det(x, 0.9, 52);
        let relative = ((coarse - fine) / fine).abs();
        assert!(relative < 1e-9, "relative difference {relative} at x = {x}");
    }

    // The committed constant is what the behaviour path actually uses.
    for &x in &swept {
        assert_eq!(
            pow_frac(x, 0.9).to_bits(),
            pow_frac_det(x, 0.9, POW_FRAC_BITS).to_bits()
        );
    }
}

#[test]
fn crossing_rounds_half_away_from_zero_and_saturates() {
    assert_eq!(demand_to_units(2.5), 3);
    assert_eq!(demand_to_units(-2.5), -3);
    assert_eq!(demand_to_units(0.0), 0);
    assert_eq!(demand_to_units(1e30), i64::MAX);
    assert_eq!(demand_to_units(-1e30), i64::MIN);
}

// --- The module-level half of the float ban -------------------------------

/// The floating-point type names. Every one of these is banned outside the
/// allowlisted files, not just the one the model happens to use.
const FLOAT_TYPE_NAMES: [&str; 4] = ["f16", "f32", "f64", "f128"];

/// Files permitted to name a floating-point type.
///
/// `src/numeric.rs` owns the float domain. `src/config.rs` is allowed the one
/// restricted field D-11 permits, and nothing else — the second assertion
/// below pins that down line by line.
const FLOAT_ALLOWLIST: [&str; 2] = ["numeric.rs", "config.rs"];

/// Every `.rs` file under `dir`, recursively, in sorted path order so that this
/// test's own behaviour does not depend on directory-read order.
fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("cannot read a directory entry").path())
        .collect();
    entries.sort();

    let mut sources = Vec::new();
    for path in entries {
        if path.is_dir() {
            sources.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            sources.push(path);
        }
    }
    sources
}

/// `line` with any `//` line comment removed.
///
/// Used only by the float-LITERAL check. A float literal inside a comment is
/// prose, not arithmetic — `src/rng.rs` legitimately names the vendored crate
/// version "0.10.2" in its module docs — whereas a float TYPE spelled anywhere,
/// including a comment, is a signal worth keeping strict. Hence the asymmetry:
/// the type check below reads whole lines, this one does not.
///
/// A `//` inside a string literal would be stripped too. No such line exists
/// under `src/`, and the failure mode is a missed detection in a check that is
/// itself a backstop, not a false accusation.
fn without_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(at) => &line[..at],
        None => line,
    }
}

/// Whether `line` contains a floating-point literal: a digit, a dot, a digit.
///
/// Rust does not require naming the type to work in floats. Both of
///
/// ```text
/// let smoothing = 0.25;                                   // inferred f64
/// let expected = observed * smoothing + expected * (1.0 - smoothing);
/// ```
///
/// call no banned method, so clippy is silent, and name no type, so
/// `names_a_float_type` is silent — yet float arithmetic creeping into a
/// behaviour module is the single most important thing this pair is supposed to
/// catch. This closes that.
///
/// Deliberately does not fire on `1..10` (the neighbour of each dot is another
/// dot), `x.0` or `Self::CONST` (no digit on one side).
fn names_a_float_literal(line: &str) -> bool {
    let code = without_line_comment(line);
    let b = code.as_bytes();
    (1..b.len().saturating_sub(1))
        .any(|i| b[i] == b'.' && b[i - 1].is_ascii_digit() && b[i + 1].is_ascii_digit())
}

/// Whether `line` names a floating-point type as a whole word. A substring
/// match alone would fire on a hex literal or a longer identifier.
fn names_a_float_type(line: &str) -> bool {
    let bytes = line.as_bytes();
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';

    FLOAT_TYPE_NAMES.iter().any(|name| {
        line.match_indices(name).any(|(at, _)| {
            let before_ok = at == 0 || !is_ident(bytes[at - 1]);
            let after = at + name.len();
            let after_ok = after >= bytes.len() || !is_ident(bytes[after]);
            before_ok && after_ok
        })
    })
}

#[test]
fn confinement_of_the_float_domain() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let sources = rust_sources(&src);
    assert!(
        !sources.is_empty(),
        "no sources found under {}",
        src.display()
    );

    for path in &sources {
        // Matched on the path RELATIVE TO `src/`, not on the bare file name. A
        // future `src/market/numeric.rs` would otherwise be allowlisted by
        // coincidence of basename rather than by being the module that owns the
        // float domain.
        let name = path
            .strip_prefix(&src)
            .expect("every source is under src/")
            .to_str()
            .expect("a source path is valid UTF-8")
            .to_string();
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

        if !FLOAT_ALLOWLIST.contains(&name.as_str()) {
            for (number, line) in text.lines().enumerate() {
                assert!(
                    !names_a_float_type(line),
                    "{}:{} names a floating-point type; only {:?} may \
                     (src/numeric.rs owns the float domain)",
                    path.display(),
                    number + 1,
                    FLOAT_ALLOWLIST
                );
                // The lint catches the accidental CALL and the check above
                // catches the named TYPE. Neither sees untyped float
                // arithmetic, which is the form the spread actually takes.
                assert!(
                    !names_a_float_literal(line),
                    "{}:{} contains a floating-point literal; only {:?} may \
                     (an inferred f64 needs no type name and calls no banned \
                     method, so it is invisible to both other guards): {}",
                    path.display(),
                    number + 1,
                    FLOAT_ALLOWLIST,
                    line.trim()
                );
            }
        }

        // The config module's allowance is narrower than the file: only the one
        // restricted demand field D-11 permits. A config module naming no float
        // at all also satisfies this, so the test does not depend on the config
        // loader having landed yet.
        if name == "config.rs" {
            for (number, line) in text.lines().enumerate() {
                if names_a_float_type(line) {
                    assert!(
                        line.contains("expected_demand"),
                        "{}:{} names a floating-point type outside the one \
                         restricted field: {}",
                        path.display(),
                        number + 1,
                        line.trim()
                    );
                }
            }
        }
    }
}
