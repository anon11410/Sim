//! Agent identity. Dense integer newtypes, and a firm arena whose identities
//! cannot silently alias across a respawn (CORE-06).
//!
//! Everything here is data plus lookup. No agent holds a reference to another
//! agent; an agent is addressed by an identity and resolved against the
//! collection that owns it, which is why no reference-counted interior-mutable
//! cell appears anywhere in this crate.
//!
//! **Why a firm identity carries a generation.** Phase 10 bankrupts a firm and
//! respawns a replacement into the same slot. Without a generation, a
//! [`FirmId`] captured before the respawn resolves afterwards to a *different*
//! firm, and the symptom is a plausible wrong number rather than a crash — the
//! hardest class of defect to find in an emergent system. With one, both
//! accessors return `None`: a typed miss the compiler forces the caller to
//! handle.
//!
//! **The field is spelled `generation`, not `gen`.** `gen` is a reserved
//! keyword in Rust edition 2024; the requirement text and the research pattern
//! both write `FirmId { slot, gen }`, but that literal spelling does not
//! compile and would force `r#gen` at every construction and field access for
//! the remaining ten phases. The type shape, the derived total order and the
//! log identity `(slot, generation)` are unchanged.
//!
//! Ordering is derived on every type here, so `(slot, generation)` is a total
//! order. That is what LABR-09's "every comparator over agents is tie-broken by
//! agent ID" is written against: a comparator ending in an identity comparison
//! never has an unspecified tie order, whatever the sort algorithm does.

/// A household, addressed by its dense index into the household vector.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HouseholdId(pub u32);

/// A position in the firm arena. Stable for the whole run: a slot is reused by
/// a respawned firm, never removed and never moved.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirmSlot(pub u16);

/// A good. One good ("food") in v1; the newtype exists so that adding a second
/// one is a type change rather than a search for bare integers.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GoodId(pub u16);

/// A firm identity: a slot plus the generation of the occupant that identity
/// was issued for.
///
/// Two identities naming the same slot in different generations are different
/// identities and compare unequal. The derived `Ord` orders slot-major then
/// generation-major.
///
/// Only `slot` is ever passed as the RNG sub-stream key's agent field, never
/// `generation` (D-03): two firms in one slot in different generations never
/// coexist at the same tick, so the key stays unique on the slot alone, and
/// letting a respawned firm inherit the previous occupant's keystream position
/// would be a defect rather than a saving.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirmId {
    pub slot: FirmSlot,
    pub generation: u32,
}

/// Who a ledger posting names. The addressing type Phase 2's ledger posts
/// against, so that a household and a firm sharing an underlying index are
/// never the same account.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Account {
    Household(HouseholdId),
    Firm(FirmId),
}

/// One arena slot: the generation currently occupying it, and the occupant.
#[derive(Debug, Clone)]
struct SlotRecord<T> {
    generation: u32,
    occupant: T,
}

/// A fixed-length arena of firms, indexed by [`FirmSlot`], where a slot is
/// reused in place by incrementing its generation.
///
/// Two construction rules, both decided once here and found wrong much later:
///
/// 1. The arena never uses the vector operation that removes an element by
///    swapping the last one into its place, and exposes no element-removal
///    operation at all. BANK-03 requires respawn in place precisely because
///    reordering the backing storage would reorder agent iteration and change
///    the trajectory of every run. [`FirmArena::respawn_in_place`] is the only
///    mutation of the slot vector, and it changes no index and no length.
/// 2. Only [`FirmId::slot`] is ever passed as the RNG sub-stream key's agent
///    field, never the generation (D-03).
///
/// No hashed collection appears here, and no point-lookup wrapper around one is
/// built: D-06 declines that escape hatch for this phase. Every v1 relation is
/// dense-integer keyed or small enough for an ordered map, and not building the
/// hatch is the cheapest way to keep the lint honest.
#[derive(Debug, Clone)]
pub struct FirmArena<T> {
    slots: Vec<SlotRecord<T>>,
}

impl<T> FirmArena<T> {
    /// Build an arena from an occupant per slot. Every slot starts at
    /// generation 0, and slot `i` holds `occupants[i]`.
    pub fn with_occupants(occupants: Vec<T>) -> Self {
        FirmArena {
            slots: occupants
                .into_iter()
                .map(|occupant| SlotRecord {
                    generation: 0,
                    occupant,
                })
                .collect(),
        }
    }

    /// Number of slots. Fixed for the life of the arena.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the arena has no slots.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The occupant `id` names, or `None` if `id` is stale or out of range.
    ///
    /// The generation comparison is the whole point of the type: an identity
    /// held across a respawn of its slot resolves here to `None`, never to the
    /// new occupant.
    pub fn get(&self, id: FirmId) -> Option<&T> {
        let record = self.slots.get(id.slot.0 as usize)?;
        (record.generation == id.generation).then_some(&record.occupant)
    }

    /// The occupant `id` names, mutably, or `None` if `id` is stale or out of
    /// range. Same generation check as [`FirmArena::get`].
    pub fn get_mut(&mut self, id: FirmId) -> Option<&mut T> {
        let record = self.slots.get_mut(id.slot.0 as usize)?;
        (record.generation == id.generation).then_some(&mut record.occupant)
    }

    /// The current identity of `slot`, or `None` if the slot is out of range.
    pub fn id_at(&self, slot: FirmSlot) -> Option<FirmId> {
        let record = self.slots.get(slot.0 as usize)?;
        Some(FirmId {
            slot,
            generation: record.generation,
        })
    }

    /// Replace the occupant of `slot` and issue its successor identity.
    ///
    /// The replacement lands at the same index; the vector's length and the
    /// position of every other slot are untouched, so agent iteration order is
    /// unchanged (BANK-03). Every identity previously issued for this slot is
    /// stale from this point on.
    ///
    /// Panics if `slot` is out of range: the arena is fixed-length for the life
    /// of a run, so an out-of-range slot is a program defect and not a runtime
    /// condition to report.
    pub fn respawn_in_place(&mut self, slot: FirmSlot, occupant: T) -> FirmId {
        let record = self
            .slots
            .get_mut(slot.0 as usize)
            .expect("respawn_in_place called with a slot outside the arena");
        record.generation += 1;
        record.occupant = occupant;
        FirmId {
            slot,
            generation: record.generation,
        }
    }

    /// Every current identity, in ascending slot order.
    ///
    /// The arena has no vacancy concept — every slot is occupied for the whole
    /// run — so the result always has one identity per slot.
    pub fn live_ids(&self) -> Vec<FirmId> {
        self.slots
            .iter()
            .enumerate()
            .map(|(index, record)| FirmId {
                slot: FirmSlot(index as u16),
                generation: record.generation,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fid(slot: u16, generation: u32) -> FirmId {
        FirmId {
            slot: FirmSlot(slot),
            generation,
        }
    }

    #[test]
    fn generation_is_part_of_identity() {
        assert_ne!(fid(3, 0), fid(3, 1));
        assert_eq!(fid(3, 0), fid(3, 0));
    }

    #[test]
    fn firm_ids_order_slot_major_then_generation() {
        let mut ids = vec![fid(1, 5), fid(0, 9), fid(1, 2)];
        ids.sort();
        assert_eq!(ids, vec![fid(0, 9), fid(1, 2), fid(1, 5)]);
    }

    #[test]
    fn get_is_some_for_a_live_identity_and_none_beyond_the_arena() {
        let arena = FirmArena::with_occupants(vec![10u32, 20, 30]);
        let live = arena.id_at(FirmSlot(1)).expect("slot 1 is in range");
        assert_eq!(arena.get(live), Some(&20));
        assert_eq!(arena.get(fid(99, 0)), None);
        assert_eq!(arena.id_at(FirmSlot(99)), None);
    }

    #[test]
    fn a_stale_identity_is_a_typed_miss_through_both_accessors() {
        let mut arena = FirmArena::with_occupants(vec![10u32, 20, 30, 40, 50]);
        let stale = arena.id_at(FirmSlot(3)).expect("slot 3 is in range");
        let fresh = arena.respawn_in_place(FirmSlot(3), 400);

        assert_eq!(arena.get(stale), None);
        assert_eq!(arena.get_mut(stale), None);
        assert_eq!(arena.get(fresh), Some(&400));
        assert_eq!(arena.get_mut(fresh), Some(&mut 400));
    }

    #[test]
    fn respawn_returns_the_same_slot_at_exactly_one_greater_generation() {
        let mut arena = FirmArena::with_occupants(vec![10u32, 20, 30, 40, 50]);
        let before = arena.id_at(FirmSlot(3)).expect("slot 3 is in range");
        let after = arena.respawn_in_place(FirmSlot(3), 400);

        assert_eq!(after.slot, before.slot);
        assert_eq!(after.generation, before.generation + 1);
        assert_eq!(arena.id_at(FirmSlot(3)), Some(after));
    }

    #[test]
    fn live_ids_are_ascending_and_cover_every_slot() {
        let mut arena = FirmArena::with_occupants(vec![10u32, 20, 30]);
        arena.respawn_in_place(FirmSlot(1), 200);
        let ids = arena.live_ids();

        assert_eq!(ids.len(), arena.len());
        assert_eq!(ids, vec![fid(0, 0), fid(1, 1), fid(2, 0)]);
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn respawn_disturbs_no_neighbouring_slot() {
        let mut arena = FirmArena::with_occupants(vec![10u32, 20, 30, 40, 50]);
        let two = arena.id_at(FirmSlot(2)).expect("slot 2 is in range");
        let four = arena.id_at(FirmSlot(4)).expect("slot 4 is in range");

        arena.respawn_in_place(FirmSlot(3), 400);

        assert_eq!(arena.id_at(FirmSlot(2)), Some(two));
        assert_eq!(arena.id_at(FirmSlot(4)), Some(four));
        assert_eq!(arena.get(two), Some(&30));
        assert_eq!(arena.get(four), Some(&50));
        assert_eq!(arena.len(), 5);
    }

    #[test]
    fn the_newtypes_and_account_carry_equality_and_a_total_order() {
        assert!(HouseholdId(1) < HouseholdId(2));
        assert!(FirmSlot(1) < FirmSlot(2));
        assert!(GoodId(1) < GoodId(2));

        let household = Account::Household(HouseholdId(7));
        let firm = Account::Firm(fid(7, 0));
        assert_ne!(household, firm);

        let mut accounts = vec![firm, household];
        accounts.sort();
        assert_eq!(accounts, vec![household, firm]);
    }
}
