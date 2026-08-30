//! CORE-06 at the library surface: a firm identity held across a respawn is a
//! typed miss, respawn moves nothing, and identity carries a total order.
//!
//! These reach `sim::ids` through the public crate surface, which is what
//! CORE-08's "integration tests can reach all code" means in practice.

use sim::ids::{Account, FirmArena, FirmId, FirmSlot, HouseholdId};

fn arena_of_five() -> FirmArena<u32> {
    FirmArena::with_occupants(vec![10, 20, 30, 40, 50])
}

fn fid(slot: u16, generation: u32) -> FirmId {
    FirmId {
        slot: FirmSlot(slot),
        generation,
    }
}

#[test]
fn stale_identity_after_respawn_is_a_typed_miss() {
    let mut arena = arena_of_five();
    let held = arena.id_at(FirmSlot(3)).expect("slot 3 is in range");
    assert_eq!(arena.get(held), Some(&40));

    let fresh = arena.respawn_in_place(FirmSlot(3), 400);

    // The held identity now names a firm that no longer exists. Both accessors
    // say so; neither silently resolves to the new occupant.
    assert_eq!(arena.get(held), None);
    assert_eq!(arena.get_mut(held), None);

    assert_eq!(arena.get(fresh), Some(&400));
    assert_eq!(arena.get_mut(fresh), Some(&mut 400));
    assert_ne!(held, fresh);
}

#[test]
fn respawn_does_not_disturb_neighbouring_slots() {
    let mut arena = FirmArena::with_occupants(vec![10u32, 20, 30]);
    let left = arena.id_at(FirmSlot(0)).expect("slot 0 is in range");
    let right = arena.id_at(FirmSlot(2)).expect("slot 2 is in range");

    arena.respawn_in_place(FirmSlot(1), 200);

    assert_eq!(arena.id_at(FirmSlot(0)), Some(left));
    assert_eq!(arena.id_at(FirmSlot(2)), Some(right));
    assert_eq!(arena.get(left), Some(&10));
    assert_eq!(arena.get(right), Some(&30));
    assert_eq!(arena.len(), 3);
}

#[test]
fn firm_ids_sort_slot_major_then_generation() {
    let mut ids = vec![fid(1, 5), fid(0, 9), fid(1, 2), fid(0, 0)];
    ids.sort();
    assert_eq!(ids, vec![fid(0, 0), fid(0, 9), fid(1, 2), fid(1, 5)]);

    // The same total order is what makes an agent-ID tie-break well defined
    // (LABR-09), including across the two account kinds.
    let mut accounts = vec![
        Account::Firm(fid(0, 0)),
        Account::Household(HouseholdId(9)),
        Account::Household(HouseholdId(2)),
    ];
    accounts.sort();
    assert_eq!(
        accounts,
        vec![
            Account::Household(HouseholdId(2)),
            Account::Household(HouseholdId(9)),
            Account::Firm(fid(0, 0)),
        ]
    );
    // A household and a firm sharing an index are never the same account.
    assert_ne!(Account::Household(HouseholdId(7)), Account::Firm(fid(7, 0)));
}

#[test]
fn live_ids_are_ascending_and_complete() {
    let mut arena = arena_of_five();
    arena.respawn_in_place(FirmSlot(1), 200);
    arena.respawn_in_place(FirmSlot(1), 201);
    arena.respawn_in_place(FirmSlot(4), 500);

    let ids = arena.live_ids();
    assert_eq!(ids.len(), arena.len());
    assert!(
        ids.windows(2).all(|pair| pair[0] < pair[1]),
        "live_ids is not strictly increasing: {ids:?}"
    );
    assert_eq!(
        ids,
        vec![fid(0, 0), fid(1, 2), fid(2, 0), fid(3, 0), fid(4, 1)]
    );
}
