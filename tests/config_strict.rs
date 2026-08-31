//! CORE-10 at the artefact level: the shipped config is the whole input, and
//! not one of its keys can quietly default.
//!
//! The load-bearing test here is `every_key_is_required`. ROADMAP criterion 3
//! names a grep for the serde `default` attribute, and research verified that
//! grep is necessary but **not sufficient**: an optional field type defaults to
//! absent with no attribute to find. The only check that actually proves the
//! requirement is to delete every leaf key in turn and assert each deletion is
//! rejected by name. The source assertions at the bottom are the cheap
//! complement, and they live inside the test binary rather than in a shell
//! script so they cannot be skipped by running the suite without the linter.
//!
//! `every_key_is_required` has one blind spot, and
//! `the_schema_and_the_shipped_config_name_the_same_leaves` is what covers it:
//! deleting keys from the shipped file can only ever see fields that are
//! ALREADY in the shipped file. A field added to `Params` with a default and
//! not added to `baseline.toml` is invisible to it. The two tests close the
//! gap from opposite directions.

use std::fs;
use std::path::{Path, PathBuf};

use sim::config::{self, Params};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

/// The shipped parameter file, as text.
fn shipped() -> String {
    fs::read_to_string(CONFIG).expect("the shipped config must be readable")
}

/// Every leaf key in `value`, as a full dotted path, in deterministic order.
///
/// A leaf is any value that is not a table, so this walks nested tables without
/// caring how deep the schema happens to be today.
fn leaf_paths(value: &toml::Value, prefix: &[String], out: &mut Vec<Vec<String>>) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let mut path = prefix.to_vec();
                path.push(key.clone());
                leaf_paths(child, &path, out);
            }
        }
        _ => out.push(prefix.to_vec()),
    }
}

/// Delete the leaf at `path` from `value`.
fn remove_at(value: &mut toml::Value, path: &[String]) {
    let (leaf, parents) = path.split_last().expect("a leaf path is never empty");
    let mut cursor = value;
    for step in parents {
        cursor = cursor
            .as_table_mut()
            .expect("a path step must address a table")
            .get_mut(step)
            .expect("a path step must exist");
    }
    cursor
        .as_table_mut()
        .expect("a leaf's parent must be a table")
        .remove(leaf)
        .expect("the leaf must exist");
}

/// The same document with its tables emitted in reverse order.
///
/// Built textually rather than by round-tripping through `toml::Value`, whose
/// table map re-sorts keys and would undo the reordering.
fn tables_reversed(raw: &str) -> String {
    let mut preamble = String::new();
    let mut blocks: Vec<String> = Vec::new();

    for line in raw.lines() {
        if line.starts_with('[') {
            blocks.push(String::new());
        }
        match blocks.last_mut() {
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
            None => {
                preamble.push_str(line);
                preamble.push('\n');
            }
        }
    }

    blocks.reverse();
    let mut out = preamble;
    for block in blocks {
        out.push_str(&block);
    }
    out
}

/// The error text of a failed parse, or a panic naming what wrongly succeeded.
fn parse_error(document: &str) -> String {
    match toml::from_str::<Params>(document) {
        Ok(params) => panic!("expected a parse failure, got {params:?}"),
        Err(error) => error.to_string(),
    }
}

/// Every `.rs` file under `dir`, recursively, in sorted path order.
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

// --- The exhaustive proof -------------------------------------------------

#[test]
fn every_key_is_required() {
    let raw = shipped();
    let document: toml::Value =
        toml::from_str(&raw).expect("the shipped config must parse as TOML");

    let mut paths = Vec::new();
    leaf_paths(&document, &[], &mut paths);
    assert!(
        paths.len() >= 40,
        "only {} leaf keys found — the schema cannot have been widened",
        paths.len()
    );

    for path in &paths {
        let dotted = path.join(".");
        let leaf = path.last().expect("a leaf path is never empty");

        let mut mutated = document.clone();
        remove_at(&mut mutated, path);
        let text = toml::to_string(&mutated).expect("a pruned document must re-serialise");

        let error = match toml::from_str::<Params>(&text) {
            Ok(params) => {
                panic!("`{dotted}` is not required — a hidden default supplied it: {params:?}")
            }
            Err(error) => error.to_string(),
        };

        assert!(
            error.contains(&format!("missing field `{leaf}`")),
            "deleting `{dotted}` produced an error that does not name it: {error}"
        );
    }
}

// --- Strictness at the parse boundary -------------------------------------

#[test]
fn unknown_key_inside_a_table_is_rejected() {
    // Inside the existing `[sim]` table: the root's own `deny_unknown_fields`
    // must not be what catches this, or nested strictness is untested.
    let error = parse_error(&shipped().replace("[sim]\n", "[sim]\nhouseolds = 1\n"));
    assert!(error.contains("unknown field"), "{error}");
    assert!(
        error.contains("houseolds"),
        "the misspelled key is not named: {error}"
    );
}

#[test]
fn unknown_table_is_rejected() {
    let error = parse_error(&format!("{}\n[oops]\nx = 1\n", shipped()));
    assert!(error.contains("unknown field"), "{error}");
    assert!(
        error.contains("oops"),
        "the stray table is not named: {error}"
    );
}

#[test]
fn empty_config_is_rejected() {
    let error = parse_error("");
    assert!(
        error.contains("missing field"),
        "an empty file must not produce a fully-defaulted parameter set: {error}"
    );
}

#[test]
fn removed_value_is_rejected() {
    let error = parse_error(&shipped().replace("households = 200", "households ="));
    assert!(
        error.contains("TOML parse error"),
        "a key with no value must fail before the deserializer runs: {error}"
    );
}

#[test]
fn float_where_int_is_not_coerced() {
    let error = parse_error(&shipped().replace("households = 200", "households = 250.0"));
    assert!(error.contains("invalid type"), "{error}");
    assert!(error.contains("floating point"), "{error}");
}

#[test]
fn string_where_int_is_not_coerced() {
    let error = parse_error(&shipped().replace("households = 200", "households = \"42\""));
    assert!(error.contains("invalid type"), "{error}");
    assert!(error.contains("string"), "{error}");
}

// --- Run identity ---------------------------------------------------------

#[test]
fn key_order_does_not_change_params_but_does_change_the_hash() {
    let raw = shipped();
    let reordered = tables_reversed(&raw);
    assert_ne!(raw, reordered, "the reordering was a no-op");

    let straight: Params = toml::from_str(&raw).expect("the shipped config must parse");
    let reversed: Params = toml::from_str(&reordered).expect("the reordered config must parse");
    assert_eq!(
        straight, reversed,
        "table order changed the parsed parameters"
    );

    // The hash is over bytes, not over the parsed value. That is deliberate: a
    // comment change is a hash change, and the comments carry the source grades
    // CORE-11 makes load-bearing.
    assert_ne!(
        config::config_hash(raw.as_bytes()),
        config::config_hash(reordered.as_bytes()),
        "reordering the file left the run's identifying hash unchanged"
    );
}

#[test]
fn hash_is_stable_across_repeated_computation() {
    let bytes = shipped().into_bytes();
    let first = config::config_hash(&bytes);

    assert_eq!(first.len(), 64, "digest is not 64 hex characters: {first}");
    assert!(
        first
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "digest is not lowercase hex: {first}"
    );

    for _ in 0..8 {
        assert_eq!(first, config::config_hash(&bytes), "the hash is not stable");
    }
}

// --- Domain validation is reached through `load`, not just available ------
//
// CR-02: `load` used to read, hash and parse, plus one money-headroom check,
// and accept every other parameter exactly as written. A `validate` that exists
// but is never called is worth nothing, so this exercises the public entry
// point against a real file rather than calling `Params::validate` directly —
// the unit tests in `src/config.rs` cover the individual bounds.

/// Write `text` to a uniquely-named temp file and return its path.
fn temp_config(name: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "sim-config-strict-{}-{name}.toml",
        std::process::id()
    ));
    fs::write(&path, text).expect("the temp config must be writable");
    path
}

#[test]
fn load_accepts_the_shipped_config() {
    config::load(Path::new(CONFIG)).expect("the shipped config must load");
}

#[test]
fn load_rejects_out_of_domain_parameters_at_run_start() {
    // Each of these loaded cleanly before, and the binary printed a tracer line
    // and exited 0 on all four at once.
    let cases: [(&str, &str, &str); 5] = [
        ("zero-households", "households = 200", "households = 0"),
        ("huge-ticks", "ticks = 3650", "ticks = 99999999"),
        (
            "negative-money",
            "total_money_cents = 2000000",
            "total_money_cents = -2000000",
        ),
        (
            "nan-demand",
            "initial_expected_demand = 330.0",
            "initial_expected_demand = nan",
        ),
        ("too-many-firms", "firms = 20", "firms = 70000"),
    ];

    for (name, from, to) in cases {
        let mutated = shipped().replace(from, to);
        assert_ne!(
            mutated,
            shipped(),
            "the substitution `{from}` matched nothing"
        );
        let path = temp_config(name, &mutated);

        let error = match config::load(&path) {
            Ok((params, _)) => panic!("`{to}` was accepted by load: {params:?}"),
            Err(error) => error,
        };
        assert!(
            matches!(error, config::ConfigError::Domain { .. }),
            "`{to}` was rejected, but not as a domain violation: {error:?}"
        );

        fs::remove_file(&path).expect("the temp config must be removable");
    }
}

// --- The two source assertions, inside the test binary --------------------

#[test]
fn no_serde_defaults_anywhere_in_src() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let sources = rust_sources(&src);
    assert!(
        !sources.is_empty(),
        "no sources found under {}",
        src.display()
    );

    for path in &sources {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

        // Line-based and attribute-order agnostic. A whole-file
        // `contains("serde(default")` matched `#[serde(default)]` and
        // `#[serde(default = "…")]` and NOTHING ELSE: every one of
        //     #[serde(rename = "x", default)]
        //     #[serde(skip_serializing_if = "…", default)]
        // is a real serde default that walked straight through it.
        //
        // Whitespace is stripped so a multi-line attribute is judged on its
        // own line rather than on an accident of formatting.
        for (number, line) in text.lines().enumerate() {
            let stripped: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            let is_default = stripped.contains("(default")
                || stripped.contains(",default")
                || stripped == "default";
            assert!(
                !(stripped.contains("serde(") && is_default),
                "{}:{} carries a serde default — a hidden hardcoded parameter: {}",
                path.display(),
                number + 1,
                line.trim()
            );
        }
    }
}

/// The schema and the shipped file must name the SAME leaf keys.
///
/// `every_key_is_required` deletes each leaf of `config/baseline.toml` in turn
/// and asserts the deletion is rejected — so it can only ever see fields that
/// are already in the shipped file. A field added to `Params` with a default
/// and *not* added to `baseline.toml` satisfies both that test and the
/// attribute grep above, and is exactly the hidden hardcoded parameter CORE-10
/// forbids.
///
/// Round-tripping a parsed `Params` back through TOML closes it from the other
/// direction: the serialised schema and the shipped file are compared as leaf
/// sets, so a schema field with no config key fails here, and a config key with
/// no schema field fails at the parse (`deny_unknown_fields`).
#[test]
fn the_schema_and_the_shipped_config_name_the_same_leaves() {
    let raw = shipped();

    let shipped_document: toml::Value =
        toml::from_str(&raw).expect("the shipped config must parse as TOML");
    let mut shipped_leaves = Vec::new();
    leaf_paths(&shipped_document, &[], &mut shipped_leaves);
    shipped_leaves.sort();

    let params: Params = toml::from_str(&raw).expect("the shipped config must parse as Params");
    let serialised = toml::to_string(&params).expect("Params must re-serialise");
    let schema_document: toml::Value =
        toml::from_str(&serialised).expect("the re-serialised schema must parse");
    let mut schema_leaves = Vec::new();
    leaf_paths(&schema_document, &[], &mut schema_leaves);
    schema_leaves.sort();

    assert_eq!(
        schema_leaves, shipped_leaves,
        "the schema and the shipped config disagree on their leaf keys — a field \
         in `Params` with no key in baseline.toml is a hidden default that \
         `every_key_is_required` cannot see"
    );
}

#[test]
fn no_optional_fields_in_the_config_schema() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config.rs");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    for form in ["Option<", "Option <", "option::Option"] {
        assert!(
            !text.contains(form),
            "{} names `{form}` — an optional field is a default with no attribute to grep for",
            path.display()
        );
    }
}
