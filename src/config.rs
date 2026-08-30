//! The parameter file is the whole input.
//!
//! Three rules make this reproducible (CORE-10):
//!
//! 1. `deny_unknown_fields` on **every** struct, not just the root — the root
//!    attribute alone catches a stray table but not a stray key inside a
//!    nested one.
//! 2. No field may default. There is no serde `default` attribute anywhere
//!    under `src/`, and no field in this module carries an optional type:
//!    research verified that an optional field defaults to absent with **no
//!    attribute to grep for**, which is exactly the hidden hardcoded parameter
//!    CORE-10 forbids. The attribute grep is a cheap complement to
//!    `tests/config_strict.rs::every_key_is_required`, never a substitute.
//! 3. The config hash is taken over the **raw file bytes**, never over the
//!    parsed `Params`. A hash of a Rust value is not stable across compiler
//!    releases, and hashing bytes means a comment change (which carries a
//!    parameter's source grade) also changes the hash — which is correct.
//!
//! Probabilities and ratios enter as parts-per-million integers, matching the
//! sampler API in `src/rng.rs`, so every threshold parameter stays in the
//! integer domain.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::money::{Money, MoneyOverflow};

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
    /// An operator-supplied money amount that the money domain cannot carry.
    ///
    /// Threat T-1-03: an absurd `total_money_cents` must surface as a named
    /// configuration error, not abort the process on a panicking operator.
    #[error("config file `{path}` supplies a money amount outside the representable range")]
    MoneyRange {
        path: PathBuf,
        #[source]
        source: MoneyOverflow,
    },
}

/// The whole parameter tree. Six tables, every key required.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub sim: Sim,
    pub money: MoneySection,
    pub household: Household,
    pub firm: Firm,
    pub bankruptcy: Bankruptcy,
    pub ownership: Ownership,
}

/// Run shape: how long, how many agents, and the seed of record.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sim {
    /// Ticks to run. One tick is one day.
    pub ticks: u32,
    /// The seed of record. `--seed` overrides it; the override is what is logged.
    pub seed: u64,
    pub households: u32,
    pub firms: u32,
    /// Days per accounting month — the cadence of the monthly decisions.
    pub month_days: u32,
}

/// The money stock. Integer cents — a float here would end conservation.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoneySection {
    /// The fixed pile. Conserved to the cent for the whole run.
    pub total_money_cents: i64,
}

/// Household behaviour: consumption, supplier choice and job search.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Household {
    /// The exponent of `(m / mean_price)`, in parts per million.
    pub consumption_exponent_ppm: u32,
    /// Size of the persistent preferred-supplier list.
    pub supplier_list_size: u32,
    /// Price advantage a rival must beat to displace a supplier, in ppm.
    pub supplier_switch_threshold_ppm: u32,
    /// Probability of a price-motivated supplier search, in ppm.
    pub price_search_prob_ppm: u32,
    /// Probability of a rationing-motivated supplier search, in ppm.
    pub rationing_search_prob_ppm: u32,
    pub firms_sampled_consumer: u32,
    pub firms_sampled_unemployed: u32,
    pub firms_sampled_employed: u32,
    /// Probability an employed household searches anyway, in ppm.
    pub employed_search_prob_ppm: u32,
    /// Monthly decay applied to an unemployed reservation wage, in ppm.
    pub reservation_wage_decay_ppm: u32,
    pub reservation_wage_floor_cents: i64,
    pub initial_liquidity_cents: i64,
    pub initial_reservation_wage_cents: i64,
}

/// Firm behaviour: production, pricing, wage setting and dividends.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Firm {
    /// Goods produced per worker per day.
    pub productivity_units_per_worker_day: u32,
    /// Weight on the newest observation in the demand expectation, in ppm.
    pub demand_smoothing_ppm: u32,
    /// Maximum single price step, in ppm.
    pub price_step_bound_ppm: u32,
    /// Probability the firm leaves its price alone, in ppm.
    pub price_inaction_prob_ppm: u32,
    /// Inventory band, as a fraction of expected demand, in ppm.
    pub inventory_floor_ppm: u32,
    pub inventory_ceiling_ppm: u32,
    /// Price band, as a multiple of marginal cost, in ppm.
    pub price_floor_over_mc_ppm: u32,
    pub price_ceiling_over_mc_ppm: u32,
    /// Maximum single wage step, in ppm.
    pub wage_step_bound_ppm: u32,
    /// Consecutive fully-staffed cycles before a wage cut.
    pub full_staff_cycles_before_wage_cut: u32,
    /// Cash retained against payroll before a dividend, in ppm.
    pub dividend_buffer_ppm: u32,
    /// Fraction of demand a firm plans to satisfy, in ppm.
    pub demand_satisfaction_ppm: u32,
    pub wage_floor_cents: i64,
    pub initial_price_cents: i64,
    pub initial_wage_cents: i64,
    pub initial_inventory_units: i64,
    // The ONE floating-point field in the whole configuration, and the one
    // crossing D-11 permits outside `src/numeric.rs`. Do not "tidy" it into an
    // integer: CAL-01 requires it strictly positive and D-13 requires it logged
    // at full round-trip precision rather than truncated. The parser refuses to
    // coerce, so the shipped value must carry a decimal point.
    pub initial_expected_demand: f64,
    pub initial_liquidity_cents: i64,
}

/// Exit and replacement of insolvent firms.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bankruptcy {
    /// Entrant size against the trimmed mean of incumbents, in ppm.
    pub entrant_size_ratio_ppm: u32,
    /// Entrant price against the market average, in ppm.
    pub entrant_price_ratio_ppm: u32,
    /// Incumbents trimmed from each tail before taking that mean.
    pub incumbent_trim_per_tail: u32,
}

/// Who owns the firms, and therefore who receives dividends.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ownership {
    pub firms_per_owner: u32,
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

    // The money stock crosses into the money domain here, through the named
    // `Result`-returning API rather than the panicking operator (threat
    // T-1-03). Requiring the stock to survive being added to itself is the
    // headroom the conservation audit's intermediate sums need; an absurd
    // amount is reported as `MoneyRange` instead of aborting the process.
    let stock = Money::from_cents(params.money.total_money_cents);
    stock
        .checked_add(stock)
        .map_err(|source| ConfigError::MoneyRange {
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
        assert!(
            error.contains("houseolds"),
            "the misspelling is not named: {error}"
        );
    }

    #[test]
    fn an_undeclared_table_is_rejected() {
        let error = parse_error(&format!("{FULL}\n[oops]\nx = 1\n"));
        assert!(error.contains("unknown field"), "{error}");
        assert!(
            error.contains("oops"),
            "the stray table is not named: {error}"
        );
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
        assert_eq!(first, second);
    }

    #[test]
    fn the_hash_is_stable_and_sensitive_to_one_comment_character() {
        let once = config_hash(FULL.as_bytes());
        assert_eq!(once.len(), 64, "digest is not 64 hex characters: {once}");
        assert!(
            once.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
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
