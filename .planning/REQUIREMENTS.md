# Requirements: Sim — Minimal Closed Economy

**Defined:** 2026-08-30
**Core Value:** The daily tick loop must be provably correct and demonstrably alive — money conserved to the cent, runs byte-identically reproducible, and an economy that fluctuates rather than pinning or spiralling.

**Cadence decision:** the planning cycle is a **21-day month** (Lengnick's period). Every published parameter is therefore used verbatim at its source grade, and no daily/weekly rate conversion is performed anywhere.

## v1 Requirements

### Core Primitives and Determinism

- [x] **CORE-01**: All monetary values use a `Money` newtype over `i64` minor units (cents) with checked arithmetic that panics on overflow regardless of build profile
- [x] **CORE-02**: `[profile.release]` sets `overflow-checks = true` (Cargo defaults it off)
- [x] **CORE-03**: All randomness derives from one master seed via `ChaCha8Rng`, and: **(a)** `StdRng` and `SysRng` are absent from the dependency graph — enforced by the `default-features = false, features = ["std", "chacha"]` feature set on `rand` 0.10.2, and verified by the fact that referencing either does not compile; **(b)** `SmallRng`, the Xoshiro generators and any other non-portable generator are never *used* — enforced by `clippy.toml` `disallowed-types` entries plus a source grep test
  - *Rationale (amended 2026-08-30; authority: `01-CONTEXT.md` D-17).* The original wording required `StdRng` **and** `SmallRng` to be absent from the dependency graph. `rand` 0.10.2 makes `SmallRng` unconditional — `rngs/mod.rs:97-108` re-exports it and the Xoshiro generators with no `cfg` guard — while feature-gating only `StdRng`, behind `#[cfg(feature = "std_rng")]`. This was verified from crate source and by compiling on `rustc 1.94.1` (`01-RESEARCH.md` Pitfall 1): `StdRng` and `rand::rng()` fail to resolve under our feature set, while `SmallRng` still compiles. Absence from the graph is unachievable for `SmallRng` without forking `rand`, so without this amendment the Phase 1 gate reads as failed permanently. Clause (b) bans *use*, which is what the gate was always for — this is a restatement, not a loosening.
- [x] **CORE-04**: RNG draws are namespaced into per-purpose sub-streams keyed on `(master_seed, tick, agent_id, purpose)`, so changing the draw count in one market cannot perturb another
- [x] **CORE-05**: Sampling uses fixed-draw algorithms (partial Fisher-Yates), never rejection sampling
- [x] **CORE-06**: Firm identity is generational (`FirmId { slot, generation }`) and accessors return `Option`, so a stale ID after respawn is a typed miss rather than a silent hit on a different firm
  - *Rationale (amended 2026-08-31; authority: `01-UAT.md` test 1, `01-05-SUMMARY.md` Deviation 1).* This requirement, `01-RESEARCH.md` Pattern 5 and `01-CONTEXT.md` D-03 originally spelled the second field `gen`. `gen` is a reserved keyword in Rust edition 2024 and does not parse as an identifier (verified by compile error; rustc suggests the `r#gen` escape). The spelling is therefore forced by the language, and the requirement text is what moves. Nothing else about the requirement changes: the type shape, the derived total order and the `Option`-returning accessors are as originally written. **Phase 3 writes `(slot, generation)` into the log schema**, so this is the spelling every logged row carries from that point on. Note the unchanged interaction with D-03: the RNG sub-stream key's `agent` field carries the **slot** only, never the generation.
- [x] **CORE-07**: `clippy.toml` bans `HashMap`/`HashSet` on behaviour paths and the 31 non-deterministic `f64` methods, enforced in CI
- [x] **CORE-08**: Crate is `lib.rs` plus a thin `main.rs` so integration tests can reach all code
- [x] **CORE-09**: `Cargo.lock` and `rust-toolchain.toml` are committed; no `rayon` dependency and no `-C target-cpu=native`
- [x] **CORE-10**: Every *simulation or economic* parameter loads from a TOML config with `deny_unknown_fields` and no serde defaults (a serde default is a hidden hardcoded parameter). Carve-out: non-economic numerical-method constants — specifically the fractional-power routine's bit count (`POW_FRAC_BITS`, used by `pow_frac_det`) and the parts-per-million and milli scale factors (`PPM_SCALE`, `MILLI_SCALE`) — are `const` items in `src/numeric.rs`, documented there, and recorded with a `GRADE: PROJECT` entry in `config/PROVENANCE.md` stating why they are not configuration
  - *Rationale (amended 2026-08-30; authority: `01-CONTEXT.md` D-14 and D-18).* Read literally, the original wording pulls a numerical-method iteration count into an economics config — which invites someone to tune it, and tuning it silently changes every trajectory. These constants are therefore documented in `config/PROVENANCE.md` rather than exposed as parameters, so the reason they are fixed is recorded where a reader looks for a parameter's provenance. The two strictness clauses that carry this requirement's weight are unchanged: the config uses `deny_unknown_fields`, and there are no serde defaults. Scope is narrowed to what the gate was for; nothing economic leaves the config.
- [x] **CORE-11**: Source provenance for every config value, in two separately gated clauses. **(a)** Every config value is annotated with its source grade (A/B/C/PROJECT), enforced by an automated test over the shipped config that names any unannotated key — **delivered in Phase 1**. **(b)** The values attributed to the baseline-model paper (Lengnick Table 1) are checked against the published paper by a person with journal access, following the verification procedure shipped alongside them in `config/PROVENANCE.md`, with any discrepancy written down and the config updated with a note rather than silently adopted — **a blocking gate on Phase 6**, the first phase that consumes those values. Until that gate runs, every affected row stands marked `UNVERIFIED` in `config/PROVENANCE.md`
  - *Rationale (amended 2026-08-30; authority: `01-CONTEXT.md` D-19, with D-20 as the standing prohibition; evidence: `01-RESEARCH.md`).* Primary-source access was egress-blocked on every candidate host across two independent research passes, so no agent can close clause (b) — and per D-20 an agent must never transcribe an attributed value from training memory to fake it. Phases 1 through 5 contain no economics and therefore consume none of these values, so gating the project's first phase on an out-of-band human action buys no risk reduction and risks a stall. The clause is **deferred, not dropped**: Phase 1 ships the annotation machinery, the honest `UNVERIFIED` marking and the domain-knowledge-free verification procedure (plan `01-08`); Phase 6, already flagged as the model's widest-sensitivity region, runs the check before consuming the values. CORE-11 remains owned by Phase 1 — what moved is the gate on clause (b), and this is where that move is recorded.

### Ledger and Invariants

- [ ] **LEDG-01**: A central `Books` module owns every cent and every goods unit; `Household` and `Firm` hold no balance fields and expose no `set_cash`
- [ ] **LEDG-02**: `transfer()` is the only cash-mutation point and is atomic — the books are never observable mid-transaction
- [ ] **LEDG-03**: `Money::split` distributes any remainder deterministically, and callers subtract the amount actually transferred
- [ ] **LEDG-04**: Money conservation is checked every tick in release builds against the initial money stock, exactly
- [ ] **LEDG-05**: Goods conservation is checked every tick: produced minus consumed equals inventory
- [x] **LEDG-06**: Non-negativity is checked every tick across cash, inventory and headcount
- [x] **LEDG-07**: Zero-sum trade is checked: every sale moves units one way and equal cash the other
- [x] **LEDG-08**: A liveness invariant asserts transactions-per-tick is greater than zero, closing the "money conserves because nothing trades" degenerate pass
- [x] **LEDG-09**: On violation the sim halts immediately and prints the tick, the agent and the offending transaction, localised by a linear scan of the per-tick journal buffer for the first non-conserving posting
  - *Rationale (amended 2026-08-31; authority: 02-RESEARCH.md; evidence: broken #50 / healed #120 / broken #200 — bisect answers 200, linear scan answers 50).* The superseded search assumes the running residual has a monotone onset — that once the books go wrong they stay wrong, so the first bad posting can be found by halving. It does not: a dropped cent healed later by an equal over-credit returns the residual to zero, and the search then names a later, unrelated posting as the offender. The measured counterexample above is exactly that shape. The linear scan was measured at 80 ns per tick over 274 postings, less than the conservation recompute it accompanies, so nothing is traded away for the correctness. Plans 02-02, 02-03 and 02-04 each make the superseded spelling a hard grep failure in `src/invariants.rs`, which is why the requirement text is what moves. This is a correction of the mechanism and not a loosening of the gate: the requirement still demands the offending transaction be named, and naming it correctly is strictly stronger than naming it plausibly.
- [ ] **LEDG-10**: Invariants are a real pipeline phase returning `Result`, never `debug_assert!`, and a negative test proves a deliberately seeded leak actually halts the run

### Tick Pipeline and Logging

- [ ] **TICK-01**: The tick is a fixed `const PHASES` table running the brief's phases in order, each completing for all agents before the next begins
- [ ] **TICK-02**: The log schema is generated and committed as `schema/schema.json`, which Python reads; schema drift is a test failure
- [ ] **TICK-03**: A per-tick series is written to `ticks.csv` with all money as integer `*_cents` columns
- [ ] **TICK-04**: A per-event stream is written to `events.jsonl` covering bankruptcy, hire, fire and dividend, sufficient to reconstruct any agent's history without re-running
- [ ] **TICK-05**: `run_meta.json` carries seed, config hash and toolchain version, held separate from the diffed logs
- [ ] **TICK-06**: Diffed logs contain no wall-clock time, hostname, path or PID
- [ ] **TICK-07**: Decision provenance is recorded as a joinable flat table (tick, agent, decision type, inputs, outcome), never free text
- [ ] **TICK-08**: 3,650 empty ticks execute and two runs diff byte-identically before any economic rule exists
- [ ] **TICK-09**: The same seed produces byte-identical logs, verified both in-process and cross-process
- [ ] **TICK-10**: A different seed produces different logs, guarding against an accidentally constant RNG

### Python Acceptance Harness

- [ ] **HARN-01**: The harness is a pytest suite in which each section-7 criterion is a named pass/fail test
- [ ] **HARN-02**: A conservation audit replays the event stream and confirms exact `int64` equality, asserting column dtypes so the check cannot silently degrade to a float tolerance
- [ ] **HARN-03**: The unemployment band test excludes bankruptcy churn from its variance computation
- [ ] **HARN-04**: A price stability test verifies no collapse to zero and no runaway, reporting `fraction_at_floor` and price CV
- [ ] **HARN-05**: An output autocorrelation test confirms good and bad stretches cluster, and separately checks for an artefact spike at the planning-cadence lag
- [ ] **HARN-06**: Firm size inequality is reported via HHI, max share and size Gini, confirming no single firm captures the market by year 10
- [ ] **HARN-07**: The seed reproducibility diff runs as an automated test, not a manual step
- [ ] **HARN-08**: Burn-in sensitivity is checked at 2x and 4x the nominal length

### Goods and Production

- [ ] **PROD-01**: Goods are entries in a goods table with a recipe, not an enum variant
- [ ] **PROD-02**: Production adds `workers * productivity` to inventory each tick, at productivity 3 units per worker-day
- [ ] **PROD-03**: All goods movements pass through the ledger so the goods identity holds by construction

### Labour Market

- [ ] **LABR-01**: Firms post vacancies from BAM's labour demand rule `L_d = expected_demand / productivity`, `V = max(L_d - L, 0)`
- [ ] **LABR-02**: Firms with excess workers fire the surplus
- [ ] **LABR-03**: Each unemployed household samples 5 vacancy-posting firms and accepts the highest wage at or above its reservation wage
- [ ] **LABR-04**: Employed households search with probability 0.1, sampling 1 firm
- [ ] **LABR-05**: Wages are contracted at hire and fixed for the contract; contracts are indefinite, ending only on quit, fire or bankruptcy
- [ ] **LABR-06**: The reservation wage ratchets to `max(current, wage_received)` while employed
- [ ] **LABR-07**: The reservation wage decays x0.9 per 21-day month while unemployed, with a positive floor
- [ ] **LABR-08**: Firms pay each worker the contracted wage; a firm that cannot cover payroll pays what it can and fires those it cannot afford
- [ ] **LABR-09**: Every comparator over agents is tie-broken by agent ID, since `sort_unstable` tie order is unspecified

### Goods Market and Consumption

- [ ] **MKT-01**: The household spending budget is `(m / P_bar)^0.9`, where `P_bar` is the mean over the household's own supplier list, not a global price index
- [ ] **MKT-02**: Households never spend to zero
- [ ] **MKT-03**: Each household samples 5 firms and buys cheapest-first, falling through to the next-cheapest on stockout
- [ ] **MKT-04**: Households keep a preferred supplier list of 7 with switch threshold 0.01, price-search probability 0.25 and rationing-search probability 0.25
- [ ] **MKT-05**: Households never observe all prices at once — search friction is preserved as a hard property
- [ ] **MKT-06**: Consumption is an explicit modelled step so the goods identity keeps one shape whether or not households hold stock

### Ownership, Accounting and Dividends

- [ ] **OWN-01**: Ownership is a queryable relation traversable in both directions, not a field, so a firm can later own a firm
- [ ] **OWN-02**: At initialisation 20 of the 200 households each own exactly one firm
- [ ] **OWN-03**: Revenue is booked to the firm through the ledger
- [ ] **OWN-04**: `dividend = max(0, firm_cash - chi * recent_payroll)` with chi = 0.1 of monthly payroll, paying out the full excess rather than a fraction
- [ ] **OWN-05**: Dividends are paid every planning cycle and strictly before the bankruptcy check
- [ ] **OWN-06**: The dividend subtraction uses the amount actually transferred, leaving any rounding residue with the firm
- [ ] **OWN-07**: `firm_cash / total_money` is logged every tick as the deflationary-stall early-warning signal

### Firm Planning

- [ ] **PLAN-01**: Expected demand updates as `E += 0.25 * (observed - E)`
- [ ] **PLAN-02**: Firms replan on a 21-day month cadence, with per-firm offsets drawn once at initialisation from the seeded RNG rather than derived from the slot index
- [ ] **PLAN-03**: Price changes are gated by an inaction probability of theta = 0.75
- [ ] **PLAN-04**: Price adjusts by `U(0, 0.02)` when inventory falls outside the band [0.25, 1.0] of expected demand
- [ ] **PLAN-05**: Price is bounded to [1.025, 1.15] x marginal cost, where `mc = wage / (productivity * 21)`, computed from the previous period with an explicit zero-output fallback
- [ ] **PLAN-06**: A firm at the price ceiling with low inventory hires rather than raising price, providing the goods-to-labour channel
- [ ] **PLAN-07**: Wages adjust by `U(0, 0.019)`; a cut requires gamma = 24 consecutive fully-staffed planning cycles
- [ ] **PLAN-08**: Offered wages are floored and never fall below it
- [ ] **PLAN-09**: Adjustment asymmetry lives in the trigger only, never in the step magnitude
- [ ] **PLAN-10**: `fraction_at_floor` is logged every tick as the price-rule inertness detector

### Bankruptcy and Entry

- [ ] **BANK-01**: A firm is insolvent when net worth is at or below zero, or output is at or below zero
- [ ] **BANK-02**: An insolvent firm releases all workers and transfers residual cash to its owner
- [ ] **BANK-03**: Respawn happens in place in the arena; `Vec::swap_remove` is never used on agent collections
- [ ] **BANK-04**: The entrant is sized at 0.8x a trimmed mean of incumbents, trimming a fixed one firm per tail rather than a percentage at 20 firms
- [ ] **BANK-05**: The entrant is priced at 1.26x the market average
- [ ] **BANK-06**: The entrant is funded from the owning household's cash, redrawing the owner if unaffordable, so conservation holds
- [ ] **BANK-07**: Firm count remains stable across the run

### Calibration and Acceptance

- [ ] **CAL-01**: All initial conditions are chosen and justified — household liquidity, firm liquidity, initial price, initial wage, initial reservation wage, initial inventory, and initial expected demand which must be strictly positive
- [ ] **CAL-02**: The total money stock is chosen through explicit exploration, being the free parameter that decides inflation versus deflation
- [ ] **CAL-03**: Burn-in length is justified by a stationarity check rather than assumed at 250 ticks
- [ ] **CAL-04**: The calibration is validated at 200 households / 20 firms, with small-N effects addressed
- [ ] **CAL-05**: A 3,650-tick run passes every hard-failure and behavioural criterion in the brief's section 7 — all of them, not most
- [ ] **CAL-06**: The acceptance run is reproducible from the committed config and seed

## v2 Requirements

Deferred. These are the brief's section 10 roadmap, tracked as future milestones rather than current scope. Each gets its own milestone and gate.

- Second good, so firms buy from firms
- Capital equipment with throughput limits and depreciation
- Dashboard, agent inspector, replay
- Endogenous firm founding and liquidation
- Multiple consumer goods with a needs hierarchy
- Government: spending creates money, taxation destroys it
- Banks, lending, default
- Demographics: birth, aging, retirement, death, inheritance, traits
- Stock market: listing, index, delisting
- R&D and technology tiers

## Out of Scope

| Feature | Reason |
|---------|--------|
| Banks, credit, interest | Later milestone; scaffolding now would bias the design |
| Government, taxes, public spending | Later milestone |
| Multiple goods, production chains | Later milestone; the goods table keeps the seam open |
| Capital equipment, depreciation | Later milestone |
| R&D, technology tiers | Later milestone |
| Births, deaths, aging, inheritance | Later milestone |
| Stock market | Later milestone; single-owner firms are its seed |
| Geography, foreign trade | Later milestone |
| Endogenous firm founding | Replaced by immediate respawn in this build |
| Graphical interface, dashboard, replay | Later milestone; this build ships an acceptance harness only |
| Reusable plotting/stats toolkit | Acceptance harness only, per project decision |
| Scaling beyond 200 agents | Separate exercise, only once the economics are correct |
| Market-clearing or Walrasian auctioneer | Destroys the disequilibrium dynamics that are the object of study |
| Perfect information over prices or vacancies | Search friction is the mechanism, not an imperfection |
| Multi-threading | Determinism requires single-threaded execution for now |
| Lengnick Eq(12) consumption bound | Text unrecovered and reported near-vacuous; omitted rather than invented |

## Traceability

Every v1 requirement maps to exactly one phase in ROADMAP.md. No orphans, no duplicates.

**Phase names:**

| Phase | Name |
|-------|------|
| Phase 1 | Primitives and the Determinism Spine |
| Phase 2 | Books, Journal and Invariants |
| Phase 3 | World, Tick Pipeline and Log Seam |
| Phase 4 | Python Acceptance Harness Skeleton |
| Phase 5 | Goods, Recipes and Production |
| Phase 6 | Labour Market, Wages and Reservation Wages |
| Phase 7 | Goods Market and Consumption |
| Phase 8 | Ownership, Accounting and Dividends |
| Phase 9 | Firm Planning |
| Phase 10 | Bankruptcy and Respawn |
| Phase 11 | Calibration, Burn-in and Full Acceptance |

**Requirement mapping:**

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 1 | Complete |
| CORE-02 | Phase 1 | Complete |
| CORE-03 | Phase 1 | Complete |
| CORE-04 | Phase 1 | Complete |
| CORE-05 | Phase 1 | Complete |
| CORE-06 | Phase 1 | Complete |
| CORE-07 | Phase 1 | Complete |
| CORE-08 | Phase 1 | Complete |
| CORE-09 | Phase 1 | Complete |
| CORE-10 | Phase 1 | Complete |
| CORE-11 | Phase 1 | Complete |
| LEDG-01 | Phase 2 | Pending |
| LEDG-02 | Phase 2 | Pending |
| LEDG-03 | Phase 2 | Pending |
| LEDG-04 | Phase 2 | Pending |
| LEDG-05 | Phase 2 | Pending |
| LEDG-06 | Phase 2 | Complete |
| LEDG-07 | Phase 2 | Complete |
| LEDG-08 | Phase 2 | Complete |
| LEDG-09 | Phase 2 | Complete |
| LEDG-10 | Phase 2 | Pending |
| TICK-01 | Phase 3 | Pending |
| TICK-02 | Phase 3 | Pending |
| TICK-03 | Phase 3 | Pending |
| TICK-04 | Phase 3 | Pending |
| TICK-05 | Phase 3 | Pending |
| TICK-06 | Phase 3 | Pending |
| TICK-07 | Phase 3 | Pending |
| TICK-08 | Phase 3 | Pending |
| TICK-09 | Phase 3 | Pending |
| TICK-10 | Phase 3 | Pending |
| HARN-01 | Phase 4 | Pending |
| HARN-02 | Phase 4 | Pending |
| HARN-03 | Phase 11 | Pending |
| HARN-04 | Phase 9 | Pending |
| HARN-05 | Phase 9 | Pending |
| HARN-06 | Phase 10 | Pending |
| HARN-07 | Phase 4 | Pending |
| HARN-08 | Phase 11 | Pending |
| PROD-01 | Phase 5 | Pending |
| PROD-02 | Phase 5 | Pending |
| PROD-03 | Phase 5 | Pending |
| LABR-01 | Phase 6 | Pending |
| LABR-02 | Phase 6 | Pending |
| LABR-03 | Phase 6 | Pending |
| LABR-04 | Phase 6 | Pending |
| LABR-05 | Phase 6 | Pending |
| LABR-06 | Phase 6 | Pending |
| LABR-07 | Phase 6 | Pending |
| LABR-08 | Phase 6 | Pending |
| LABR-09 | Phase 6 | Pending |
| MKT-01 | Phase 7 | Pending |
| MKT-02 | Phase 7 | Pending |
| MKT-03 | Phase 7 | Pending |
| MKT-04 | Phase 7 | Pending |
| MKT-05 | Phase 7 | Pending |
| MKT-06 | Phase 7 | Pending |
| OWN-01 | Phase 8 | Pending |
| OWN-02 | Phase 8 | Pending |
| OWN-03 | Phase 8 | Pending |
| OWN-04 | Phase 8 | Pending |
| OWN-05 | Phase 8 | Pending |
| OWN-06 | Phase 8 | Pending |
| OWN-07 | Phase 8 | Pending |
| PLAN-01 | Phase 9 | Pending |
| PLAN-02 | Phase 9 | Pending |
| PLAN-03 | Phase 9 | Pending |
| PLAN-04 | Phase 9 | Pending |
| PLAN-05 | Phase 9 | Pending |
| PLAN-06 | Phase 9 | Pending |
| PLAN-07 | Phase 9 | Pending |
| PLAN-08 | Phase 9 | Pending |
| PLAN-09 | Phase 9 | Pending |
| PLAN-10 | Phase 9 | Pending |
| BANK-01 | Phase 10 | Pending |
| BANK-02 | Phase 10 | Pending |
| BANK-03 | Phase 10 | Pending |
| BANK-04 | Phase 10 | Pending |
| BANK-05 | Phase 10 | Pending |
| BANK-06 | Phase 10 | Pending |
| BANK-07 | Phase 10 | Pending |
| CAL-01 | Phase 11 | Pending |
| CAL-02 | Phase 11 | Pending |
| CAL-03 | Phase 11 | Pending |
| CAL-04 | Phase 11 | Pending |
| CAL-05 | Phase 11 | Pending |
| CAL-06 | Phase 11 | Pending |

**Cross-category placements** (requirement placed outside the phase its ID prefix suggests):

| Requirement | Phase | Reason |
|-------------|-------|--------|
| HARN-03 | Phase 11 | Excluding bankruptcy churn from the unemployment-band variance needs the churn introduced in Phase 10; the band itself is a section-7 acceptance criterion. |
| HARN-04 | Phase 9 | Price stability, `fraction_at_floor` and price CV *are* Phase 9's gate — without them the price-rule check is an eyeballed chart. |
| HARN-05 | Phase 9 | Output autocorrelation and the planning-cadence artefact spike *are* Phase 9's gate. |
| HARN-06 | Phase 10 | Phase 10 is the first point at which the firm-size distribution's generating process is complete (goods market plus entrant sizing). |
| HARN-08 | Phase 11 | Burn-in sensitivity at 2x and 4x is inseparable from CAL-03. |

These follow the research constraint that the harness grows alongside the sim so every phase gate is an automated check. HARN-01, HARN-02 and HARN-07 remain in Phase 4, being the seam and counter-checks buildable against the pre-economics empty run.

**Coverage:**

- v1 requirements: 87 total
- Mapped to phases: 87
- Unmapped: 0

---
*Requirements defined: 2026-08-30*
*Last updated: 2026-08-30 after roadmap creation — traceability populated, 87/87 mapped*
