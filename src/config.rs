//! The parameter file is the whole input.
//!
//! Two rules make this reproducible (CORE-10):
//!
//! 1. `deny_unknown_fields` on **every** struct, not just the root — the root
//!    attribute alone catches a stray table but not a stray key inside a
//!    nested one.
//! 2. The config hash is taken over the **raw file bytes**, never over the
//!    parsed `Params`. A hash of a Rust value is not stable across compiler
//!    releases, and hashing bytes means a comment change (which carries a
//!    parameter's source grade) also changes the hash — which is correct.
//!
//! No field in this file is `Option<T>` and none carries a serde `default`:
//! either is an invisible input that would not appear in the committed config.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Everything that can go wrong turning a path into a `Params`.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config file `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("config file `{path}` is not valid UTF-8")]
    Utf8 {
        path: PathBuf,
        #[source]
        source: std::str::Utf8Error,
    },
    #[error("could not parse config file `{path}`")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// The whole parameter tree. Widened to the full parameter set by plan 01-06.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub sim: Sim,
    pub money: MoneySection,
}

/// Run shape: how long, how many agents, and the seed of record.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sim {
    pub ticks: u32,
    pub seed: u64,
    pub households: u32,
    pub firms: u32,
}

/// The money stock. Integer cents — a float here would end conservation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoneySection {
    pub total_money_cents: i64,
}

/// Read `path` once as raw bytes, hash those bytes, and parse those same bytes.
///
/// Returns the parsed parameters and the lowercase hex SHA-256 of the file as
/// it was read — the two are guaranteed to describe the same byte sequence
/// because the file is read exactly once.
pub fn load(path: &Path) -> Result<(Params, String), ConfigError> {
    let bytes = std::fs::read(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let hash = config_hash(&bytes);

    let text = std::str::from_utf8(&bytes).map_err(|source| ConfigError::Utf8 {
        path: path.to_path_buf(),
        source,
    })?;

    let params: Params = toml::from_str(text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    Ok((params, hash))
}

/// Lowercase hex SHA-256 of `bytes`.
///
/// Hex is built byte by byte rather than through a `LowerHex` impl on the
/// digest type, so the idiom survives a future `sha2` major bump.
pub fn config_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest.iter() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A complete, schema-valid document, embedded rather than read from
    /// `config/baseline.toml` so this module pins the *schema* and cannot be
    /// broken by a later edit to the shipped parameter values. The shipped file
    /// is exercised end to end by `tests/config_strict.rs` and the tracer test.
    const FULL: &str = "\
[sim]
ticks = 3650
seed = 42
households = 200
firms = 20
month_days = 21

[money]
total_money_cents = 2000000

[household]
consumption_exponent_ppm = 900000
supplier_list_size = 7
supplier_switch_threshold_ppm = 10000
price_search_prob_ppm = 250000
rationing_search_prob_ppm = 250000
firms_sampled_consumer = 5
firms_sampled_unemployed = 5
firms_sampled_employed = 1
employed_search_prob_ppm = 100000
reservation_wage_decay_ppm = 900000
reservation_wage_floor_cents = 1000
initial_liquidity_cents = 5000
initial_reservation_wage_cents = 6300

[firm]
productivity_units_per_worker_day = 3
demand_smoothing_ppm = 250000
price_step_bound_ppm = 20000
price_inaction_prob_ppm = 750000
inventory_floor_ppm = 250000
inventory_ceiling_ppm = 1000000
price_floor_over_mc_ppm = 1025000
price_ceiling_over_mc_ppm = 1150000
wage_step_bound_ppm = 19000
full_staff_cycles_before_wage_cut = 24
dividend_buffer_ppm = 100000
demand_satisfaction_ppm = 950000
wage_floor_cents = 1000
initial_price_cents = 105
initial_wage_cents = 6300
initial_inventory_units = 165
initial_expected_demand = 330.0
initial_liquidity_cents = 50000

[bankruptcy]
entrant_size_ratio_ppm = 800000
entrant_price_ratio_ppm = 1260000
incumbent_trim_per_tail = 1

[ownership]
firms_per_owner = 1
";

    /// The error text of a failed parse, or a panic naming the value that
    /// unexpectedly parsed.
    fn parse_error(document: &str) -> String {
        match toml::from_str::<Params>(document) {
            Ok(params) => panic!("expected a parse failure, got {params:?}"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn the_full_document_parses() {
        toml::from_str::<Params>(FULL).expect("the embedded full document must parse");
    }

    #[test]
    fn an_empty_document_is_rejected_by_name() {
        let error = parse_error("");
        assert!(
            error.contains("missing field"),
            "an empty file must not produce a fully-defaulted parameter set: {error}"
        );
    }

    #[test]
    fn a_misspelled_key_inside_a_table_is_rejected() {
        // Inside `[sim]`, not as a second `[sim]` table: the root's
        // `deny_unknown_fields` would catch the latter for the wrong reason.
        let error = parse_error(&FULL.replace("[sim]\n", "[sim]\nhouseolds = 1\n"));
        assert!(error.contains("unknown field"), "{error}");
        assert!(error.contains("houseolds"), "the misspelling is not named: {error}");
    }

    #[test]
    fn an_undeclared_table_is_rejected() {
        let error = parse_error(&format!("{FULL}\n[oops]\nx = 1\n"));
        assert!(error.contains("unknown field"), "{error}");
        assert!(error.contains("oops"), "the stray table is not named: {error}");
    }

    #[test]
    fn a_decimal_is_not_coerced_into_an_integer_key() {
        let error = parse_error(&FULL.replace("households = 200", "households = 250.0"));
        assert!(error.contains("invalid type"), "{error}");
        assert!(error.contains("floating point"), "{error}");
    }

    #[test]
    fn a_quoted_number_is_not_coerced_into_an_integer_key() {
        let error = parse_error(&FULL.replace("households = 200", "households = \"42\""));
        assert!(error.contains("invalid type"), "{error}");
        assert!(error.contains("string"), "{error}");
    }

    #[test]
    fn a_key_with_no_value_is_a_parse_error() {
        let error = parse_error(&FULL.replace("households = 200", "households ="));
        assert!(error.contains("TOML parse error"), "{error}");
    }

    #[test]
    fn parsing_the_same_document_twice_is_equal() {
        let first = toml::from_str::<Params>(FULL).expect("first parse");
        let second = toml::from_str::<Params>(FULL).expect("second parse");
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    #[test]
    fn the_hash_is_stable_and_sensitive_to_one_comment_character() {
        let once = config_hash(FULL.as_bytes());
        assert_eq!(once.len(), 64, "digest is not 64 hex characters: {once}");
        assert!(
            once.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "digest is not lowercase hex: {once}"
        );
        assert_eq!(once, config_hash(FULL.as_bytes()), "the hash is not stable");

        let commented = format!("{FULL}# one more character\n");
        assert_ne!(
            once,
            config_hash(commented.as_bytes()),
            "a comment change must change the hash — the comments carry the source grades"
        );
    }
}
