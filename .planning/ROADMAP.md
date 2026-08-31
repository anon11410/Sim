# Roadmap: Sim — Minimal Closed Economy

## Overview

Eleven phases build a provably-correct daily tick loop for a 200-household / 20-firm closed
economy, then bring it to life. The first four phases contain **zero economics by design**:
money primitives and the determinism spine, then the ledger and its invariants, then the tick
pipeline and log seam, then an independent Python harness that can prove the sim wrong. Only
once every cent is owned by one ledger, every tick is checked, and every run is diffable does
the first economic rule appear — so each rule is *born* under the check rather than retrofitted
into it.

The economics then arrive in **money-movement dependency order**: goods exist (5), money moves
firm→household via wages (6), household→firm via purchases (7), and the loop closes via
dividends (8). Firm planning is built ninth even though it runs *first* in the tick — **tick
order is not build order**, and a reaction rule cannot be tuned before there is something to
react to. Bankruptcy is last among the economics because it consumes the generational arena,
worker release and the ownership relation from three earlier phases. Calibration and full
section-7 acceptance close the milestone as a real phase, because every initial condition and
the total money stock are genuinely unspecified in the literature.

The Python harness grows alongside the sim rather than arriving at the end, so every phase gate
is an automated check rather than an eyeballed chart. Cadence throughout is a **21-day month**;
every published parameter is used verbatim at source grade with no daily or weekly rate
conversion anywhere.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Primitives and the Determinism Spine** - Money, IDs, config and seeded randomness with the properties that cannot be retrofitted (completed 2026-08-31)
- [x] **Phase 2: Books, Journal and Invariants** - One ledger owns every cent and every unit; five checks halt the run on the offending posting (completed 2026-08-31)
- [ ] **Phase 3: World, Tick Pipeline and Log Seam** - 3,650 empty ticks run and two seeds diff byte-identically before any economics exist
- [ ] **Phase 4: Python Acceptance Harness Skeleton** - An independent harness that reads the run directory and can prove the sim wrong
- [ ] **Phase 5: Goods, Recipes and Production** - Goods as a data table with a recipe, produced into a conserved stock
- [ ] **Phase 6: Labour Market, Wages and Reservation Wages** - Money moves firm→household through search friction that generates involuntary unemployment
- [ ] **Phase 7: Goods Market and Consumption** - Money moves household→firm through bounded sampling and cheapest-first purchase
- [ ] **Phase 8: Ownership, Accounting and Dividends** - The cycle-closing flow that keeps the economy from stalling
- [ ] **Phase 9: Firm Planning** - Expectations, price rule and wage rule react without spiralling, collapsing or producing a schedule artefact
- [ ] **Phase 10: Bankruptcy and Respawn** - Firms die and are replaced without breaking conservation, identity or the size distribution
- [ ] **Phase 11: Calibration, Burn-in and Full Acceptance** - A committed config and seed pass every section-7 criterion, all of them

## Phase Details

### Phase 1: Primitives and the Determinism Spine

**Goal**: The project's vocabulary — money, identity, configuration and randomness — exists with the correctness properties that every later phase depends on and that no later phase can add cheaply.
**Depends on**: Nothing (first phase)
**Requirements**: CORE-01, CORE-02, CORE-03, CORE-04, CORE-05, CORE-06, CORE-07, CORE-08, CORE-09, CORE-10, CORE-11
**Success Criteria** (what must be TRUE):

  1. `Money` arithmetic panics on overflow in **both** debug and release profiles, and `Money::split(n)` sums exactly to the original amount — property-tested over amounts that do **not** divide evenly, so a remainder-dropping implementation fails rather than passing on round numbers.
  2. Two constructions of the RNG from the same master seed produce identical `u64` streams; sub-streams keyed on different `(tick, agent_id, purpose)` tuples produce **different** streams, so an added draw in one market provably cannot perturb another. Counter-check: a different master seed produces a different stream, so a constant RNG cannot pass.
  3. A TOML config with an unknown key, a missing key, or a removed value fails to load with a named error; `grep` finds no `#[serde(default)]` anywhere; the config hash is reproducible across runs.
  4. `cargo clippy` **fails the build** when code introduces a `HashMap`/`HashSet` on a behaviour path or calls one of the 31 non-deterministic `f64` methods; `Cargo.toml` contains no `rayon` and `rust-toolchain.toml` and `Cargo.lock` are committed.
  5. Every config value carries a source-grade annotation (A/B/C/PROJECT), asserted by the annotation test — which names any unannotated key — rather than by review. The second half of CORE-11, that the Lengnick Table 1 values are checked against the published paper with any discrepancy recorded rather than silently adopted, is **gated on Phase 6 per D-19** — primary-source access is egress-blocked here and no phase before 6 consumes the values; until that gate runs the affected rows stand marked `UNVERIFIED` in `config/PROVENANCE.md`. Deferred, not dropped: Phase 1 ships the machinery, the honest marking and the verification procedure.

**Plans**: 8/8 plans executed in 4 waves
Plans:
**Wave 1**

- [x] 01-01-PLAN.md — Tracer: the crate spine end to end (config → hash → effective seed → sub-stream draw → Money), plus the committed toolchain pin, lockfile and release overflow check
- [x] 01-02-PLAN.md — Amend CORE-03 into two testable clauses, scope CORE-10, split CORE-11 and Phase 1 criterion 5 so the paper-verification clause is visibly gated on Phase 6 rather than silently graded here, and correct the rand 0.10 small-RNG claim in CLAUDE.md

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-03-PLAN.md — Money: panicking operators, the named Result API, and a split that conserves every cent under property test
- [x] 01-04-PLAN.md — RNG sub-streams: the bit-packed key, the Purpose enum, the re-entry guard and the fixed-draw samplers
- [x] 01-05-PLAN.md — Generational FirmId with an in-place-respawn arena, and the confined float domain with a deterministic fractional power
- [x] 01-06-PLAN.md — Config strictness: the full parameter schema, deny-unknown on every struct, and the exhaustive missing-key proof

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 01-08-PLAN.md — Source-grade provenance, the UNVERIFIED rows and the Phase 6 verification procedure

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 01-07-PLAN.md — The determinism lint wall: generated ban lists, a negative test that proves they block, and CI

**Research**: light flag — RNG sub-stream keying scheme, and `f64` vs `i64` milli-units for `expected_demand`. Both cheap to research now, expensive to change later. Both resolved in `01-RESEARCH.md` and locked as CONTEXT.md D-01/D-02 and D-11/D-12.

### Phase 2: Books, Journal and Invariants

**Goal**: A single ledger owns every cent and every goods unit, and the invariant checks halt the run on the tick a violation occurs, naming the offending posting.
**Depends on**: Phase 1
**Requirements**: LEDG-01, LEDG-02, LEDG-03, LEDG-04, LEDG-05, LEDG-06, LEDG-07, LEDG-08, LEDG-09, LEDG-10
**Success Criteria** (what must be TRUE):

  1. No type outside `books` can move value: `Household` and `Firm` carry no balance fields and expose no `set_cash`, `transfer()` is the only cash-mutation point, and it is atomic — a test observing the books mid-transaction is impossible to write.
  2. **The negative test passes for every check**: a deliberately seeded leak — a dropped cent, an over-credited sale, a driven-negative balance, a non-zero-sum trade — halts the run and prints the tick, the agent and the offending posting, localised by a linear scan of the per-tick journal for the first non-conserving posting. An invariant never observed to fire has never been shown to work. *(Localisation clause amended 2026-08-31; authority: `02-RESEARCH.md`, restated in `02-CONTEXT.md`. It previously read "halving the journal"; a search over halves assumes the running residual has a monotone onset, which it does not — measured counterexample: a journal broken at posting 50, healed at 120 and broken again at 200, where halving answers 200 and the correct answer is 50.)*
  3. The liveness invariant halts a build in which a tick records zero transactions, closing the "money conserves because nothing trades" degenerate pass. It is config-gated **off** for Phase 3's pre-economics empty run and **on** by default from Phase 6 onward.
  4. Invariants run in **release** builds as a real pipeline phase returning `Result` — `grep` proves no `debug_assert!` on the invariant path, and a release binary with a seeded violation still halts.

**Plans**: 7 plans

Plans:

- [x] 02-01-PLAN.md — The liveness config gate, `Display` for every ledger address, and the source amendments: criterion 2's localisation clause, LEDG-09, and the Phase 3 / Phase 6 cross-phase criteria
- [x] 02-02-PLAN.md — TRACER (one slice, split at the module seam for a resume boundary): `Books`, the journal and the check set, proved by a tick loop that halts on a tick that traded nothing
- [x] 02-03-PLAN.md — Goods: production, consumption, the cash-for-units swap, and the one-shape goods identity
- [x] 02-04-PLAN.md — Non-negativity, zero-sum, headcount in the books, and the check-order contract
- [x] 02-05-PLAN.md — The negative tests: four seeded corruptions, the halting tick loop, localisation across a cancelling residual
- [x] 02-06-PLAN.md — LEDG-02's four legs: panic-atomicity, two compile-fail probes, eight source guards, the lint entries
- [x] 02-07-PLAN.md — Properties: conservation, transfer-return agreement, the goods identity, the two-source agreement

### Phase 3: World, Tick Pipeline and Log Seam

**Goal**: A run executes the full fixed tick order and writes a complete, diffable run directory — with the log schema and provenance table in place before any economic rule can be written outside them.
**Depends on**: Phase 2
**Requirements**: TICK-01, TICK-02, TICK-03, TICK-04, TICK-05, TICK-06, TICK-07, TICK-08, TICK-09, TICK-10
**Success Criteria** (what must be TRUE):

  1. **3,650 empty ticks execute end to end**, invariants pass, and a run directory is produced containing `ticks.csv` (money as integer `*_cents` columns), `events.jsonl`, `run_meta.json` and the generated, committed `schema/schema.json`.
  2. Two runs at the same seed produce **byte-identical** `ticks.csv` and `events.jsonl`, verified both in-process and cross-process; `run_meta.json` holds seed, config hash and toolchain and is excluded from the diff; no wall-clock, hostname, path or PID appears in any diffed file.
  3. Counter-check against the vacuous-reproducibility pass: two runs at **different** seeds produce different logs, because the empty pipeline consumes at least one RNG draw per tick (activation-order shuffle plus a per-tick draw-count column). Reproducibility cannot pass by never consuming the RNG.
  4. A test asserts the exact name sequence of the `const PHASES` table and fails on reorder; a second test fails on schema drift between the generated schema and the committed one.
  5. Decision provenance exists as a joinable flat table (tick, agent, decision type, inputs, outcome) — empty at this phase but present, schema-validated and never free text.
  6. **The process-level half of Phase 2 criterion 2.** The built binary, run against the shipped configuration with `invariants.liveness_enabled` overridden to `true` against an economy that trades nothing, exits non-zero with a stderr line naming tick 0. Phase 2 can only prove the halt at the library level — a tick loop that aborts at the right tick — because the `const PHASES` table and the binary's tick loop are this phase's, so the process level belongs to neither phase unless it is written here. **This does not contradict criterion 1**: criterion 1 exercises the same binary with the **shipped** value `false`, which is why 3,650 empty ticks pass there, and this criterion overrides that one key and nothing else. Cheap by construction — the liveness violation is the one violation reachable through the public ledger API, so it needs no fault injection at all, and the pipeline it drives already exists by this point. Cross-references Phase 2 criterion 2.
  7. **The balance-field obligation inherited from Phase 2 criterion 1.** `Household` and `Firm` first exist in this phase, so this is the first phase that can verify the half of LEDG-01 that is a claim about them: they carry no balance field, expose no `set_cash`, and every cent they appear to hold is read through the ledger accessors. Guard 7f in `tests/lints.sh` is extended to name those two types in the same commit that introduces them, so the guard cannot lag the types it polices. Phase 2 can verify only the positive property about where a balance may be written, which is why this half is recorded against this phase rather than left inside a lint failure string. Cross-references Phase 2 criterion 1.

**Plans**: 6 plans

Plans:

- [ ] 03-01-PLAN.md — Amend criterion 3 to the counter-check that was measured to work, and add the log seam's four crates with the two rejected ones kept out of the lockfile
- [ ] 03-02-PLAN.md — TRACER: `World`, the fixed nine-phase `PHASES` table and the tick log, one decade of empty ticks from `--config` to a diffable `ticks.csv`, with guard `7f-agents` added in the same commit as `Household` and `Firm`
- [ ] 03-03-PLAN.md — The event stream opened by the endowment, the provenance table with its eager header, and `run_meta.json` with the build-time compiler string
- [ ] 03-04-PLAN.md — The schema generated from the writers rather than a second derive, committed, and its drift test proved to fire
- [ ] 03-05-PLAN.md — The determinism suite: same-seed in-process and cross-process, different-seed, the enforced exclusion, and the process-level halt at tick 0
- [ ] 03-06-PLAN.md — The committed 50-tick golden run and guard 7h extended to every module that can render a halt message

**Note**: If Phase 7 resolves consumption as its own pipeline step, the `PHASES` table and its order test are extended in that same commit.

### Phase 4: Python Acceptance Harness Skeleton

**Goal**: An independent Python harness reads the run directory across the disk boundary and can demonstrably prove the sim wrong — built against the empty run, before there is any economics to flatter it.
**Depends on**: Phase 3
**Requirements**: HARN-01, HARN-02, HARN-07
**Success Criteria** (what must be TRUE):

  1. `pytest` runs against a run directory via a `--run-dir` fixture, with **every** section-7 criterion present as a named test; the behavioural ones skip with an explicit "awaiting phase N" reason rather than silently not existing.
  2. The conservation audit **replays the event stream from the initial endowment** — a second, independent derivation, never a re-read of the sim's own aggregate — and asserts exact `int64` equality, with dtype assertions on every `*_cents` column so the check cannot silently degrade to a float tolerance.
  3. **The harness fails loudly on a hand-corrupted log**: a flipped cent, a dropped event, a float-typed money column and a schema-drifted header each produce a distinct named failure. A harness that has never failed has never been shown to work.
  4. The seed-reproducibility diff runs as an automated test (not a manual step), the different-seed-differs mutation test runs alongside it, and a committed `uv.lock` pins the Python environment.

**Plans**: TBD
**Departure from category alignment**: HARN-03, HARN-04, HARN-05, HARN-06 and HARN-08 are **not** in this phase. Each is a statistical test over a live economy that cannot be written meaningfully against an empty run, so each is mapped to the phase whose gate it converts from an eyeballed chart into an automated check (see Phases 9, 10 and 11). This is the research's "the harness grows alongside the sim" constraint applied at requirement granularity.

### Phase 5: Goods, Recipes and Production

**Goal**: Goods exist as data with a recipe rather than as an enum variant, and are produced into a stock conserved by the ledger.
**Depends on**: Phase 3
**Requirements**: PROD-01, PROD-02, PROD-03
**Success Criteria** (what must be TRUE):

  1. Over 3,650 ticks the goods identity holds exactly every tick: `produced − consumed − Σ inventory == 0`, checked by the ledger, not by the production code that would be asserting its own arithmetic.
  2. Output equals `productivity × headcount` at productivity 3, with headcount read **at the production step** rather than at tick start — the ordering distinction that makes the identity meaningful once hiring exists.
  3. A test fixture adds a second good to the goods table by config alone and runs without touching production code, proving the forward-compatibility mandate. No v1 behaviour changes: the shipped config still has one good.
  4. No code path outside `books` adds to or removes from inventory — goods movement is as tightly held as cash movement.

**Plans**: TBD

### Phase 6: Labour Market, Wages and Reservation Wages

**Goal**: Money moves firm→household through a frictional labour market in which involuntary unemployment is generated by the search friction rather than assumed.
**Depends on**: Phase 5
**Requirements**: LABR-01, LABR-02, LABR-03, LABR-04, LABR-05, LABR-06, LABR-07, LABR-08, LABR-09
**Success Criteria** (what must be TRUE):

  1. Money conserves exactly across 3,650 ticks of hiring, firing and paying, and unemployment is strictly between 0% and 100% throughout. Counter-check against "fluctuates" passing on a flat line: a variance floor and a max-run-length-of-identical-values limit, not merely a mean inside a band.
  2. No household ever observes more than its sample — 5 vacancy-posting firms for the unemployed, 1 for an employed household searching with probability 0.1 — enforced by construction and asserted by a test, because perfect information silently clears the market.
  3. A hired household's wage is fixed for the life of its contract and changes only on quit, fire or bankruptcy; a firm short of payroll pays what it can and fires those it cannot afford, and no balance ever goes negative.
  4. Wealth rank correlates ≈ 0 with agent ID after 3,650 ticks, proving the per-tick seeded reshuffle of job-seeker and worker order is doing its job and no spurious ID-monotone distribution has formed.
  5. Reservation wages ratchet to `max(current, wage_received)` while employed and decay ×0.9 per 21-day month while unemployed, never falling below a positive floor; every comparator over agents is tie-broken by agent ID.
  6. **CORE-11 clause (b), gated here per D-19**: before these values are consumed, the Lengnick Table 1 rows marked `UNVERIFIED` in `config/PROVENANCE.md` have been checked against the published paper by a person with journal access, following the procedure shipped in Phase 1 — each row recorded as agrees / differs (with the paper's value written down) / not in Table 1, and any discrepancy written down and the config updated with a note rather than silently adopted. **If BAM is also to hand on that pass, check the two BAM rows (`config/PROVENANCE.md` open items V-3 and V-3a) at the same time** — they are not in Table 1 and so are not gated here, but this is the phase where someone has journal access, and V-3a is a known value/source contradiction whose blocking gate sits in Phase 10 criterion 6.
  7. **The liveness gate flips on here.** `invariants.liveness_enabled` is `true` in `config/baseline.toml` by the end of this phase, flipped in the commit that first makes wages move money, with the `config/PROVENANCE.md` row updated to match the new value in that same commit. The three-file agreement between the shipped config, its annotation and its provenance row is policed by `no_annotation_is_orphaned` and `every_config_key_has_a_provenance_row`, so a half-flip is a red test rather than a silent drift. Phase 2 criterion 3 is the constraint's origin — off for Phase 3's pre-economics empty run, on by default from this phase — and this is the flip that makes LEDG-08 load-bearing for every phase after it, because from that point a tick recording no transaction is a halt rather than a permitted state.

**Plans**: TBD
**Research**: **flagged** — the reservation-wage / wage-step coupling is the widest-sensitivity parameter region in the model, both error signs produce plausible-looking pathologies, and the Lengnick values are grade B rather than read from the paper.
**Note**: `expected_demand` is a static initial value here (`L_d = expected_demand / productivity`); Phase 9 makes it adaptive.

### Phase 7: Goods Market and Consumption

**Goal**: Money moves household→firm through a frictional goods market, closing the outbound leg of the circular flow with prices and wages deliberately held static.
**Depends on**: Phase 6
**Requirements**: MKT-01, MKT-02, MKT-03, MKT-04, MKT-05, MKT-06
**Success Criteria** (what must be TRUE):

  1. A full circular flow exists — wages paid, goods bought — and both conservation invariants hold over 3,650 ticks while prices and wages stay static, isolating market mechanics from adjustment rules.
  2. Households never spend to zero, and never observe all prices: each samples 5 firms from its own persistent supplier list of 7 (switch threshold 0.01, price-search 0.25, rationing-search 0.25) and buys cheapest-first, falling through to the next-cheapest on stockout.
  3. Cross-sectional price dispersion is non-zero and the unmet-demand fraction is small but **non-zero** — the counter-check against a dead steady state in which nothing is ever rationed and the search mechanism is decorative.
  4. Each firm's `last_sales` is derived from that firm's own purchase events, never from an aggregate, and consumption is an explicit modelled step so the goods identity keeps one shape whether or not households hold stock.

**Plans**: TBD

### Phase 8: Ownership, Accounting and Dividends

**Goal**: Profits flow back to owning households, closing the circular flow. This is the only cycle-closing flow in a bankless economy and the only equilibrating force — it ships as one phase and is never split.
**Depends on**: Phase 7
**Requirements**: OWN-01, OWN-02, OWN-03, OWN-04, OWN-05, OWN-06, OWN-07
**Success Criteria** (what must be TRUE):

  1. **A 3,650-tick run does not stall.** `firm_cash / total_money` is logged every tick from this phase's first commit, its regression slope over the post-burn-in window is indistinguishable from zero, and the ratio stays below 0.5 on every tick — the early-warning signal fires one to two years before prices or unemployment would show anything.
  2. Dividends pay the **full excess** above `chi × recent_payroll` every planning cycle and strictly **before** the bankruptcy check, and the amount subtracted is the amount actually transferred — the split is exactly zero-sum after rounding, with any residue left at the firm.
  3. Counter-check that the stall test is not vacuous: a run with dividends disabled by config **must fail** criterion 1. A test that cannot detect the failure mode it exists to detect is theatre.
  4. Ownership is a relation queryable in both directions (household→firms, firm→owner), initialised with 20 of 200 households holding one firm each, and supports a firm owning a firm without a schema change.

**Plans**: TBD
**Research**: **flagged** — this is where the brief is least specified, where the project deviates most from the literature (single-owner firms have no published precedent; all three papers pay pro-rata to the whole population), and where the most damaging failure mode originates. Expect a materially more unequal wealth distribution than any paper reports.

### Phase 9: Firm Planning

**Goal**: Firms react — expectations update, prices and wages adjust — without spiralling, collapsing, or manufacturing a business cycle out of the planning schedule. Built last of the reactive economics because it needs sales history and a non-stalling economy to tune against.
**Depends on**: Phase 8
**Requirements**: PLAN-01, PLAN-02, PLAN-03, PLAN-04, PLAN-05, PLAN-06, PLAN-07, PLAN-08, PLAN-09, PLAN-10, HARN-04, HARN-05
**Success Criteria** (what must be TRUE):

  1. Prices move over 3,650 ticks and neither collapse to zero nor run away. Counter-check against the subtle failure that looks like success: `fraction_at_floor` sits in the low single digits and price CV stays non-zero, so prices are not "admirably stable" merely because every firm is pinned at the floor and cross-sectional search has silently died.
  2. Up and down price adjustment counts are roughly balanced, confirming the asymmetry lives in the trigger only and never in the step magnitude — an asymmetric step gives the random walk positive drift by construction.
  3. Output autocorrelation **decays** rather than merely being positive, and the ACF shows **no spike at the 21-day planning-cadence lag** — the directly computable detector for synchronised replanning masquerading as a business cycle. Per-firm stagger offsets are drawn once at init from the seeded RNG, never from the slot index, and are config-toggleable.
  4. A firm at the 1.15× price ceiling with low inventory **hires instead of raising price**, and the choice is visible in the provenance table — the goods-market-to-labour-market channel the brief otherwise has no path for.
  5. A wage cut requires 24 consecutive fully-staffed **planning cycles** (counted in cycles, not ticks), and offered wages never fall below the floor.

**Plans**: TBD
**Requirement moves**: HARN-04 (price stability, `fraction_at_floor`, price CV) and HARN-05 (output autocorrelation and the planning-cadence artefact spike) are placed here rather than Phase 4, because they *are* this phase's gate — without them criteria 1 and 3 are eyeballed charts.

### Phase 10: Bankruptcy and Respawn

**Goal**: Firms fail and are replaced without breaking conservation, agent identity or the firm-size distribution. Last among the economics because it consumes the generational arena, worker release and the ownership relation from three earlier phases.
**Depends on**: Phase 9
**Requirements**: BANK-01, BANK-02, BANK-03, BANK-04, BANK-05, BANK-06, BANK-07, HARN-06
**Success Criteria** (what must be TRUE):

  1. Firms die (net worth ≤ 0 or output ≤ 0) and respawn across a decade with money conserved exactly every tick; net respawn cash flow is ≈ 0 because entrants are funded from the owning household's cash with a fixed-draw redraw when unaffordable, never created ex nihilo.
  2. **No stale `FirmId` ever resolves**: a lookup at the pre-respawn generation returns `None`, per-`(slot, generation)` log groups are complete and non-overlapping, and `Vec::swap_remove` appears nowhere on an agent collection — respawn happens in place at `generation+1`.
  3. Firm count stays at 20 on every tick and bankruptcies per year fall inside a bounded range — counter-checked at both ends, since zero failures and runaway churn are both defects and only one of them looks like one.
  4. **No single firm captures the market by year 10**: HHI, max share and size Gini are reported by the harness and within bounds, computed over firms above an age threshold so a cohort of young entrants cannot flatter the distribution.
  5. Entrants are sized at `bankruptcy.entrant_size_ratio_ppm` × a trimmed mean of incumbents (a fixed one firm per tail, not a percentage, at 20 firms) and priced at 1.26× the market average — pricing at market makes entrants fail immediately and the bankruptcy rate run away.
  6. **Blocking gate — `config/PROVENANCE.md` open item V-3a is adjudicated before BANK-04 consumes the value.** The key ships `800000` (0.8×) while its own SOURCE field cites BAM `size-replacing-firms` = 0.2. Three non-equivalent readings stand open and the repository does not say which holds: a transcription error (the value should be `200000`); a deliberate `1 − 0.2` derivation (which regrades the row from B to C and puts the derivation in the SOURCE field); or a misread source parameter. Per D-20 this must **not** be resolved from model memory — it needs a BAM read. Criterion 5 above is written against the config key rather than a literal precisely so this phase cannot pass by asserting a number nobody has checked.
     - *Amended 2026-08-31 (authority: `01-UAT.md` test 2, `01-VERIFICATION.md` W1).* Criterion 5 previously read "sized at 0.8×", asserting as settled fact the very value V-3a holds open. Phase 6 criterion 6 does not cover it — that criterion scopes to Lengnick Table 1 rows and this is a BAM row — so before this amendment nothing in the roadmap forced the check.

**Plans**: TBD
**Requirement move**: HARN-06 (firm-size inequality) is placed here rather than Phase 4, because Phase 10 is the first point at which the distribution's generating process is complete — the goods market and the entrant-sizing rule jointly produce it — and it converts this phase's gate from "count stays 20" into a real degeneracy check.

### Phase 11: Calibration, Burn-in and Full Acceptance

**Goal**: A committed config and seed produce a 10-year run that passes every section-7 criterion — all of them, not most — with every degenerate-pass counter-check passing alongside.
**Depends on**: Phase 10
**Requirements**: CAL-01, CAL-02, CAL-03, CAL-04, CAL-05, CAL-06, HARN-03, HARN-08
**Success Criteria** (what must be TRUE):

  1. Every initial condition — household liquidity, firm liquidity, price, wage, reservation wage, inventory, and a **strictly positive** expected demand — is chosen, justified in writing and recorded in the committed config with its source grade, replacing the provisional placeholders earlier phases ran on.
  2. The total money stock is chosen by **documented exploration** showing the runs on either side inflate and deflate — the free parameter that decides the economy's fate is not settled by a single lucky point.
  3. Burn-in length is justified by a stationarity check (half-window mean-and-variance comparison) with conclusions stable at 2× and 4× the nominal length, rather than assumed at the brief's 250 ticks.
  4. **A 3,650-tick run from the committed config and seed passes every hard-failure and behavioural criterion in the brief's section 7**, and passes every counter-check beside it: variance floors, max run-length of identical values, ACF decay and stationarity, and unemployment-band variance computed with bankruptcy churn excluded so the band cannot be satisfied by churn alone.
  5. The acceptance run reproduces byte-identically from the committed config and seed on a second OS/toolchain in CI, and the calibration is shown to hold at 200 households / 20 firms with small-N effects addressed rather than inherited from a 1000/100 calibration.

**Plans**: TBD
**Research**: **flagged** — this phase is a search, not an implementation. All initial conditions and the total money stock are genuine literature gaps, and whether the Lengnick calibration survives at 200/20 is unaddressed anywhere in the sources.
**Requirement moves**: HARN-03 (bankruptcy-excluded unemployment variance) and HARN-08 (burn-in sensitivity at 2× and 4×) are placed here rather than Phase 4 — HARN-03 needs the bankruptcy churn introduced in Phase 10 to have something to exclude, and HARN-08 is inseparable from CAL-03.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Primitives and the Determinism Spine | 8/8 | Complete    | 2026-08-31 |
| 2. Books, Journal and Invariants | 7/7 | Complete    | 2026-08-31 |
| 3. World, Tick Pipeline and Log Seam | 0/6 | Not started | - |
| 4. Python Acceptance Harness Skeleton | 0/TBD | Not started | - |
| 5. Goods, Recipes and Production | 0/TBD | Not started | - |
| 6. Labour Market, Wages and Reservation Wages | 0/TBD | Not started | - |
| 7. Goods Market and Consumption | 0/TBD | Not started | - |
| 8. Ownership, Accounting and Dividends | 0/TBD | Not started | - |
| 9. Firm Planning | 0/TBD | Not started | - |
| 10. Bankruptcy and Respawn | 0/TBD | Not started | - |
| 11. Calibration, Burn-in and Full Acceptance | 0/TBD | Not started | - |

## Coverage

All 87 v1 requirements map to exactly one phase. No orphans, no duplicates.

| Phase | Requirements | Count |
|-------|--------------|-------|
| 1 | CORE-01 … CORE-11 | 11 |
| 2 | LEDG-01 … LEDG-10 | 10 |
| 3 | TICK-01 … TICK-10 | 10 |
| 4 | HARN-01, HARN-02, HARN-07 | 3 |
| 5 | PROD-01 … PROD-03 | 3 |
| 6 | LABR-01 … LABR-09 | 9 |
| 7 | MKT-01 … MKT-06 | 6 |
| 8 | OWN-01 … OWN-07 | 7 |
| 9 | PLAN-01 … PLAN-10, HARN-04, HARN-05 | 12 |
| 10 | BANK-01 … BANK-07, HARN-06 | 8 |
| 11 | CAL-01 … CAL-06, HARN-03, HARN-08 | 8 |
| **Total** | | **87** |

## Ordering Constraints

These are load-bearing and established independently by more than one research document. A
phase reorder that violates any of them is a defect, not a preference.

1. **Ledger, invariants, tick pipeline and log schema precede any economic rule** (Phases 1–4
   contain zero economics). Every `DESIGN`-tagged pitfall lives here and none can be retrofitted
   cheaply; without them every later economic bug is also an accounting mystery.
2. **Dividends ship in the same phase as firm accounting, never split** (Phase 8). In a bankless
   closed economy this is the only cycle-closing flow and the only equilibrating force.
   Deferring it means tuning Phase 9's price rule against a dying economy and misdiagnosing the
   stall as a price-rule bug.
3. **Money movement is introduced in dependency order**: goods exist (5) → money moves
   firm→household via wages (6) → household→firm via purchases (7) → the loop closes via
   dividends (8). Only then does Phase 9 have a circular flow to react to.
4. **Bankruptcy is last among the economics** (Phase 10) — it consumes the generational arena
   (3), worker release (6) and the ownership relation (8), and a firm that dies before the
   economy is alive teaches nothing.
5. **Tick order is not build order.** Firm planning runs *first* in the tick and is built
   *ninth*.
6. **The harness grows alongside the sim** (Phase 4 skeleton, then HARN requirements landing with
   the phases they gate), so each phase gate is an automated check rather than an eyeballed chart.
7. **Calibration is a phase, not acceptance polish** (Phase 11). Treating it as polish is how a
   build ends up with a passing run at exactly one point in parameter space.

## Standing Notes

- **Cadence is a 21-day month.** Every published parameter is used verbatim at its source grade;
  there are no daily or weekly rate conversions anywhere. The weekly/daily conversion tables in
  research SUMMARY.md §C are **superseded** — the 21-day option it describes as "closes this gap
  entirely" is the one taken. Consequently γ = 24 planning cycles, reservation decay is ×0.9 per
  month, price step is `U(0, 0.02)`, wage step is `U(0, 0.019)`, χ = 0.1 of monthly payroll, and
  `mc = wage / (productivity × 21)`.
- **The planning-cadence ACF lag is 21, not 7.** Any research text referring to "lag 7" refers to
  the superseded weekly cadence.
- **Every acceptance criterion admits a degenerate pass** and is therefore paired with a
  counter-check: money conserves if nothing trades (liveness invariant, Phase 2); reproducibility
  passes if the RNG is never consumed (different-seed mutation test + per-tick draw counts, Phase
  3); unemployment "fluctuates" from bankruptcy churn (churn-excluded variance, Phase 11); prices
  look stable because every firm is pinned at the floor (`fraction_at_floor` + price CV, Phase 9).
- **Section 10 of the brief is out of scope.** Banks, government, demographics, capital, multiple
  goods, R&D, stock market and geography are forward-compatibility context only, recorded under v2
  in REQUIREMENTS.md. No scaffolding is built for them; the three forward-compatibility mandates
  already in PROJECT.md (goods as data, ownership as a relation, provenance from tick 1) suffice.
- **Phase work lands on `gsd/phase-{n}-{slug}` branches and ships as PRs into `main`.**
