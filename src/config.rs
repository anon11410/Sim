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
use crate::numeric::PPM_SCALE;
use crate::rng::{AGENT_BITS, TICK_BITS};

/// The largest tick the sub-stream key's tick field can carry.
const MAX_TICK: u64 = (1u64 << TICK_BITS) - 1;

/// The largest agent index the sub-stream key's agent field can carry.
const MAX_AGENT: u64 = (1u64 << AGENT_BITS) - 1;

/// The largest firm count `FirmSlot` — a `u16` — can address without aliasing.
const MAX_FIRMS: u64 = u16::MAX as u64;

/// A parameter that parsed but lies outside the domain its consumer requires.
///
/// Separate from the parse errors because it is a different kind of wrong: the
/// file is well-formed TOML matching the schema, and the value is still one the
/// model cannot run on.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("`{key}` {why}")]
pub struct DomainViolation {
    /// The dotted config path, e.g. `sim.ticks`.
    pub key: &'static str,
    /// What the domain is, and what goes wrong outside it.
    pub why: String,
}

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
    /// A parameter that parsed but is outside the domain its consumer requires.
    ///
    /// Every one of these was previously accepted in silence and surfaced — if
    /// at all — as a panic thousands of ticks later, or as a plausible wrong
    /// number that never failed anything. Run start is the only useful place to
    /// reject them.
    #[error("config file `{path}` supplies an out-of-domain parameter")]
    Domain {
        path: PathBuf,
        #[source]
        source: DomainViolation,
    },
}

/// The whole parameter tree. Seven tables, every key required.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub sim: Sim,
    pub money: MoneySection,
    pub household: Household,
    pub firm: Firm,
    pub bankruptcy: Bankruptcy,
    pub ownership: Ownership,
    pub invariants: Invariants,
}

/// Build a [`DomainViolation`] for `key`.
fn violation(key: &'static str, why: impl Into<String>) -> DomainViolation {
    DomainViolation {
        key,
        why: why.into(),
    }
}

impl Params {
    /// Check every parameter against the domain its consumer actually requires.
    ///
    /// `deny_unknown_fields` and the type system between them prove the file has
    /// the right *shape*. They prove nothing about the *values*: a zero
    /// household count, a tick count past the sub-stream key's 24-bit field, a
    /// negative money pile and a non-finite demand expectation are all
    /// schema-valid TOML. Each of those was accepted in silence before this
    /// function existed, and each fails later in a way that is much harder to
    /// attribute — a panic on tick 16 777 216, or a firm whose expected demand
    /// crosses to zero units and simply looks like a firm that chose to produce
    /// nothing.
    ///
    /// Every check below names the consumer that imposes the bound, so a future
    /// reader can tell a real constraint from a guessed one. Bounds that are not
    /// imposed by a consumer in this crate are deliberately **absent**: this
    /// validates the domain, it does not calibrate the economy (that is CAL-01
    /// and CAL-02's job, in Phase 11).
    ///
    /// The first violation wins. There is no value in enumerating the rest: the
    /// operator has to edit the file and re-run regardless.
    pub fn validate(&self) -> Result<(), DomainViolation> {
        // --- run shape, against the sub-stream key's field widths ---------

        if self.sim.ticks == 0 || u64::from(self.sim.ticks) > MAX_TICK {
            return Err(violation(
                "sim.ticks",
                format!(
                    "must be in 1..={MAX_TICK}: the sub-stream key's tick field is \
                     {TICK_BITS} bits wide, and a run that outgrows it panics in \
                     pack_stream_key mid-simulation rather than at start-up"
                ),
            ));
        }

        if self.sim.households == 0 {
            return Err(violation(
                "sim.households",
                "must be non-zero: an empty population is a zero divisor, not an economy",
            ));
        }
        if u64::from(self.sim.households) > MAX_AGENT {
            return Err(violation(
                "sim.households",
                format!(
                    "must be at most {MAX_AGENT}: the sub-stream key's agent field is \
                     {AGENT_BITS} bits wide"
                ),
            ));
        }

        if self.sim.firms == 0 {
            return Err(violation(
                "sim.firms",
                "must be non-zero: an economy with no producers has nothing to trade",
            ));
        }
        if u64::from(self.sim.firms) > MAX_FIRMS {
            return Err(violation(
                "sim.firms",
                format!(
                    "must be at most {MAX_FIRMS}: FirmSlot is a u16, and a wider arena \
                     would alias two firms onto one identity"
                ),
            ));
        }

        if self.sim.month_days == 0 {
            return Err(violation(
                "sim.month_days",
                "must be non-zero: it is the divisor of the monthly accounting cadence",
            ));
        }

        // --- the money pile ------------------------------------------------

        if self.money.total_money_cents <= 0 {
            return Err(violation(
                "money.total_money_cents",
                "must be strictly positive: the core invariant is that money is a fixed \
                 pile that only ever changes hands, and an empty or negative pile is not one",
            ));
        }

        // --- the one float field, CAL-01 ------------------------------------
        //
        // Finiteness first: TOML 1.0 accepts the `nan` and `inf` literals, and a
        // NaN compares false against every bound, so an ordering check alone
        // would wave it straight through. From there it reaches pow_frac and
        // then demand_to_units, which maps NaN to 0 — a whole firm's demand
        // expectation silently becoming zero units.
        let expected_demand = self.firm.initial_expected_demand;
        if !expected_demand.is_finite() || expected_demand <= 0.0 {
            return Err(violation(
                "firm.initial_expected_demand",
                "must be finite and strictly positive (CAL-01): a non-finite or \
                 non-positive value reaches pow_frac, whose domain it is outside, and \
                 then the float/integer crossing, which turns it into a plausible zero",
            ));
        }

        // --- pow_frac_det's exponent domain, 0 < alpha < 1 -------------------

        let alpha_ppm = i64::from(self.household.consumption_exponent_ppm);
        if alpha_ppm <= 0 || alpha_ppm >= PPM_SCALE {
            return Err(violation(
                "household.consumption_exponent_ppm",
                format!(
                    "must be in 1..{PPM_SCALE}: pow_frac_det documents its domain as \
                     0 < alpha < 1, and outside it the release build returns 1.0 or a \
                     truncated value rather than failing"
                ),
            ));
        }

        // --- probabilities are probabilities ---------------------------------
        //
        // Only the keys that actually feed a coin. The other ppm keys are ratios
        // and bands — `price_ceiling_over_mc_ppm` and `entrant_price_ratio_ppm`
        // ship above one on purpose — so a blanket `_ppm <= PPM_SCALE` rule
        // would reject the shipped baseline.
        for (key, p_ppm) in [
            (
                "household.price_search_prob_ppm",
                self.household.price_search_prob_ppm,
            ),
            (
                "household.rationing_search_prob_ppm",
                self.household.rationing_search_prob_ppm,
            ),
            (
                "household.employed_search_prob_ppm",
                self.household.employed_search_prob_ppm,
            ),
            (
                "firm.price_inaction_prob_ppm",
                self.firm.price_inaction_prob_ppm,
            ),
        ] {
            if i64::from(p_ppm) > PPM_SCALE {
                return Err(violation(
                    key,
                    format!(
                        "is {p_ppm} ppm, above {PPM_SCALE}: Stream::coin_ppm can never \
                         reach it, so the coin is deterministically true with no \
                         diagnostic anywhere"
                    ),
                ));
            }
        }

        // --- sample widths against the pool they sample from -----------------

        for (key, k) in [
            (
                "household.supplier_list_size",
                self.household.supplier_list_size,
            ),
            (
                "household.firms_sampled_consumer",
                self.household.firms_sampled_consumer,
            ),
            (
                "household.firms_sampled_unemployed",
                self.household.firms_sampled_unemployed,
            ),
            (
                "household.firms_sampled_employed",
                self.household.firms_sampled_employed,
            ),
        ] {
            if k > self.sim.firms {
                return Err(violation(
                    key,
                    format!(
                        "is {k} but sim.firms is {}: Stream::sample_k cannot draw more \
                         distinct firms than exist and would panic on the first tick",
                        self.sim.firms
                    ),
                ));
            }
        }

        Ok(())
    }
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

/// Which invariant checks are active for this run.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Invariants {
    /// The only switch for the liveness check, which asserts that a tick
    /// recorded at least one cash transaction (LEDG-08).
    ///
    /// Read exactly once, at check-set construction, and never on the per-tick
    /// path: the check set a tick runs is decided before the run starts, so a
    /// tick never pays for a branch on a value that cannot change under it.
    ///
    /// Ships `false`. ROADMAP Phase 3 criterion 1 runs 3650 pre-economics
    /// ticks in which nothing trades, and every one of them would halt with the
    /// check on; ROADMAP Phase 6 criterion 7 owns flipping the shipped value in
    /// the commit that first makes wages move money.
    pub liveness_enabled: bool,
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

    // Shape is not domain. Everything above proves the file is well-formed TOML
    // matching the schema; this is what proves the values are ones the model can
    // actually run on, and it happens at run start because every alternative
    // place to notice is thousands of ticks too late.
    params.validate().map_err(|source| ConfigError::Domain {
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

    // --- Semantic validation (CR-02) -------------------------------------
    //
    // Before `Params::validate` existed, `load` did three things — read, hash,
    // parse — plus one money-headroom check, and every other parameter was
    // accepted exactly as written. Verified by execution: a baseline mutated to
    // `households = 0`, `ticks = 99999999`, `total_money_cents = -2000000` and
    // `initial_expected_demand = nan` loaded cleanly and the binary printed a
    // tracer line and exited 0.

    /// `FULL` with `line` replaced by `replacement`, parsed.
    fn params_with(line: &str, replacement: &str) -> Params {
        let document = FULL.replace(line, replacement);
        assert_ne!(document, FULL, "the substitution `{line}` matched nothing");
        toml::from_str::<Params>(&document).expect("the mutated document must still parse")
    }

    /// The key named by the violation `line` -> `replacement` produces.
    fn rejected_key(line: &str, replacement: &str) -> &'static str {
        match params_with(line, replacement).validate() {
            Ok(()) => panic!("`{replacement}` was accepted, but it is out of domain"),
            Err(violation) => violation.key,
        }
    }

    #[test]
    fn the_shipped_schema_shape_validates() {
        let params = toml::from_str::<Params>(FULL).expect("the embedded document must parse");
        assert_eq!(params.validate(), Ok(()));
    }

    #[test]
    fn a_run_length_past_the_sub_stream_key_is_rejected_at_start_up() {
        // 2^24 - 1 is the last tick the key's tick field can carry. One past it
        // used to panic at tick 16 777 216, after hours of simulated economy.
        assert_eq!(
            rejected_key("ticks = 3650", "ticks = 16777216"),
            "sim.ticks"
        );
        assert_eq!(rejected_key("ticks = 3650", "ticks = 0"), "sim.ticks");

        // The boundary itself is legal, so the check is a bound and not an
        // off-by-one that also rejects the largest usable run.
        assert_eq!(
            params_with("ticks = 3650", "ticks = 16777215").validate(),
            Ok(())
        );
    }

    #[test]
    fn an_empty_population_or_an_empty_firm_set_is_rejected() {
        assert_eq!(
            rejected_key("households = 200", "households = 0"),
            "sim.households"
        );
        assert_eq!(rejected_key("firms = 20", "firms = 0"), "sim.firms");
        assert_eq!(
            rejected_key("month_days = 21", "month_days = 0"),
            "sim.month_days"
        );
    }

    #[test]
    fn a_firm_count_wider_than_the_slot_type_is_rejected() {
        // FirmSlot is a u16; see the aliasing guard in `src/ids.rs`.
        assert_eq!(rejected_key("firms = 20", "firms = 70000"), "sim.firms");
        assert_eq!(
            params_with("firms = 20", "firms = 65535").validate(),
            Ok(()),
            "the last addressable firm count must remain legal"
        );
    }

    #[test]
    fn a_population_past_the_agent_field_is_rejected() {
        assert_eq!(
            rejected_key("households = 200", "households = 20000000"),
            "sim.households"
        );
    }

    #[test]
    fn a_non_positive_money_pile_is_rejected() {
        // The tracer used to print `money_cents=-2000000` without complaint.
        assert_eq!(
            rejected_key(
                "total_money_cents = 2000000",
                "total_money_cents = -2000000"
            ),
            "money.total_money_cents"
        );
        assert_eq!(
            rejected_key("total_money_cents = 2000000", "total_money_cents = 0"),
            "money.total_money_cents"
        );
    }

    #[test]
    fn a_non_finite_or_non_positive_demand_expectation_is_rejected() {
        // TOML 1.0 accepts the `nan` and `inf` literals, and a NaN compares
        // false against every bound — so an ordering check alone waves it
        // through. It then reaches pow_frac (NaN out) and the crossing (0 out).
        for bad in [
            "initial_expected_demand = nan",
            "initial_expected_demand = inf",
            "initial_expected_demand = -inf",
            "initial_expected_demand = 0.0",
            "initial_expected_demand = -1.0",
        ] {
            assert_eq!(
                rejected_key("initial_expected_demand = 330.0", bad),
                "firm.initial_expected_demand",
                "{bad} was not rejected"
            );
        }
    }

    #[test]
    fn a_consumption_exponent_outside_pow_fracs_domain_is_rejected() {
        // pow_frac_det documents its domain as 0 < alpha < 1.
        for bad in [
            "consumption_exponent_ppm = 0",
            "consumption_exponent_ppm = 1000000",
            "consumption_exponent_ppm = 2000000",
        ] {
            assert_eq!(
                rejected_key("consumption_exponent_ppm = 900000", bad),
                "household.consumption_exponent_ppm",
                "{bad} was not rejected"
            );
        }
    }

    #[test]
    fn a_probability_above_one_is_rejected_but_a_ratio_above_one_is_not() {
        assert_eq!(
            rejected_key(
                "price_inaction_prob_ppm = 750000",
                "price_inaction_prob_ppm = 2000000"
            ),
            "firm.price_inaction_prob_ppm"
        );
        assert_eq!(
            rejected_key(
                "price_search_prob_ppm = 250000",
                "price_search_prob_ppm = 1000001"
            ),
            "household.price_search_prob_ppm"
        );

        // The shipped baseline already carries ppm keys above one million —
        // `price_ceiling_over_mc_ppm` and `entrant_price_ratio_ppm` are ratios,
        // not probabilities. A blanket rule over `_ppm` would reject the
        // baseline, so this check must stay scoped to the coin-fed keys.
        assert_eq!(
            params_with(
                "price_ceiling_over_mc_ppm = 1150000",
                "price_ceiling_over_mc_ppm = 1400000"
            )
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn a_sample_wider_than_the_firm_pool_is_rejected() {
        // `Stream::sample_k` asserts `k <= pool.len()`, so this used to be a
        // panic on tick 1 rather than a config rejection.
        assert_eq!(
            rejected_key("supplier_list_size = 7", "supplier_list_size = 999"),
            "household.supplier_list_size"
        );
        assert_eq!(
            rejected_key("firms_sampled_consumer = 5", "firms_sampled_consumer = 21"),
            "household.firms_sampled_consumer"
        );
        // Sampling the whole pool is legal — sample_k's bound is `<=`.
        assert_eq!(
            params_with("firms_sampled_consumer = 5", "firms_sampled_consumer = 20").validate(),
            Ok(())
        );
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
