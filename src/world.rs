//! The agents, and the activation state one tick derives from the seed
//! (TICK-01, TICK-10, LEDG-01).
//!
//! [`World`] owns every agent and the four values a tick recomputes before the
//! pipeline runs: the tick number, the two activation orders, the draw count
//! those orders cost, and the digest of the permutation they form. The digest
//! is the value that carries the seed into a byte the determinism diff can see
//! (TICK-10); the draw count beside it is the fixed-draw-sampling assertion
//! (CORE-05) restated as a logged column.
//!
//! **No agent type declares a money-typed field, and none declares a
//! balance-shaped field name.** The books own every cent and every unit
//! (LEDG-01); a quantity an agent appears to hold is read back through `Books`
//! by `Account`, never cached here. ROADMAP Phase 3 criterion 7 makes that
//! mechanical rather than a matter of review: `tests/lints.sh` guard
//! `7f-agents` searches this file for a money-typed field, for a
//! balance-shaped field *name* whatever its type, and for the two type
//! declarations themselves — the third clause exists so the first two cannot
//! decay into guards over an empty set if a type is ever renamed away.
//!
//! The configuration has its own `Household` and `Firm` sections, which are
//! different types in a different module. This file refers to the
//! configuration only as [`crate::config::Params`] and deliberately imports
//! neither of those two names: with both in scope every mention of either
//! would be ambiguous to a reader, whatever the compiler resolved it to.

use crate::config::Params;
use crate::ids::{FirmArena, HouseholdId};

/// One household. An identity, and nothing else at this phase.
///
/// It holds no money and no goods, by design rather than by omission: those
/// live in the books and are addressed by `Account::Household(id)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Household {
    pub id: HouseholdId,
}

/// One firm. Its posted price in integer cents, and nothing else at this phase.
///
/// The price is a *decision*, not a balance — it is a number the firm chose,
/// not a quantity it holds — which is why it may live here while cash,
/// inventory and headcount may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Firm {
    pub price_cents: i64,
}

/// The population, and the tick's activation state.
///
/// Agents are addressed by ID and never by reference: households are a `Vec`
/// indexed by [`HouseholdId`], firms live in a [`FirmArena`] whose slots are
/// stable for the life of the run.
///
/// The two order buffers hold *indices*, not identities: `household_order`
/// holds household indices and `firm_order` holds firm slot numbers, which is
/// the form `Stream::shuffle_in_place` permutes and the form the sub-stream key
/// takes as its agent field.
#[derive(Debug, Clone)]
pub struct World {
    /// The tick being executed. The only clock in this system.
    pub tick: u32,
    pub households: Vec<Household>,
    pub firms: FirmArena<Firm>,
    /// This tick's household activation order, as household indices.
    pub household_order: Vec<u32>,
    /// This tick's firm activation order, as firm slot numbers.
    pub firm_order: Vec<u32>,
    /// Draws the two activation shuffles took this tick. Constant for a fixed
    /// population, and logged so that a divergence localises to a tick.
    pub draws_this_tick: u32,
    /// A value derived from this tick's activation permutation. Always
    /// positive, so the log column parses as a signed 64-bit integer rather
    /// than widening to an object column on the analysis side.
    pub activation_digest: i64,
}

impl World {
    /// Build the population from the run's parameters.
    ///
    /// Every firm opens at the configured initial price. Nothing else is set:
    /// the opening balances live in the books, which `Books::new` endows and
    /// then clears the endowment postings from, so that tick 0's journal is
    /// empty and the liveness check cannot pass on an endowment.
    ///
    /// # Panics
    ///
    /// If the configured firm count exceeds the arena's slot range.
    /// `config::load` refuses such a count before this point; the bound is
    /// restated by `FirmArena::with_occupants` rather than re-derived here.
    pub fn new(params: &Params) -> World {
        let households: Vec<Household> = (0..params.sim.households)
            .map(|index| Household {
                id: HouseholdId(index),
            })
            .collect();

        let firm_count = usize::try_from(params.sim.firms)
            .expect("a firm count fits in a pointer-sized index on every supported target");
        let firms = FirmArena::with_occupants(vec![
            Firm {
                price_cents: params.firm.initial_price_cents,
            };
            firm_count
        ]);

        World {
            tick: 0,
            household_order: Vec::with_capacity(households.len()),
            firm_order: Vec::with_capacity(firms.len()),
            households,
            firms,
            draws_this_tick: 0,
            activation_digest: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");

    fn shipped() -> Params {
        crate::config::load(Path::new(CONFIG))
            .expect("the shipped configuration loads")
            .0
    }

    #[test]
    fn the_population_is_the_configured_one() {
        let params = shipped();
        let world = World::new(&params);

        assert_eq!(
            world.households.len(),
            params.sim.households as usize,
            "one household per configured household"
        );
        assert_eq!(
            world.firms.len(),
            params.sim.firms as usize,
            "one firm slot per configured firm"
        );

        // Identities are the dense index, so `households[i].id == i`. Asserted
        // rather than assumed: every activation order below is a permutation of
        // those indices, and a mismatch would make the order name the wrong
        // agent while looking entirely well-formed.
        for (index, household) in world.households.iter().enumerate() {
            assert_eq!(
                household.id,
                HouseholdId(u32::try_from(index).expect("the household count is bounded"))
            );
        }
    }

    #[test]
    fn every_firm_opens_at_the_configured_price() {
        let params = shipped();
        let world = World::new(&params);

        for id in world.firms.live_ids() {
            let firm = world.firms.get(id).expect("a live identity resolves");
            assert_eq!(firm.price_cents, params.firm.initial_price_cents);
        }
    }

    #[test]
    fn a_fresh_world_has_no_activation_state() {
        let params = shipped();
        let world = World::new(&params);

        // The orders are empty until the first shuffle, and the digest is not
        // yet a digest of anything. A world whose digest looked plausible
        // before a shuffle ran would hide a pipeline that never shuffled.
        assert_eq!(world.tick, 0);
        assert!(world.household_order.is_empty());
        assert!(world.firm_order.is_empty());
        assert_eq!(world.draws_this_tick, 0);
        assert_eq!(world.activation_digest, 0);
    }
}
