# Sim — Minimal Closed Economy

## What This Is

An agent-based macroeconomic simulation. 200 households and 20 firms trade a single good
("food") in a closed economy where money is a fixed pile that only ever changes hands. One
tick is one day; the target run is 10 simulated years.

This first build is a correctness foundation, not a demo. Every later capability — a second
good, capital, banks, government, demographics, a stock market — is built on top of this daily
loop, so the loop has to be right before it is interesting.

## Core Value

**The daily tick loop must be provably correct and demonstrably alive.** Money conserved to
the cent every tick, runs byte-identically reproducible from a seed, and an economy that
fluctuates rather than pinning or spiralling. If this is wrong, nothing built on it can be
right.

## Requirements

### Validated

(None yet — ship to validate)

### Active

**Simulation core**
- [ ] Closed economy in Rust: 200 households, 20 firms, 1 good, 1 tick = 1 day
- [ ] Fixed tick order, each step completing for all agents before the next begins:
      firm planning → labour market → production → wages → goods market → firm accounting →
      bankruptcy → invariants → log
- [ ] Firm planning on a weekly cadence, agents staggered across the week (synchronised
      decisions create fake oscillations)
- [ ] Adaptive demand expectation: `expected_demand += λ * (last_sales - expected_demand)`
- [ ] Price rule keyed to inventory vs buffer target, floored at unit labour cost
- [ ] Wage rule keyed to unfilled vacancies and inventory, floored
- [ ] Decentralised labour market with bounded firm sampling per job seeker
- [ ] Reservation wages that rise while employed and decay while unemployed
- [ ] Decentralised goods market with bounded firm sampling and cheapest-first purchase,
      falling through to next-cheapest on stockout
- [ ] Wealth-dependent household spending budget that never spends to zero
- [ ] Firm ownership with dividends paid above a working-capital buffer
- [ ] Bankruptcy: release workers, transfer residual cash to owner, remove firm, respawn a
      smaller firm owned by a random household (phase-1 placeholder for endogenous entry)

**Correctness scaffolding — built in from the first commit, not bolted on**
- [ ] Integer money in minor units (cents) throughout; no float in money, prices, wages or
      balances
- [ ] ID-based data layout: `Vec<Household>` / `Vec<Firm>`, IDs as indices, no inter-agent
      references, no `Rc<RefCell<...>>`
- [ ] Four invariants checked every tick — money conservation, goods conservation, no negative
      balances, zero-sum trade — halting immediately and printing tick, agent and transaction
      on violation
- [ ] Determinism: single seeded RNG, seed recorded with every run, same seed produces a
      byte-identical log, no behaviour-affecting iteration over hash maps, single-threaded
- [ ] Structured machine-readable logging written to disk each tick (per-tick series plus
      per-event records for bankruptcy, hire, fire, cash-out) — sufficient to reconstruct any
      agent's history without re-running
- [ ] Every parameter exposed in a config file; none hardcoded in logic

**Forward compatibility — three constraints so later work is not a rewrite**
- [ ] Goods are data, not code: a goods table with a recipe, even with one good
- [ ] Ownership is a relation, not a field: a firm will later own another firm
- [ ] Decisions carry provenance: record the inputs that drove a price change or hire alongside
      the outcome, from the first tick

**Acceptance**
- [ ] Python acceptance harness reading the sim's log files — conservation audit, unemployment
      band, price-level stability, output autocorrelation, firm-size distribution, and
      seed-reproducibility diff — plus a handful of diagnostic charts
- [ ] A 3,650-tick (10-year) run passing every criterion in the brief's section 7, with the
      first 250 ticks discarded as burn-in

### Out of Scope

Explicitly excluded from this build. Each is planned for later, and scaffolding for them now
would bias the design.

- Banks, credit, interest — later roadmap step
- Government, taxes, public spending — later roadmap step
- Multiple goods, production chains, needs hierarchy — later roadmap step
- Capital equipment, throughput limits, depreciation — later roadmap step
- R&D and technology tiers — later roadmap step
- Births, deaths, aging, traits, inheritance — later roadmap step
- Stock market: listing, index, delisting — later roadmap step
- Geography and foreign trade — later roadmap step
- Endogenous firm founding and liquidation — replaced by immediate respawn in this build
- Graphical interface, dashboard, agent inspector, replay — later roadmap step
- Reusable plotting/stats toolkit — this build ships an acceptance harness only
- Scaling beyond 200 agents — a separate exercise, only once the economics are correct

## Context

**Reference models.** This design follows an established class of agent-based macro models.
Where a mechanism is ambiguous, the published approach is preferred over inventing one.

- Lengnick, *Agent-based macroeconomics: A baseline model* — closest match to this build
- Delli Gatti et al., the BAM model (*Bottom-up Adaptive Macroeconomics*) — decentralised
  labour, goods and credit markets; bankrupt firms replaced by smaller entrants
- Caiani et al., stock-flow-consistent benchmark — the accounting discipline behind the
  invariants

Because these models are published and well-behaved, unrealistic output is to be treated as a
**defect, not a discovery**. If prices spiral or unemployment pins, the bug is most likely in
the price rule, the reservation wage decay, or missing dividends.

**Search frictions are the point.** Households must never see all prices at once and job
seekers must never see all vacancies at once. Perfect information clears the market and every
interesting dynamic disappears.

**The dividend link is load-bearing.** Without profits flowing back to owning households, cash
accumulates inside firms, drains out of households, and the economy deflates into a stall
within a few simulated years. This is the single most common way a first build of this model
dies.

**Scope discipline.** The brief's section 10 lists ten follow-on steps. They are context for
forward-compatibility decisions only — they are not roadmap phases for this milestone and no
scaffolding is built for them.

## Constraints

- **Tech stack**: Rust for the simulation — not for speed at this scale, but because porting a
  tuned agent-based model later is brutal: passing tests is not enough, emergent behaviour has
  to be reproduced, and small numeric or ordering differences change the entire trajectory
- **Tech stack**: Python for analysis and charts, reading the sim's log files — nothing about
  plotting or statistics belongs in the Rust binary
- **Numeric**: integer cents everywhere in money — float money drifts, and drift over thousands
  of ticks silently destroys conservation
- **Architecture**: IDs never references — reaching for `Rc<RefCell<...>>` is the signal the
  design went wrong
- **Determinism**: single-threaded, single seeded RNG; byte-identical logs for a given seed are
  a test, not an aspiration
- **Performance**: a 200-agent decade completes in seconds — this is what makes debugging
  possible, and is the reason not to build for scale yet
- **Configuration**: no parameter hardcoded in logic; all expected to need tuning

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust for sim, Python for analysis | Porting a tuned ABM later requires reproducing emergent behaviour, not just passing tests | — Pending |
| Integer cents for all money | Float drift over thousands of ticks silently breaks conservation | — Pending |
| IDs as vector indices, never inter-agent references | Standard ABM layout in Rust; avoids shared-mutable-state friction | — Pending |
| Invariants checked every tick, halt on violation | A silent assertion failure is worthless; catch the breaking transaction | — Pending |
| Goods modelled as a data table with a recipe | Phase 2 adds a second good; a one-variant enum would have to be torn out | — Pending |
| Ownership modelled as a relation | Later a firm will own another firm | — Pending |
| Provenance recorded from the first tick | Retroactive provenance never covers early history | — Pending |
| Wages contracted at hire, not floating to the firm's current offer | Matches Lengnick; implied by separate `Household.wage` and `Firm.offered_wage` fields | — Pending |
| Household spending: Lengnick's wealth-dependent fraction of cash | Brief specifies the shape but not the parameters; prefer published approach | — Pending |
| Reservation wage rise rate while employed set from Lengnick | Brief's parameter table gives only the decay rate | — Pending |
| Initial ownership: 20 of 200 households hold one firm each | Simplest assignment consistent with "every firm owned by exactly one household" | — Pending |
| Bankruptcy respawn redraws when the sampled owner cannot fund a firm | Brief does not specify the edge case; redraw keeps firm count stable | — Pending |
| Acceptance harness only, no reusable plotting toolkit | Section 7 must be verifiable; a chart suite is roadmap step 3 | — Pending |
| Section 10 items are context, not roadmap phases | User scoped this milestone to the brief alone | — Pending |
| `main` is the integration branch; phase work lands via PR | Each phase gate becomes a reviewable PR | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-08-30 after initialization*
