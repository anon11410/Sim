# Phase 2: Books, Journal and Invariants - Context

**Gathered:** 2026-08-31
**Status:** Ready for planning
**Mode:** Autonomous smart-discuss — classified **infrastructure phase**, no grey areas escalated

<domain>
## Phase Boundary

A single `Books` module becomes the sole owner of every cent and every goods unit
in the simulation. `Household` and `Firm` hold no balance fields at all — the
two-mutable-borrows problem is dissolved rather than worked around, because both
legs of any transfer live inside one owner.

On top of that ledger sit the four conservation invariants plus a liveness
check, running as a **real pipeline phase returning `Result`** in release builds,
halting on the tick a violation occurs and naming the offending posting.

Delivers LEDG-01 … LEDG-10. Contains **no economic behaviour** — nothing decides
a price, a wage or a hire here. This phase exists so that every economic rule
added from Phase 5 onward is born under the check rather than retrofitted into
it.

</domain>

<decisions>
## Implementation Decisions

Classified infrastructure by the smart-discuss test: every ROADMAP success
criterion for this phase is technical (`grep` proves no `debug_assert!` on the
invariant path; "a test observing the books mid-transaction is impossible to
write"), and no user-facing behaviour is described. The design is already
pinned by REQUIREMENTS LEDG-01…10 and the research summary's Correctness
Constraints, so no grey area was escalated for a user decision.

### Locked by prior decisions — not reopened here

- **Agents own no value.** `Books` holds every balance; no `set_cash` exists
  anywhere. (Research SUMMARY, Architecture — the load-bearing move.)
- **`transfer()` is the only cash-mutation point, and is atomic.** The books are
  never observable mid-transaction, which is what makes zero-sum trade a
  property of the API rather than a thing to test for.
- **Invariants are a pipeline phase returning `Result`, never `debug_assert!`.**
  `debug_assert!` is compiled out of release, and an invariant absent from the
  binary that produced the run is worth nothing. Cost is ~220 `i64` adds per
  tick — run it in release, every tick, always.
- **The journal is a per-tick buffer, not an append-forever log.** A decade
  produces ~10⁶ postings; a violation is always locatable inside the tick it
  occurred. Accumulate, check, bisect to name the offending posting, clear.
  Disk write is a config flag.
- **Liveness is config-gated off for Phase 3's pre-economics empty run** and on
  by default from Phase 6. Recorded as a cross-phase constraint by the
  roadmapper: LEDG-08 would otherwise fail TICK-08's 3,650 empty ticks.
- **Money is already checked.** `Money` panics on overflow in every profile and
  `Money::split` conserves the remainder — both delivered and tested in Phase 1,
  so LEDG-03's obligation here is on the *callers*: subtract what was actually
  transferred, never the intended amount.

### Claude's Discretion

All remaining implementation choices — module layout inside `books`, the
journal's internal representation, the exact `Result` error type carried out of
the invariant phase, and how the bisect-to-offending-posting search is written.
Guided by ROADMAP success criteria and the conventions Phase 1 established.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`ids::Account`** — `Household(HouseholdId) | Firm(FirmId)`. Its doc comment
  states outright that it is "the addressing type Phase 2's ledger posts
  against, so that a household and a firm sharing an underlying index are never
  the same account." The seam was cut for this phase.
- **`ids::FirmArena<T>`** — generational slots, `respawn_in_place` as the only
  slot-vector mutation, no element removal exposed at all. The ledger can hold
  per-firm balances against `FirmId` without any respawn hazard.
- **`money::Money`** — private `i64`, checked operators that panic in every
  profile, `checked_add` / `checked_sub` / `try_scale` returning
  `MoneyOverflow`, and `split(n)` with deterministic remainder distribution.
- **`numeric`** — the float/integer boundary, with the banned-method lint
  already enforcing it.

### Established Patterns

- Module-level `//!` docs carry the requirement IDs (`CORE-01`, `LEDG-…`) and
  the decision IDs (`D-07`, `D-20`) that justify the design. Follow this — the
  validation audit greps for these tags.
- Guards are adversarial, not declarative: `tests/lints.sh` injects a hazard and
  proves the gate blocks it. Phase 2's negative test is the same discipline
  applied to the invariants — seed a leak, prove the run halts.
- Constants that would drift are generated from a source of truth and committed,
  never typed from memory (`clippy.toml` from the pinned std source).
- `pub mod` surface in `lib.rs` is flat and alphabetical; `main.rs` stays thin.

### Integration Points

- `src/lib.rs` — add `pub mod books;` (and a journal module if separated).
- `Account` is the posting key; no new addressing type should be invented.
- Phase 3 consumes this: the `const PHASES` table will call the invariant phase
  as a real pipeline step, and the log schema will carry the conservation series.

</code_context>

<specifics>
## Specific Ideas

- The gate for this phase is the **negative test**: a deliberately seeded leak —
  a dropped cent, an over-credited sale, a driven-negative balance, a non-zero-sum
  trade — must halt the run and print tick, agent and offending posting. An
  invariant never observed to fire has never been shown to work.
- `firm_cash / total_money` is wanted as a logged series (OWN-07) because it is
  the earliest warning of the deflationary stall, visible 1–2 years before prices
  or unemployment move. The ledger should make that ratio cheap to compute.

</specifics>

<deferred>
## Deferred Ideas

- Writing the journal to disk each tick — a config flag exists in the design but
  the per-tick buffer is what Phase 2 needs; disk persistence belongs with the
  log seam in Phase 3.
- Any economic behaviour whatsoever. Nothing in this phase decides a price, a
  wage or a hire.

</deferred>
