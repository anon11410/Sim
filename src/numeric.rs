//! The float domain, and the only place in the crate permitted to name a
//! floating-point type.
//!
//! Every other module works in integer cents, integer units and integer
//! parts-per-million. This module exists so that the one place the model
//! genuinely needs fractional arithmetic — MKT-01's `(m / P_bar)^0.9` — is a
//! module you can hold in one hand, and so the crate-wide ban on the
//! non-deterministic standard-library float methods never needs an
//! allow-attribute escape hatch (D-11).
//!
//! **Every operation used here is IEEE-754 correctly rounded**: addition,
//! subtraction, multiplication, division, comparison, `round`, and the square
//! root. Each therefore has a single uniquely determined result for a given
//! pair of inputs, on every machine and on every invocation. Not one method
//! carrying the standard library's unspecified-precision disclaimer — which
//! says in as many words that precision is not deterministic and can differ
//! *within the same execution, from one invocation to the next* — appears
//! anywhere in this crate.
//!
//! Two boundaries are drawn here and nowhere else:
//!
//! * [`pow_frac`] raises a positive value to a fractional power without
//!   calling the standard library's power routine, which is on the banned list
//!   for exactly the reason above.
//! * [`demand_to_units`] is the single named crossing from this domain back to
//!   the integer domain.

/// Bits of the fractional power consumed by [`pow_frac_det`].
///
/// Forty gives a worst relative error of about 2e-12 against the standard
/// library's power routine, measured over twenty thousand inputs — far below
/// any economically meaningful resolution — while costing twelve fewer
/// iterations than full precision.
///
/// **This is a committed constant, not a tunable.** Changing it changes every
/// trajectory exactly as changing an economic parameter would, and would force
/// every golden run and snapshot to be regenerated. That is why it is code and
/// not a configuration key: putting an iteration count into an economics config
/// invites someone to tune it. CORE-10's carve-out covers this; the matching
/// `GRADE: PROJECT` rationale is recorded in `config/PROVENANCE.md`.
pub const POW_FRAC_BITS: u32 = 40;

/// Parts per million. Probabilities and rates enter the model as integers on
/// this scale, so a threshold parameter never has to be a float. Not a
/// configuration key for the same reason as [`POW_FRAC_BITS`] (D-14).
pub const PPM_SCALE: i64 = 1_000_000;

/// Thousandths. The second integer scale the model uses for rates. Not a
/// configuration key (D-14).
pub const MILLI_SCALE: i64 = 1_000;

/// `x` raised to the power `alpha`, for `x > 0` and `0 < alpha < 1`, using
/// `bits` binary digits of `alpha` and **only** correctly-rounded operations.
///
/// Write `alpha` in binary. Then `x^alpha` is the product of `x^(2^-k)` over
/// the set bits `k` of `alpha`, and `x^(2^-k)` is `k` repeated square roots of
/// `x`. So the whole computation is square roots and multiplications, both of
/// which IEEE-754 requires to be correctly rounded, and the result is uniquely
/// determined — bit-identical across invocations, processes and machines.
///
/// Consequences worth relying on: at `alpha = 0.5` a single set bit is
/// consumed on the first iteration, so the result is exactly `x.sqrt()`, and at
/// `alpha = 0.25` it is exactly `x.sqrt().sqrt()`.
///
/// The three-argument form exists so `bits` can be swept in a test. Callers on
/// the behaviour path use [`pow_frac`], which fixes it at [`POW_FRAC_BITS`].
pub fn pow_frac_det(x: f64, alpha: f64, bits: u32) -> f64 {
    debug_assert!(x > 0.0, "pow_frac_det is defined for a positive base only");
    debug_assert!(
        alpha > 0.0 && alpha < 1.0,
        "pow_frac_det is defined for a power strictly between zero and one"
    );

    let mut accumulator = 1.0f64;
    let mut root = x;
    let mut remaining = alpha;

    for _ in 0..bits {
        // One more square root: `root` is now x^(2^-k) at iteration k.
        root = root.sqrt();
        // Shift `remaining` left by one binary place; a carry past one means
        // this bit of the power is set, so fold the current root in.
        remaining *= 2.0;
        if remaining >= 1.0 {
            accumulator *= root;
            remaining -= 1.0;
        }
    }

    accumulator
}

/// `x` raised to the power `alpha` at the committed [`POW_FRAC_BITS`].
///
/// This is the entry point the behaviour path calls.
pub fn pow_frac(x: f64, alpha: f64) -> f64 {
    pow_frac_det(x, alpha, POW_FRAC_BITS)
}

/// The **only** crossing from the float domain to the integer domain in this
/// crate (D-11).
///
/// Rounds half away from zero, then casts. The cast saturates rather than
/// wrapping, so a magnitude beyond the integer range yields the nearest
/// representable bound instead of a large value of the opposite sign — the
/// failure mode a wrapping cast would produce from a large positive demand.
/// Debug builds additionally assert that the input is finite, so a division
/// that produced a non-finite value fails at the crossing rather than at some
/// later use of the result.
///
/// Note that this rounds only where the model needs a whole number of units.
/// The demand field itself is written to the run record at full round-trip
/// precision and is never truncated on the way out (D-13).
pub fn demand_to_units(x: f64) -> i64 {
    debug_assert!(x.is_finite(), "a non-finite value reached the crossing");
    x.round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The inputs the research pass measured over.
    const SAMPLE_INPUTS: [f64; 4] = [0.01, 1.0, 2.0, 199.99];

    /// A deterministic sweep of the model's plausible input range.
    fn swept_inputs() -> Vec<f64> {
        let mut values = Vec::new();
        let mut step = 1;
        while step <= 20_000 {
            values.push(step as f64 * 0.01);
            step += 1;
        }
        values
    }

    #[test]
    fn half_power_is_exactly_one_square_root() {
        for x in SAMPLE_INPUTS {
            assert_eq!(pow_frac(x, 0.5).to_bits(), x.sqrt().to_bits(), "x = {x}");
        }
    }

    #[test]
    fn quarter_power_is_exactly_two_square_roots() {
        for x in SAMPLE_INPUTS {
            assert_eq!(
                pow_frac(x, 0.25).to_bits(),
                x.sqrt().sqrt().to_bits(),
                "x = {x}"
            );
        }
    }

    #[test]
    fn one_raised_to_any_fractional_power_is_one() {
        for alpha in [0.1f64, 0.5, 0.9] {
            assert_eq!(pow_frac(1.0, alpha), 1.0, "alpha = {alpha}");
        }
    }

    #[test]
    fn pow_frac_is_non_decreasing_in_x() {
        let mut previous = f64::NEG_INFINITY;
        for x in swept_inputs() {
            let value = pow_frac(x, 0.9);
            assert!(value >= previous, "not monotone at x = {x}");
            previous = value;
        }
    }

    #[test]
    fn pow_frac_returns_one_bit_pattern_across_many_calls() {
        let first = pow_frac(1.5, 0.9).to_bits();
        for _ in 0..100_000 {
            assert_eq!(pow_frac(1.5, 0.9).to_bits(), first);
        }
    }

    #[test]
    fn twenty_bits_and_forty_bits_differ_somewhere_on_the_range() {
        let differs = swept_inputs()
            .into_iter()
            .any(|x| pow_frac_det(x, 0.9, 20) != pow_frac_det(x, 0.9, 40));
        assert!(differs, "the bit count is decorative, not load-bearing");
    }

    #[test]
    fn forty_bits_and_full_precision_agree_to_one_part_in_a_billion() {
        for x in swept_inputs() {
            let coarse = pow_frac_det(x, 0.9, 40);
            let fine = pow_frac_det(x, 0.9, 52);
            let relative = ((coarse - fine) / fine).abs();
            assert!(relative < 1e-9, "relative difference {relative} at x = {x}");
        }
    }

    #[test]
    fn pow_frac_uses_the_committed_bit_count() {
        for x in SAMPLE_INPUTS {
            assert_eq!(
                pow_frac(x, 0.9).to_bits(),
                pow_frac_det(x, 0.9, POW_FRAC_BITS).to_bits()
            );
        }
        // If POW_FRAC_BITS were lowered to the sweep's coarse end, this would
        // hold with equality and the assertion would fail.
        let differs = swept_inputs()
            .into_iter()
            .any(|x| pow_frac(x, 0.9) != pow_frac_det(x, 0.9, 20));
        assert!(differs, "POW_FRAC_BITS is no finer than the coarse sweep");
    }

    #[test]
    fn the_crossing_rounds_half_away_from_zero() {
        assert_eq!(demand_to_units(2.5), 3);
        assert_eq!(demand_to_units(-2.5), -3);
        assert_eq!(demand_to_units(2.4), 2);
        assert_eq!(demand_to_units(-2.4), -2);
    }

    #[test]
    fn the_crossing_maps_zero_to_zero_and_saturates_out_of_range() {
        assert_eq!(demand_to_units(0.0), 0);
        assert_eq!(demand_to_units(1e30), i64::MAX);
        assert_eq!(demand_to_units(-1e30), i64::MIN);
    }

    #[test]
    fn the_integer_scale_constants_are_ppm_and_milli() {
        assert_eq!(PPM_SCALE, 1_000_000);
        assert_eq!(MILLI_SCALE, 1_000);
        assert_eq!(POW_FRAC_BITS, 40);
    }
}
