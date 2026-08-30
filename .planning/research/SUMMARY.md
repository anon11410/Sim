# Project Research Summary

**Project:** Sim — Minimal Closed Economy
**Domain:** Deterministic agent-based macroeconomic simulation (Lengnick / BAM class), Rust core + Python acceptance harness
**Researched:** 2026-08-30
**Confidence:** MEDIUM-HIGH (stack and architecture HIGH; economic parameters MEDIUM — see Confidence Assessment)

## Executive Summary

This is a closed, bankless, fixed-money agent-based macro model of the Lengnick (2013) / BAM (Delli Gatti et al. 2011) / Caiani et al. (2016) class: 200 households, 20 firms, one good, one tick = one day, 3,650 ticks. Experts build these as **rules of thumb over bounded local information, resolved sequentially, with disequilibrium as the object of study** — never as a market-clearing solver. The research converges on one coherent synthesis: a **Lengnick core** (everything that must be *local*: consumption `(m/P̄)^0.9`, inventory-band price rule with both cost bounds, γ-rigid wage rule, reservation wage, preferred-supplier list, θ=0.75 price inaction), plus **BAM** for what Lengnick lacks or handles degenerately (contract-at-hire wages, cheapest-first purchase with fall-through, `L^d = expected_demand / λ` labour demand, bankruptcy + trimmed-mean entry), plus **Caiani** for expectations (`E += 0.25·(observed − E)`) and the stock-flow-consistent accounting discipline the four invariants operationalise.

On the engineering side, two properties decide whether the build succeeds — **byte-identical reproducibility from a seed** and **money conservation to the cent** — and every stack choice is justified against one or both. The load-bearing decisions are: `ChaCha8Rng` (documented portable) rather than `StdRng`/`SmallRng` (documented *non*-portable); a `Money(i64)`-cents newtype with checked arithmetic **and** `overflow-checks = true` in the release profile (verified: default release silently wraps); a **central ledger** (`Books`) that owns every cent and every unit so agents hold no value and zero-sum trade is structurally true; invariants as a real pipeline phase returning `Result` (never `debug_assert!`, which is compiled out of release); generational `FirmId{slot,gen}` with respawn-in-place; and CSV + JSONL logs with money as integer `*_cents` columns so the Python conservation audit is an exact `int64` equality rather than a tolerance check.

The dominant risks are economic, not technical, and they are ranked. **The deflationary stall is #1**: in a closed fixed-money economy `M = household_cash + firm_cash` identically, so persistent firm profit drains households by construction and the dividend payout rule is *the only equilibrating force in the model* — it is a missing equilibrium condition, not a missing feature, and it must ship in the same phase as firm accounting. **Validation theatre is #2**: every acceptance criterion in the brief has a degenerate way to pass (money conserves perfectly if nothing trades; reproducibility passes if the RNG is never consumed; unemployment "fluctuates" from bankruptcy churn), so the counter-checks are real roadmap work. **Cadence mis-scaling is #3**: every Lengnick rate is per 21-day month while the brief's cadence is weekly and its rates are daily, and the brief's 1%/day reservation decay compounds to ~19%/month against Lengnick's published 10% — a correctness error, not a tuning preference. Mitigation is uniform: published values where they exist, invariants plus liveness checks from the first commit, and a genuine calibration phase because the initial conditions and total money stock are unspecified in every source.

## Key Findings

### Recommended Stack

Single Cargo crate with `lib.rs` + thin `main.rs` (so `tests/` can reach the code — with only `main.rs`, integration tests cannot import anything), a sibling `analysis/` Python directory, and no workspace. `rust-toolchain.toml` pinning rustc 1.94.1 and a committed `Cargo.lock` are **load-bearing for determinism, not hygiene**: `sort_unstable` tie order is unspecified and changed in Rust 1.81, and `rand`'s distribution algorithms are only *typically* portable across versions.

**Core technologies:**
- **`rand` 0.10.2** (`default-features = false`, features `["std","chacha"]`) with **`ChaCha8Rng`** — the one seeded RNG. Feature-gating makes `rand::rng()` *not compile*, so no path can reach OS entropy. `StdRng`/`SmallRng` are explicitly disclaimed as non-portable. Note the 0.10 API break: `Rng`→ core trait, `RngExt`→ extension, `gen_range`→`random_range`, `choose_multiple`→`sample`.
- **`Money(i64)` cents newtype, no crate** — checked `Add`/`Sub` panic on overflow regardless of build profile; `[profile.release] overflow-checks = true` catches raw `i64` that escapes the newtype. `Money::split(n)` distributes remainder deterministically. No `From<f64>`, no `Mul<f64>`, no decimal `Display`.
- **`serde` 1.0.229 + `toml` 1.1.4** with `#[serde(deny_unknown_fields)]` and **no `#[serde(default)]` anywhere** — a serde default *is* a hardcoded parameter. Reject `config`/`figment`: layering is an invisible input that breaks reproducibility.
- **`csv` 1.4.0 → `ticks.csv`, `serde_json` 1.0.151 → `events.jsonl`**, both byte-identical (itoa/ryu/zmij, fixed field order, `BTreeMap`-sorted keys). `run_meta.json` carries the nondeterministic metadata and is excluded from the diff. Parquet/Arrow explicitly rejected — opaque binary cannot be `diff`ed, and the determinism proof *is* a diff. `tracing`'s JSON layer rejected — timestamps and span ids.
- **`clap` 4.6.6** — exactly three flags (`--config`, `--seed`, `--out`); parameter sweeps generate TOML files, never CLI overrides.
- **`thiserror` 2.0.20** in the lib (`InvariantViolation` carries tick, agent, transaction), **`anyhow`** in `main.rs` only. **`proptest` 1.11.0** (committed `.proptest-regressions` — a counterexample found once replays forever), **`insta` 1.48.0**, **`assert_cmd` 2.2.2**, **`sha2`**.
- **Python 3.13 + pandas 3.0.5, numpy 2.5.2, statsmodels 0.15.0, matplotlib 3.11.1**, managed by `uv` with a committed `uv.lock`. The harness is **pytest**, not a script or a notebook. `statsmodels.tsa.stattools.acf` (biased estimator + Bartlett bands, the ABM-literature convention) — not `Series.autocorr`, which is a different estimator.

**Float boundary (verified against rustc 1.94.1 std source):** `+ - * / sqrt mul_add floor ceil round trunc` are correctly rounded and bit-reproducible. **31 `f64` methods** (`powf powi exp ln log2 sin cos tan hypot cbrt …`) carry a std disclaimer that precision "can even differ within the same execution from one invocation to the next" — banned on the behaviour path via a `clippy.toml disallowed-methods` entry. `f64` is acceptable for `expected_demand` only, with one named crossing function to integers; `i64` milli-units is the equally defensible alternative to settle in that phase.

### Expected Features

**Must have (table stakes — without these the model class does not produce believable dynamics):**
- Decentralised **goods** market with bounded sampling (5 sampled from a persistent list of n=7), cheapest-first with stockout fall-through
- Decentralised **labour** market with bounded sampling (β=5 unemployed, 1 employed, π=0.1 employed-searches-anyway) — involuntary unemployment is *generated by* the friction
- Inventory-band price rule with **both** cost bounds: υ=0.02, θ=0.75, φ∈[0.25, 1.0], ϑ∈[**1.025**, **1.15**]×marginal cost
- Downward-rigid wage rule: δ=0.019, γ=24 **planning cycles** (`ceil` on rises, `floor` on cuts — a deliberate integer ratchet)
- Reservation wage: ×0.9/month when unemployed; **ratchet** `w_r ← max(w_r, wage_received)` when employed (a ratchet, not a rate — it is self-limiting and cannot drive a spiral alone)
- **Dividends via a stock buffer** (χ=0.1 × payroll), same phase as accounting
- Adaptive demand expectation λ=0.25; inventories as a real conserved stock; hire/fire keyed to the inventory band
- Per-tick **reshuffled** activation order from the seeded RNG (fixed order is a systematic wealth transfer correlated with agent ID)
- Integer money with "subtract what was **actually** transferred"; stock-flow-consistent accounting; bankruptcy + replacement entry

**Should have (differentiators):**
- Preferred-supplier list with ζ=0.01 / ψ_price=0.25 / ψ_quant=0.25 revision — the strongest single source of realistic heterogeneity in the class
- **θ = 0.75 price inaction** — the *published* desynchroniser, and the answer to the problem the brief solves with staggering
- Entrant sized at 0.8 × a 5%-trimmed mean of incumbents, priced at **1.26 × market average** (counter-intuitive but load-bearing: entrants priced at market fail immediately and the bankruptcy rate runs away)
- Employed-vs-unemployed search intensity (5 vs 1); rationing-driven "blackmarking" supplier switching; firing notice period

**Defer (v1.x, trigger-gated):** blackmarking, BAM's price-or-quantity-never-both damper, firing notice, γ/χ/α/λ sensitivity sweep, cross-validation against Lengnick's stylised facts.
**Out of scope (v2+):** banks, government, multiple goods, capital, R&D, demographics, stock market, geography, GUI, plotting toolkit, scaling. Build **no scaffolding** — the three forward-compatibility constraints already in PROJECT.md are sufficient.

**Anti-features with named destruction mechanisms:** Walrasian clearing (sets excess demand to zero by construction, so the inventory band never fires and the price rule has no input); perfect price information (dispersion collapses in one tick, ζ never binds, one giant and 19 corpses); representative agents (every rule is non-linear — Jensen on `(m/P̄)^0.9`, thresholds on bankruptcy — and the *distribution* is the output); global average price as a decision input (BAM does it and works, but it synchronises firms onto one signal — follow Lengnick, every input local); overdrafts (an implicit zero-interest credit market); float money; simultaneous/double-buffered update (re-introduces the clearing problem); clamping "unrealistic" output; multi-threading.

### Architecture Approach

Six decisions carry the architecture. **Agents hold no money and no goods** — a single `Books` module owns every cent and every unit, which dissolves the two-mutable-borrows problem and makes conservation a local check. **`FirmId` is `{slot, gen}`** over a fixed-size arena, so a stale ID is a typed miss rather than a silently-wrong firm, and `(slot, gen)` is the per-agent log identity. **The journal is a per-tick buffer**, read by the invariant checker to name the offending posting, then cleared. **The tick is a `const` array of named function pointers** — the array *is* the ordering, one test asserts the exact name sequence, and invariants and logging are phases inside it so they cannot be skipped. **`Ctx { world, rng, params, sink, tick }`** carries the cross-cutting concerns so provenance and RNG never appear in a decision function's signature. **`ChaCha8Rng`, never `StdRng`.**

**Major components:**
1. **`money` / `ids` / `config` / `rng`** — the vocabulary: `Cents(i64)`, `HouseholdId`/`FirmId{slot,gen}`/`GoodId`/`Account`, `Params` + config hash, the single RNG wrapper with hand-rolled fixed-draw samplers.
2. **`books`** — all cash, all stock, the per-tick journal, and `transfer`/`settle`/`produce`/`consume`. The *only* thing that moves value; contains no economic rules.
3. **`world`** — `Vec<Household>`, the `Vec<Firm>` arena, `Ctx`. Behavioural state only, never balances.
4. **`invariants`** — four checks plus journal bisection to name the breaking posting. Returns `Result`, reads `&Books`, mutates nothing.
5. **`phases/*`** — one file per tick phase, in tick order: planning, labour, production, wages, goods_market, accounting, bankruptcy, invariants, log.
6. **`goods` / `ownership`** — the goods table with recipes (data, not an enum) and the ownership edge list with a both-direction index (a relation, not a field).
7. **`log`** — `Sink` trait (`NullSink`/`VecSink`/`RunWriter`), record types, and a **generated, committed `schema/schema.json`** that Python reads; schema drift is a test failure.
8. **`analysis/`** — pytest acceptance harness reading the run directory across a disk boundary. The boundary is a feature: it forces the log to be complete.

### Critical Pitfalls

1. **The deflationary stall** — firms accumulate, households run dry, every invariant still passes because no money was lost, it moved. *Avoid:* log `firm_share = firm_cash / M` from tick 1 (it fires 1–2 years before prices or unemployment show anything); denominate the buffer in **flow** terms (a multiple of recent wage bill, never a nominal constant, which becomes a permanent sink as the price level drifts); **drain the entire excess above the buffer, not a fraction** (a fractional drain leaves a geometrically-decaying residue that still accumulates in aggregate); match payout cadence to spending cadence; route every firm-side cash exit to a household.
2. **Validation theatre** — every acceptance criterion admits a degenerate pass. *Avoid:* **liveness invariants** (transaction count > 0, cents transferred > 0, employment > 0, goods produced and consumed > 0, every tick); a **mutation test** (different seed *must* produce a different log); a **negative test** (a deliberately-broken build must halt on a seeded violation — an invariant never observed to fire has never been shown to work); harness **replays logged transfer events** from the initial endowment rather than re-reading the sim's own aggregate; variance floors and max-run-length limits, not just a mean in a band; ACF must **decay**, not merely be positive.
3. **Determinism leaks beyond HashMap** — the biggest is **RNG consumption-order drift**: one added draw shifts every subsequent draw, so you fix a bug and cannot tell whether the trajectory changed from the fix or the reseeding. *Avoid:* **per-purpose RNG sub-streams** keyed on `(master_seed, tick, agent_id, purpose_tag)` — the single highest-leverage decision available and impossible to retrofit; **fixed-draw sampling** (partial Fisher-Yates, never rejection sampling — note the bankruptcy owner-redraw loop is a variable-draw site); an RNG draw-count series per tick to localise divergence; **every comparator tie-broken by agent ID** (`sort_unstable_by_key(|f| (f.price, f.id))`); `HashMap` for point lookups only, never iterated; no `rayon` in `Cargo.toml` at all.
4. **Price collapse / a floor that binds too often** — the subtle mode matters more than the obvious one: if most firms sit exactly at the floor, cross-sectional price dispersion collapses, "sample 5 and buy cheapest" becomes a coin flip, and **the search mechanism is silently dead while prices look admirably stable**. *Avoid:* floor as a named, tested function with an explicit zero-output fallback (`wage_rate / productivity`, never `ulc = 0`); floor at **1.025× marginal cost**, ceiling at 1.15×; log `fraction_at_floor` and price CV every tick.
5. **Synchronisation artefacts** — all firms replanning on one day produces a clean periodic component at exactly the planning period that looks like a business cycle and is the schedule. Fixed within-tick ordering produces a smooth, plausible, entirely spurious wealth distribution monotone in agent ID. *Avoid:* one fresh seeded permutation per market per tick; per-firm planning offsets **drawn at init from the RNG, not `id % 7`** (which leaves 7 cohorts in lockstep); acceptance check that the ACF has no spike at the planning cadence.
6. **Order-of-operations accounting** — `last_sales` vs `sales_this_tick` as separate fields with one named `roll_over()`; `output == productivity × headcount_at_production` as an invariant; **atomic `transfer()`** so the books are never mid-transaction at a statement boundary (this is what makes per-market invariant checks safe); dividends land before the bankruptcy check in the specified tick order, so the buffer must be sized to make that safe.

## Reconciled Decisions — where the research documents disagreed

The roadmapper should treat this section as binding. Each row states one rule.

### A. Build order — ONE reconciled sequence

ARCHITECTURE proposed S0–S9; PITFALLS proposed P1–P6; FEATURES proposed a dependency-ordered MVP list. **Taken: the ARCHITECTURE S0–S9 spine**, because it is the only one derived from actual compile-verified dependency edges, with **two amendments**:

- **The Python harness is promoted from "start during S2" to its own early phase.** ARCHITECTURE says start it at S2; PITFALLS shows the conservation-replay and mutation tests are themselves substantial correctness work. Making it a phase forces the schema seam and the counter-checks to exist before there is economics to flatter them.
- **A calibration/burn-in phase is added at the end, which no document had as a phase.** FEATURES establishes that all initial conditions and the total money stock are genuinely unspecified in the literature; PITFALLS establishes that burn-in must be justified by a stationarity check rather than by the brief's number. This is real work and it is not "acceptance".

Two ordering claims are load-bearing and are inherited unchanged from all three documents:

- **Ledger + invariants + pipeline + log schema must exist before ANY economic rule.** Every economic phase is then born under the check rather than retrofitted into it. PITFALLS reinforces this: pitfalls 10, 11, 12, 15-liveness and 16 all land in the first phase and are all `DESIGN`-tagged — they cannot be retrofitted cheaply, and without them every economic bug is also an accounting mystery.
- **Dividends ship in the same phase as firm accounting, never later.** In a bankless closed economy this is the only cycle-closing flow and the only equilibrating force. Deferring it means tuning the price rule against a dying economy and misdiagnosing the stall as a price-rule bug.

Note also ARCHITECTURE's framing, which the roadmap should adopt explicitly: **tick order is not build order.** `planning` runs first in the tick but is built seventh, because a reaction rule cannot be tuned before there is something to react to.

### B. Dividend rule — stock buffer, full drain

**FEATURES and PROJECT.md agree in kind and differ only in magnitude.** Both specify a **stock** rule (a working-capital buffer), not a flow share. FEATURES recommends Lengnick's χ=0.1 × payroll and rejects flow rules on a structural argument: `Div = δ·π` bounds the *rate* cash leaves a firm but places no bound on the *stock*, and BAM survives δ=0.15 only because banks recycle firm cash as loan repayments and bank capital while bankruptcy periodically flushes the accumulation. This project has neither. PROJECT.md's "2 weeks of payroll" is the same shape.

**The one rule for the roadmapper:**

```
buffer   = χ × recent_payroll          (flow-denominated, χ in config)
dividend = max(0, firm_cash − buffer)  (FULL drain of the excess, not a fraction)
```

Magnitudes: Lengnick's χ=0.1 is 0.1 of a **monthly** (21-day) payroll ≈ 2.1 days ≈ 0.30 of a *weekly* payroll. The brief's 2 weeks ≈ 10 working days ≈ 0.48 of a monthly payroll — roughly **5× Lengnick's buffer**. Take **Lengnick's χ=0.1-of-monthly as the default** (it is the published value, and the smaller buffer is the safer side of the stall), expose χ in config, and treat the brief's 2-week figure as the upper end of the sensitivity sweep. Pay per planning cycle, not monthly — PITFALLS notes that a payout cadence slower than the spending cadence parks money for most of its life.

Two implementation constraints carry forward verbatim in effect: **subtract what was actually paid**, not the intended amount (rounding residue stays with the firm and money conserves to the unit); and **dividends precede the bankruptcy check** in the specified tick order, so the buffer must be sized to make that safe.

**Deliberate, documented deviation:** all three papers pay dividends to the whole household population pro-rata to wealth. This project's single-owner firms have no published precedent. Keep the deviation — it is the seed of the later stock-market milestone — but expect a materially more unequal wealth distribution than any paper reports, and remember it when the firm-size and wealth-distribution acceptance checks are calibrated.

### C. Cadence — a correctness issue, not a tuning preference

Every Lengnick rate is per **21-day month**. The brief plans **weekly** and states rates **per day**. Applying monthly numbers at a weekly cadence runs the whole adjustment side ~3× too fast. **Decision: keep the weekly planning cadence, rescale every rate, and put the conversion table in the config file as comments.** (Adopting a 21-day cadence instead is the alternative that closes this gap entirely — cheaper if the roadmapper prefers it, and it would make every Lengnick number drop in verbatim.)

| Lengnick, per month (21d) | Per week (7d) | Per day |
|---|---|---|
| reservation wage ×0.9 | ×0.9^(1/3) = **×0.9655** (−3.45%) | ×0.9^(1/21) = **×0.99500** (−0.50%) |
| price step U(0, 0.020) | U(0, **0.0067**) | U(0, 0.00095) |
| wage step U(0, 0.019) | U(0, **0.0063**) | U(0, 0.00090) |
| γ = 24 months | **72 weeks** | 504 days |
| χ = 0.1 × monthly payroll | **0.30 × weekly payroll** | 2.1 days of payroll |

**The brief's 1%/day reservation decay compounds to 1 − 0.99²¹ = 19%/month, ~2× Lengnick's published 10%.** Use **0.5%/day** (or apply ×0.9 once per month). The conversions themselves are grade C — arithmetic on grade-B values — and are the research's own, not published.

**And the trap that costs a factor of 7:** γ must count **planning cycles, not ticks**. At a weekly cadence γ=24 cycles ≈ 5.5 months, already far weaker than Lengnick's 24 months; matching Lengnick's rigidity needs γ=72 weeks. Getting this wrong destroys the downward nominal wage rigidity that keeps the price level up.

### D. Price bounds — 1.025× floor and a 1.15× ceiling

The brief floors price at unit labour cost (a 1.0× multiplier) and omits a ceiling entirely. **Lengnick's published bounds are ϑ_l = 1.025 × marginal cost and ϑ_u = 1.15 × marginal cost.** PITFALLS independently warns that a too-tight floor collapses price dispersion and silently kills the search mechanism. These reconcile cleanly and point the same way:

- **Floor = 1.025 × MC.** A 1.0× floor permits zero-margin pricing indefinitely — the firm never accumulates the χ buffer, never pays dividends, and drags the price level down: exactly the deflationary stall.
- **Ceiling = 1.15 × MC**, and it is a genuinely separate mechanism, not decoration: a firm with low inventory that is already at 1.15× cost **will not raise price — it must hire instead.** This is a real channel from the goods market to the labour market that the brief currently has no path for.
- **Marginal cost** `mc = wage_rate / (λ · l · period_length)`; with λ=3, l=1, 21 days, `mc = w/63`. Compute from the *previous* period's realised wage bill and output (price is set at planning time, before this tick's wages exist), with an explicit zero-output fallback of `wage_rate / productivity`.
- **Monitor, don't just implement:** log `fraction_at_floor` and price CV every tick; healthy is low single-digit percent at the floor. Near-100% means the price rule is inert and the floor is doing all the work.
- **Asymmetry lives in the trigger, not the magnitude.** `U(0, υ)` both directions, θ=0.75 gating both. Do not add a magnitude asymmetry; none is published, and an asymmetric step gives the random walk positive drift by construction.

### E. Staggering — a project invention; run both, and keep them separable

**Flag clearly: no published model in this class staggers agent decisions.** Lengnick and BAM are both explicitly synchronous. Lengnick desynchronises via **θ = 0.75 price inaction** (a Calvo-style random-inaction rule, gating *price only* — the wage rule and hire/fire run every cycle for every firm) plus a **daily reshuffle of household order**.

**Decision: ship θ=0.75 as the primary, published desynchroniser. Keep the brief's weekly stagger as well, but implement it as a config-toggleable project deviation and record it as such.** Rationale: they attack different things (θ desynchronises the price decision; the stagger desynchronises the whole planning cycle), the brief mandates the stagger, and PITFALLS treats synchronised replanning as a first-class artefact source. Running both is safe; running neither is not.

Two constraints on the stagger implementation: the per-firm offset must be **drawn once at init from the seeded RNG, not `firm_id % 7`** (which leaves 7 cohorts of ~3 firms still in lockstep, and correlates cadence with slot, which respawn then perturbs); and the ACF of the aggregate series must show **no spike at lag 7** — that is the directly computable artefact detector and it belongs in the harness. If the stagger turns out to be inert once θ is in, disabling it via config is a one-line experiment.

### F. Other reconciled conflicts

- **Wage contract semantics.** PROJECT.md records "wages contracted at hire, **matches Lengnick**". The attribution is **wrong** — in Lengnick the wage *floats* and the household has no wage field at all. **Keep the decision, re-attribute it to BAM.** It is required by the project's separate `Household.wage` / `Firm.offered_wage` layout, it avoids a same-tick feedback loop between the reservation-wage ratchet and the firm's offer, and it fixes Lengnick's degenerate "interns work for zero" insolvency handling. Contract length is a **PROJECT CHOICE**: recommend indefinite (ends only on quit, fire or bankruptcy), because there is no unemployment benefit and no credit market to cushion forced churn.
- **Labour demand rule.** Lengnick adjusts ±1 worker per cycle; at 10 workers/firm and a weekly cadence that is a 10% workforce swing per week — far too fast. **Take BAM's `L^d = expected_demand / λ`, `V = max(L^d − L, 0)`**; it scales with firm size and the project already has `expected_demand`. Note the deviation: Lengnick's firm-size distribution is partly an artefact of the ±1 rule.
- **Inventory reference.** If λ=0.25 smoothing is adopted, evaluate Lengnick's inventory band **against expected demand**, not last period's raw demand — otherwise two different demand notions drive price and quantity, which is a subtle desynchronisation bug.
- **Purchase ordering.** Take BAM's cheapest-first with fall-through (the brief specifies it), but know that stacking it on Lengnick's persistent 7-firm list makes price competition fiercer than in *either* paper. **If prices deflate in testing, reverting to Lengnick's random-order-within-list is the published fallback** — check here first.
- **Entrant funding.** BAM creates entrant net worth ex nihilo, which would break conservation. The project's owner-funded respawn is the correct conservation-preserving adaptation and has no published counterpart. Take BAM's *sizing* (0.8 × a 5%-trimmed mean of incumbents; at 20 firms consider trimming a fixed 1 from each tail rather than a percentage) and *pricing* (1.26 × market average).

## Preserved Parameters — do not compress these away

Every value carries its source grade: **A** = model authors' own code, **B** = annotated replication citing the paper's table/equation numbers, **C** = derived arithmetic, **PROJECT** = a choice with no published precedent. These close the gaps in the project's own parameter table and are the single most valuable research output.

| Parameter | Value | Symbol | Source | Cadence | Grade |
|---|---|---|---|---|---|
| Consumption exponent, `c = (m/P̄)^α` | **0.9** | α | Lengnick T1, Eq(11) | — | B |
| — `P̄` is the mean over the household's **own** supplier list, not a global index | | | Lengnick | | B |
| Demand-expectation smoothing, `E += λ(obs − E)` | **0.25** | λ | Caiani `adaptiveParam` | period | **A** |
| Productivity (goods per worker-day) | **3** | λ_prod | Lengnick T1 | day | B |
| Price adjustment bound, `×(1 ± U(0,υ))` | **0.02** | υ | Lengnick T1 | month | B |
| P(firm considers a price change) | **0.75** | θ | Lengnick T1 | month | B |
| Inventory floor / demand | **0.25** | φ_l | Lengnick T1 | month | B |
| Inventory ceiling / demand | **1.0** | φ_u | Lengnick T1 | month | B |
| Price floor / marginal cost | **1.025** | ϑ_l | Lengnick T1 | — | B |
| Price ceiling / marginal cost | **1.15** | ϑ_u | Lengnick T1 | — | B |
| Wage adjustment bound (`ceil` up, `floor` down) | **0.019** | δ | Lengnick T1 | month | B |
| Cycles of full staffing before a wage cut | **24** | γ | Lengnick T1 | month | B |
| Dividend buffer / payroll | **0.1** | χ | Lengnick T1 | month | B |
| Reservation wage decay, unemployed | **×0.9** | — | Lengnick | month | B |
| Reservation wage, employed | **`max(w_r, wage_received)`** — a ratchet, not a rate | — | Lengnick | month | B |
| — grade-A alternative | ±\|N(0, 0.0094)\|; cut if unemployed >0.49 of last 4 periods; raise if employed and U ≤ 0.08 | ζ | Caiani `AdaptiveWageStrategy` | period | **A** |
| Preferred-supplier list size | **7** | n | Lengnick T1 | — | B |
| Supplier switch price threshold | **0.01** | ζ | Lengnick T1 | — | B |
| P(price search) / P(rationing search) | **0.25 / 0.25** | ψ_price, ψ_quant | Lengnick T1 | month | B |
| Demand satisfaction fraction | **0.95** | — | Lengnick | day | B |
| Firms sampled by a consumer | **5** | nbSellers | Caiani (Lengnick: n=7 persistent list) | period | **A** / B |
| Firms sampled, unemployed job seeker | **5** | β | Lengnick T1 | month | B |
| Firms sampled, employed job seeker | **1** | — | Lengnick | month | B |
| P(employed household searches anyway) | **0.1** | π | Lengnick T1 | month | B |
| Inventory target / expected sales (alt.) | **0.1** | — | Caiani `inventoryShare` | period | **A** |
| Bankruptcy trigger | **net worth ≤ 0 or output ≤ 0** | — | BAM submodel 39 | — | B |
| Entrant size vs trimmed mean | **0.8×** | 1−s | BAM `size-replacing-firms`=0.2 | — | B |
| Incumbent trim for the mean | **5% tails** | — | BAM submodel 41 | — | B |
| Entrant price vs market average | **1.26×** | — | BAM `replace-bankrupt` | — | B |
| Month length | **21 days** | — | Lengnick | — | B |
| Marginal cost | `w / (λ_prod · l · 21)` = `w/63` | mc | Lengnick | — | B |
| Weekly/daily rescalings of all of the above | see §C table | — | derived | — | **C** |
| Labour demand `L^d = E[D]/λ_prod`, `V = max(L^d − L, 0)` | — | — | BAM | period | B |
| Contract length | indefinite (ends on quit/fire/bankruptcy) | — | — | — | **PROJECT** |
| Stagger assignment | per-firm offset drawn at init from the seeded RNG | — | — | — | **PROJECT** |
| Entrant funding | from the owning household's cash, redraw if unaffordable | — | — | — | **PROJECT** |
| Single-owner firms | 20 of 200 households hold one firm each | — | — | — | **PROJECT** |

**Do NOT treat as sourced:** the weekly/daily conversions (grade C arithmetic), and everything in the PROJECT rows. The Lengnick numbers are grade **B** — recovered from an annotated replication that cites Table 1 and Eq(5)–(12) inline, not read from the PDF.

## Correctness Constraints — carry forward verbatim in effect

The roadmapper must not silently drop any of these. Each is a `DESIGN` decision that cannot be retrofitted cheaply.

1. **`ChaCha8Rng`, never `StdRng`/`SmallRng`** — the latter are documented non-portable across library versions and platforms.
2. **`overflow-checks = true` in `[profile.release]`** (Cargo defaults it *off*), **plus** checked arithmetic inside the `Money` newtype — the profile flag catches raw `i64` that escapes the type, the type catches everything else regardless of profile.
3. **Invariants are a real pipeline phase returning `Result`, never `debug_assert!`** — `debug_assert!` is compiled out of release, and an invariant absent from the binary that produced the run is worth nothing. Cost is ~220 `i64` adds per tick; run it in release, every tick, always.
4. **Generational `FirmId{slot, gen}`, respawn in place, never `Vec::swap_remove`** — `swap_remove` corrupts every ID-carrying structure and every stored log series; PITFALLS rates recovery from it as HIGH and prevention as the only sane strategy.
5. **RNG sub-streams keyed per purpose** — `(master_seed, tick, agent_id, purpose_tag)`, so a refactor that changes the draw count in one market cannot perturb another. Plus fixed-draw sampling (partial Fisher-Yates, never rejection sampling) and a per-tick draw-count series for localising divergence.
6. **Every comparator tie-broken by agent ID** — `sort_unstable` tie order is unspecified and Rust 1.81 replaced both sort implementations. Ties on 20 firms priced in integer cents are constant.
7. **Money logged as integer `*_cents` columns** — a decimal string makes pandas read `float64` and degrades the conservation audit from exact `int64` equality to a tolerance check. Assert the dtype in the harness.
8. **`lib.rs` + thin `main.rs`** — a binary-only crate exposes nothing, so `tests/` could not reach the code at all.

Supporting rules of the same character: single `transfer()` as the only cash-mutation point, made **atomic** so the books are never mid-transaction; balance fields private, no `pub fn set_cash`; `Money::split` distributes remainder deterministically and callers subtract what was *actually* transferred; no `HashMap` iteration on a behaviour path (`clippy.toml disallowed-types`); the 31 non-deterministic `f64` methods banned via `disallowed-methods`; no `rayon` in `Cargo.toml` at all; no `-C target-cpu=native`; no wall-clock, hostname, path or PID in the diffed logs; committed `Cargo.lock` + `rust-toolchain.toml`; provenance recorded as a **joinable flat table** (tick + agent + decision type), never free text.

## Implications for Roadmap

Suggested phase structure: **11 phases**, Fine granularity. Each phase's gate is stated because the gates are what make the ordering safe.

### Phase 1: Primitives and the determinism spine
**Rationale:** These are the vocabulary; every later phase depends on them and retrofitting any one is a rewrite. All the `DESIGN`-tagged pitfalls live here.
**Delivers:** `Cents(i64)` with checked arithmetic and `split()`; `HouseholdId` / `FirmSlot` / `FirmId{slot,gen}` / `GoodId` / `Account`; `Params` from TOML with `deny_unknown_fields` and no serde defaults, plus a config hash; the `ChaCha8Rng` wrapper with **per-purpose sub-streams**, fixed-draw samplers and a seeded shuffle utility; `clippy.toml` bans; `[profile.release] overflow-checks = true`; `rust-toolchain.toml`; `lib.rs` + thin `main.rs`.
**Avoids:** Pitfalls 10 (conservation leaks), 11 (rounding), 12 (determinism drift), 16 (undiagnosable diagnostics).
**Gate:** property tests — `Cents` never silently wraps, `split(n)` sums exactly to the whole, same seed → identical `u64` stream, generated permutations are valid permutations with a fixed draw count.

### Phase 2: Books, journal and invariants — built as one unit
**Rationale:** The ledger is what the invariants check; separating them means writing the checks against a moving target. Nothing economic may enter before this gate.
**Delivers:** `books.rs` (all cash, all stock, per-tick journal, atomic `transfer`/`settle`/`produce`/`consume`, overdraft refusal); `invariants.rs` (money conservation, goods conservation, non-negative balances, zero-sum trade — plus the **liveness** checks) returning a `thiserror` `Violation` carrying tick, agent, posting, delta and sign; journal bisection; a ring buffer of the last N transfers dumped on violation.
**Avoids:** Pitfalls 10, 13 (order-of-operations, via atomic transfer), 15 (liveness half).
**Gate:** a deliberately corrupted `Books` **fails each of the four checks** naming the right agent and posting — the negative test. An invariant never observed to fire has never been shown to work.

### Phase 3: World, tick pipeline and the log seam
**Rationale:** The harness must be real before any economics exist, so every rule is born under the check and inside the schema.
**Delivers:** `world.rs` (arena, `Ctx`); `phases/mod.rs` with the `const PHASES` table — all 9 phases present as no-ops; `log/` (`Sink` trait, record types, generated + committed `schema/schema.json`, `RunWriter` → `ticks.csv` / `events.jsonl` / `run_meta.json`); `tests/determinism.rs`; `--halt-at-tick` / `--dump-state-at-tick`.
**Uses:** `csv`, `serde_json`, `serde`, `clap`, `sha2`, `insta`, `assert_cmd`.
**Gate:** 3,650 **empty** ticks run; invariants pass trivially; a run directory is produced; two runs at the same seed diff clean; the phase-order test asserts the exact name sequence.

### Phase 4: Python acceptance harness skeleton
**Rationale:** Promoted to its own phase from ARCHITECTURE's "start at S2". The conservation replay and the mutation test are substantial correctness work, and building them against the empty run proves the seam before economics can flatter it.
**Delivers:** `uv` project with a committed lock; `schema.py` validating against the committed schema; conservation audit as an **event replay** from the initial endowment (a second, independent derivation, not a re-read of the sim's own aggregate); seed-reproducibility diff via `hashlib`; the **different-seed-differs** mutation test; dtype assertions on `*_cents`; pytest wiring with a `--run-dir` fixture.
**Avoids:** Pitfall 15 (validation theatre) at the point where it is cheapest to close.
**Gate:** the harness passes on the empty run and **fails loudly** on a hand-corrupted log.

### Phase 5: Goods, recipes and production
**Rationale:** The first thing that moves the goods identity, and `goods_market` cannot sell what does not exist.
**Delivers:** `goods.rs` (`GoodsTable`, `Recipe` — data, not an enum, satisfying forward-compat mandate 1); `phases/production.rs`.
**Gate:** `created − destroyed − Σstock == 0` over 3,650 ticks; `output == productivity × headcount_at_production`.

### Phase 6: Labour market, wages and reservation wages
**Rationale:** The first thing that moves **money**, and households need income before they can have a budget.
**Delivers:** bounded sampling (β=5 / 1, π=0.1); **contract-at-hire** wages (BAM, re-attributed); reservation wage decay ×0.9/month rescaled and the employed-side **ratchet**; `phases/wages.rs`; per-tick seeded shuffle of job-seeker and worker order.
**Avoids:** Pitfalls 4 (unemployment pinning), 6b (ordering artefacts), 12.
**Gate:** money conserves across 3,650 ticks of hiring and paying; unemployment is neither 0% nor 100%; wealth-vs-ID rank correlation ≈ 0.

### Phase 7: Goods market and household consumption
**Rationale:** Closes the household→firm leg of the circular flow. Prices and wages stay **static** here on purpose.
**Delivers:** bounded sampling (5) over a persistent preferred-supplier list (n=7, ζ=0.01, ψ=0.25/0.25); cheapest-first with stockout fall-through; `(m/P̄)^0.9` with `P̄` over the household's own list; per-firm `last_sales` from that firm's actual purchase events (never an aggregate); unmet demand recorded.
**Avoids:** Pitfalls 7 (search friction mis-sizing), 8 (firm-size degeneracy), 5 (dead steady state).
**Gate:** a full circular flow exists; both conservation invariants hold; price CV and unmet-demand fraction are small but non-zero.

### Phase 8: Ownership, accounting and dividends — one phase, never split
**Rationale:** **The single most important ordering constraint in the roadmap.** This is the only cycle-closing flow in a bankless economy and the only equilibrating force. Deferring it means tuning Phase 9's price rule against a dying economy and misdiagnosing the stall.
**Delivers:** `ownership.rs` (edge list + both-direction index — a relation, satisfying forward-compat mandate 2); `phases/accounting.rs` with the **flow-denominated buffer and full drain of the excess**; `firm_share` logged from this phase's first commit; the deterministic remainder rule.
**Avoids:** Pitfall 1 (the deflationary stall), Pitfall 13d (dividends before the solvency check).
**Gate:** a 3,650-tick run does **not** stall; the regression slope of `firm_share` over the retained window is indistinguishable from zero; `firm_share < 0.5` every tick; the split is exactly zero-sum after rounding.

### Phase 9: Firm planning — expectations, price rule, wage rule
**Rationale:** Built **last of the reactive economics** because it needs sales history and a non-stalling economy to tune against. Tick order is not build order.
**Delivers:** adaptive expectation λ=0.25; price rule with **both** bounds (1.025× / 1.15× MC), υ=0.02, θ=0.75, φ∈[0.25,1.0]; wage rule δ=0.019 with γ counted in **planning cycles**; weekly cadence with per-firm RNG-drawn stagger offsets (config-toggleable); `last_sales` / `sales_this_tick` separated with one named `roll_over()`; ULC computed from the previous period's realised figures with an explicit zero-output fallback.
**Avoids:** Pitfalls 2 (price spiral), 3 (collapse / inert floor), 6a (synchronised cadence), 13a/13c.
**Gate:** prices move and do not spiral; `fraction_at_floor` in low single digits; up/down adjustment counts roughly balanced; output autocorrelates; **no ACF spike at lag 7**.

### Phase 10: Bankruptcy and respawn
**Rationale:** Last among the economics because it needs the generational arena (P3), worker release (P6) and the ownership relation (P8).
**Delivers:** mark-then-sweep — release workers, residual cash → owner via `transfer()`, retire ownership edges, respawn at `gen+1` **in place**, entrant sized at 0.8 × a trimmed mean of incumbents and priced at 1.26 × market average, seed capital from the owning household with a fixed-draw redraw.
**Avoids:** Pitfall 9 (bankruptcy/respawn artefacts), Pitfall 12 (the redraw loop is a variable-draw RNG site — must use a sub-stream), Pitfall 13f.
**Gate:** firms die and respawn over a decade with money conserved; no stale `FirmId` ever resolves; firm count stays 20; per-`(slot,gen)` log groups are complete and non-overlapping; net respawn cash flow ≈ 0; bankruptcies/year bounded.

### Phase 11: Calibration, burn-in and full acceptance
**Rationale:** A phase no source document had, and the one the literature guarantees is needed: **all initial conditions and the total money stock are unspecified in every paper**, and the total money stock is the free parameter that decides whether the economy inflates or deflates. Burn-in must be evidence, not the brief's number.
**Delivers:** initial money stock / liquidity / price / wage calibration; burn-in justified by a half-window mean-and-variance comparison and stability at 2× and 4×; the full section-7 suite **with every counter-check from Pitfall 15** (variance floors, max run-length of identical values, ACF decay and stationarity, distribution over firms above an age threshold, bankruptcy-excluded unemployment variance); diagnostic charts; a second-OS/toolchain CI reproducibility job.
**Gate:** a 3,650-tick run passes every section-7 criterion with the first 250 ticks discarded, and every degenerate-pass counter-check also passes.

### Phase Ordering Rationale

- **Vocabulary → ledger → harness → economics.** Phases 1–4 contain zero economics by design. Every `DESIGN`-tagged pitfall (sub-streams, atomic transfer, split remainder, invariant-as-phase, provenance schema, liveness) is impossible or expensive to retrofit, and without them every later economic bug is also an accounting mystery.
- **Money movement is introduced in dependency order:** goods exist (P5) → money moves firm→household via wages (P6) → money moves household→firm via purchases (P7) → the loop is closed by dividends (P8). Only then is there a circular flow for the reactive rules (P9) to react to.
- **Dividends before planning, not after.** Both PITFALLS and ARCHITECTURE state this independently and it is the ordering most likely to be violated by a "get something moving first" instinct.
- **Bankruptcy last among the economics** because it consumes three earlier phases' outputs (arena, worker release, ownership) and because a firm that dies before the economy is alive teaches nothing.
- **The harness grows alongside the sim** rather than arriving at the end, so each phase's gate is a real, automated check rather than an eyeballed chart.
- **Calibration is a phase, not a task.** Treating it as acceptance polish is how a build ends up with a passing run at exactly one point in parameter space.

### Research Flags

Phases likely needing `--research-phase` during planning:
- **Phase 6 (labour market):** the reservation-wage / wage-step parameter coupling is the widest-sensitivity region in the model, both error signs produce plausible-looking pathologies, and the Lengnick values are grade B rather than read from the paper. PITFALLS flags this explicitly.
- **Phase 8 (ownership / accounting / dividends):** where the brief is least specified, where the project deviates most from the literature (single-owner firms), and where the most damaging failure mode originates. PITFALLS flags this explicitly.
- **Phase 11 (calibration):** the initial conditions and total money stock are genuine literature gaps; this phase is a search, not an implementation.
- **Phase 1 (light flag only):** the RNG sub-stream keying scheme and the `f64`-vs-`i64`-milli-units decision for `expected_demand` are both open and both cheap to research, expensive to change.

Phases with standard patterns (skip research-phase):
- **Phases 2, 3, 5:** ledger, pipeline, log schema and production are fully specified by ARCHITECTURE with compile-verified patterns.
- **Phase 4:** pandas/pytest/statsmodels usage is completely conventional; STACK gives the per-criterion tooling table.
- **Phases 7, 9, 10:** well-determined by the brief plus the recovered parameter table — the numbers already exist above.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | **HIGH** | Versions from the crates.io/PyPI APIs on 2026-08-30; behavioural claims read from extracted crate tarballs and the local rustc 1.94.1 std source; the overflow and RNG claims were compiled and executed first-hand. |
| Features | **MEDIUM** | Ranges from grade A (Caiani values from the authors' own `S120/jmab` and `S120/benchmark` repos) to grade B (Lengnick values from an annotated replication citing Table 1 and Eq(5)–(12) inline) to explicit GAPs. **The primary paper PDFs were egress-blocked** — no Lengnick value here was read from the paper. |
| Architecture | **HIGH** | Every Rust pattern was compiled and executed on rustc 1.94.1 before being written down. Weaker spots: `rand` distribution value-stability (MEDIUM, from issue #786) and log-volume estimates (MEDIUM, derived not measured). |
| Pitfalls | **MEDIUM** | Rust-specific claims (HashMap `RandomState`, `StdRng`, the 1.81 sort replacement, Cargo's overflow-check default) are solid and cross-checked. Economic claims are search-result synthesis cross-checked across independent sources, not direct quotation, because the same egress proxy blocked the papers. |

**Overall confidence:** MEDIUM-HIGH. The *engineering* is close to certain; the *parameters* need one verification pass.

### Gaps to Address

**Needs verifying (a value exists; we did not read it from the source):**
- **Lengnick Table 1** — the single highest-value remaining verification action. Spot-check α, γ, θ, υ, δ, φ, ϑ, χ, n, ζ, ψ, β, π against the published table. Open-access PDF: `https://www.econstor.eu/bitstream/10419/45012/1/654079951.pdf` (blocked in this session). *Handle:* an explicit early task in Phase 1 or Phase 6; it de-risks the widest-sensitivity parameter group in the model.
- **Lengnick Eq (12)** — a further consumption bound, text not recovered. The replication reports it near-vacuous and omits it with no observable effect. *Handle:* omit; do **not** substitute an invention.
- **BAM's book-stated labour contract length** — the reference NetLogo uses `8 + Poisson(10)`, unconfirmed against the book. *Handle:* moot under the recommended indefinite-contract PROJECT CHOICE.
- **Search sample sizes in published calibrations** — PITFALLS could only describe these as "small single digits"; FEATURES recovered 5 and 5 from reference implementations. *Handle:* the recovered values supersede, but confirm in the Table 1 pass.
- **Whether the Lengnick calibration survives at 200/20 rather than 1000/100** — the 10:1 ratio is preserved but small-N effects (20 firms, 5% trims) are not addressed anywhere in the literature. *Handle:* expect calibration work in Phase 11; this is the most likely source of "unrealistic output" that is *not* a coding defect.

**Must be chosen, not looked up (no published value exists):**
- **All initial conditions** — household liquidity, firm liquidity, initial price, initial wage, initial reservation wage, initial inventory, initial expected demand (which must be **> 0** or the price rule divides by zero). The replication annotates each "Value not stated in paper". *Handle:* Phase 11, with the derived starting point `w = 63 × p` (one month's output of one worker at λ=3) and household liquidity at a small multiple of the monthly wage.
- **Total money stock** — exogenous and unspecified in every source; the free parameter that decides inflation vs deflation. *Handle:* explicit exploration in Phase 11. The 250-tick burn-in is the right instinct but is not a substitute for choosing a coherent money stock.
- **Stagger assignment scheme, owner-funded entry, single-owner firms, contract length** — all PROJECT CHOICE with no published counterpart. *Handle:* record each in PROJECT.md's Key Decisions as a deviation, not as "following the literature".
- **Weekly/daily rate conversions** — grade C arithmetic, not published. *Handle:* they close automatically if a 21-day cadence is adopted instead.
- **Purchased food: consumed immediately or held as household inventory?** The brief's tick order has no consumption phase; this determines whether the goods identity has a household-stock term. *Handle:* model `consume` explicitly either way so the identity keeps one shape; resolve in Phase 7.

**Also unresolved, cheap to settle in-phase:** `f64` vs `i64` milli-units for `expected_demand` (Phase 9); the dividend remainder policy — ascending-ID vs rotating offset (Phase 8); `insta` snapshot window size (Phase 3); whether `firm_panel` carries books-derived columns (Phase 3).

## Sources

### Primary (HIGH confidence)
- **crates.io and PyPI registry APIs**, queried 2026-08-30 — every version, publication date and dependency requirement.
- **Crate source tarballs** from `static.crates.io`, extracted and read: `rand-0.10.2` (`src/rngs/mod.rs` portable-vs-non-portable taxonomy, `src/rngs/std.rs` `StdRng` disclaimer, `src/seq/mod.rs` 32/64-bit index portability, `CHANGELOG.md`), `rand_chacha-0.10.0`, `toml-1.1.4`.
- **Local rustc 1.94.1 std/core source** — the verbatim "Unspecified precision … non-deterministic" blurb and the exact enumeration of the 31 affected `f64` methods; "each `HashMap` instance uses a different seed"; confirmed no entropy source in `core/src/slice/sort`.
- **First-hand compile-and-run experiments** on rustc/cargo 1.94.1 — default release profile silently wraps `i64::MAX - 1 + 6`; `overflow-checks = true` panics; `ChaCha8Rng::seed_from_u64` identical across runs and profiles; `rand::rng()` fails to compile under the chosen feature set. Every architecture pattern (split borrows, central ledger, generational arena, `const PHASES` table, `dyn Sink`) was compiled and executed.
- **`S120/jmab` and `S120/benchmark`** (© Caiani and Godin, the paper's own authors — equivalent to an appendix) — `SimpleAdaptiveExpectation.java`, `AdaptiveMarkUpOnAC`, `AdaptiveWageStrategy`, `TargetExpectedInventoriesOutputStrategy`, `modelBenchmark_light.xml`.

### Secondary (MEDIUM confidence)
- **`newwayland/baseline-economy`** — a Mesa replication of Lengnick with parameters annotated `# Calibration values (Table 1)` and equations cited `Eq(5)–(12)`, plus `notes/issues.md` cataloguing exactly where the paper is silent. Source for every Lengnick number above (grade B).
- **`alexplatasl/BAMmodel`** — full ODD protocol (initialisation table, submodels 1–44) and the reference NetLogo. Source for bankruptcy, entry sizing/pricing, contract-at-hire, cheapest-first purchase (grade B).
- **`rust-random/rand` issue #786** — distribution value-stability across versions.
- Rustonomicon borrow-splitting; `slotmap`, `schemars`, `typed-index-collections` docs; Tratt on Rust's two kinds of assert.
- ABM-literature synthesis on activation-order artefacts (Huberman & Glance 1993 vs Nowak & May 1992), the ABM replication crisis, and Gualdi/Tarzia/Zamponi/Bouchaud on unemployment pinning as a *generic* property of minimal macro ABMs.

### Tertiary (LOW confidence — needs validation)
- **Blocked by this environment's egress proxy** (listed so the verification step knows where to look): the Lengnick JEBO/EconStor PDFs, the Caiani et al. PDF, sim4edu, arXiv, RePEc, SSRN, JASSS, CoMSES, `docs.rs`, `doc.rust-lang.org`. Where docs were unreachable, claims were verified against the corresponding **source**, which is the stronger citation — but no economic paper was read directly.
- Deflationary-stall *timescales* ("1–3 simulated years") are reasoned arithmetic, not measured. The mechanism is certain; the speed should be measured in the first runs.
- Bouchaud phase-diagram findings come from Mark-0, which has a credit channel this build lacks; the hiring/firing-asymmetry result is applied, the credit-ceiling axis is not.

---
*Research completed: 2026-08-30*
*Ready for roadmap: yes*
