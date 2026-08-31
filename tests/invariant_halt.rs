//! The end-to-end proof for LEDG-08, LEDG-09 and LEDG-10: a library tick loop
//! aborts at exactly the tick that recorded no transaction when the liveness
//! gate is on, and runs to completion when it is off.
//!
//! Everything here goes through the crate's public API, and that is the point.
//! The liveness violation is the one violation reachable with no fault
//! injection at all — an integration test cannot reach a crate-internal
//! test-only corruption method, so a violation it *can* reach is the honest
//! subject for this level. The seeded corruptions live in plan 02-05, in unit
//! tests, where they belong.
//!
//! Phase 2 has no tick pipeline, so "halts the run" is proved here at the
//! library level: the `?` really propagates and the loop really stops. The
//! process level — a non-zero exit code with the message on stderr — is
//! Phase 3's, against the built binary.

use std::path::Path;

use sim::books::Books;
use sim::config::{self, Params};
use sim::ids::{Account, FirmId, FirmSlot, HouseholdId};
use sim::invariants::{CheckId, CheckSet, Violation};
use sim::money::Money;

/// How many ticks the loop attempts.
const TICKS: u32 = 10;

/// The one tick on which the loop moves no money.
const SILENT_TICK: u32 = 4;

/// The shipped configuration, loaded through the real deserialisation path,
/// with only the liveness gate set afterwards.
///
/// Loading rather than hand-writing a parameter literal is deliberate twice
/// over: it exercises the deserialisation the binary uses, and setting the
/// field afterwards proves the gate is read from the parameters rather than
/// from a constant compiled into the check set.
fn shipped_with_liveness(enabled: bool) -> Params {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/baseline.toml");
    let (mut params, _hash) = config::load(&path).expect("the shipped configuration loads");
    params.invariants.liveness_enabled = enabled;
    params
}

/// Drive a tick loop that moves a cent on every tick except [`SILENT_TICK`],
/// running the check set and closing the tick each time.
///
/// Records the last tick whose body began in `reached`, because "it returned an
/// error" and "it stopped" are two different claims and only the second is what
/// LEDG-10 asks for.
fn run_loop(liveness_enabled: bool, reached: &mut Option<u32>) -> Result<(), Violation> {
    let params = shipped_with_liveness(liveness_enabled);
    let mut books = Books::new(&params).expect("the shipped endowment sums to the stock");
    let checks = CheckSet::from_params(&params);

    let payer = Account::Household(HouseholdId(0));
    let payee = Account::Firm(FirmId {
        slot: FirmSlot(0),
        generation: 0,
    });

    for tick in 0..TICKS {
        *reached = Some(tick);

        if tick != SILENT_TICK {
            books
                .transfer(payer, payee, Money::from_cents(1))
                .expect("an endowed household can pay a cent");
        }

        checks.run(&books, tick)?;
        books.end_of_tick();
    }

    Ok(())
}

#[test]
fn with_the_gate_on_the_loop_halts_at_exactly_the_tick_that_traded_nothing() {
    let mut reached = None;
    let outcome = run_loop(true, &mut reached);

    // The rendered message is a different claim from the value, and LEDG-09 is
    // written about what a human reads. Asserted once, here; plan 02-05 tests
    // the message exhaustively over every variant.
    let rendered = outcome
        .as_ref()
        .expect_err("the gate is on and one tick trades nothing")
        .to_string();
    assert!(
        rendered.contains(&format!("tick {SILENT_TICK}")),
        "the halt message does not name the tick: {rendered}"
    );

    // Whole-value equality, never a substring of the message: a substring
    // assertion passes when the wrong check fired, when the tick is wrong and
    // when the counts are wrong.
    assert_eq!(
        outcome,
        Err(Violation::Liveness {
            tick: SILENT_TICK,
            counted: 0,
            required: 1,
        })
    );

    // The loop STOPPED. Without this, a loop that swallowed the error and ran
    // to the end would pass the assertion above.
    assert_eq!(
        reached,
        Some(SILENT_TICK),
        "the loop began a tick after the violating one"
    );
}

#[test]
fn with_the_gate_off_the_identical_loop_runs_every_tick() {
    let mut reached = None;
    let outcome = run_loop(false, &mut reached);

    // The gate is the only difference between this test and the one above.
    assert_eq!(outcome, Ok(()));
    assert_eq!(reached, Some(TICKS - 1));
}

#[test]
fn the_gate_removes_exactly_one_check_and_never_disables_the_phase() {
    let on = CheckSet::from_params(&shipped_with_liveness(true));
    let off = CheckSet::from_params(&shipped_with_liveness(false));

    assert_eq!(
        on.active_ids(),
        vec![CheckId::MoneyConservation, CheckId::Liveness]
    );
    assert_eq!(off.active_ids(), vec![CheckId::MoneyConservation]);
}
