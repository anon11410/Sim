# Pitfalls Research

**Domain:** Minimal closed-economy agent-based macroeconomic simulation (Lengnick / BAM class), Rust core + Python acceptance harness
**Researched:** 2026-08-30
**Confidence:** MEDIUM overall (see per-pitfall tags; primary PDFs were egress-blocked, so literature claims come from search-result synthesis cross-checked across independent sources rather than direct quotation)

---

## How to read this document

Every pitfall carries a **Catch** tag naming the cheapest mechanism that reliably detects it. The downstream roadmap should turn each tag into a concrete artefact:

| Tag | Means | Becomes |
|-----|-------|---------|
| `INVARIANT` | Checked every tick inside the sim, halts on violation | A function in the invariants step |
| `TEST` | Unit or property test, runs in CI | A `#[test]` / `proptest` case |
| `METRIC` | Computed by the Python harness from the logged series | An acceptance criterion with a numeric threshold |
| `DESIGN` | Cannot be tested for after the fact — must be decided before code exists | A Key Decision in PROJECT.md |

Phase labels below are indicative and use this shape; map them onto whatever the roadmap actually names:

- **P1 Foundations** — money type, config, seeded RNG, ID layout, structured logging, invariant harness
- **P2 Tick loop & firms** — tick order, weekly staggered planning, adaptive expectations, production
- **P3 Labour market** — firm sampling, offers, reservation wages, wage rule
- **P4 Goods market** — firm sampling, cheapest-first with fall-through, consumption budget
- **P5 Firm accounting** — profit, dividends, bankruptcy, respawn
- **P6 Acceptance** — Python harness, 10-year run, calibration

**A note on the project's stance.** The brief is right that unrealistic output is a defect rather than a discovery — but with one important qualification the research makes explicit. Gualdi, Tarzia, Zamponi & Bouchaud's *Tipping points in macroeconomic agent-based models* (arXiv:1307.5319; JEDC 2015) shows that models of exactly this class have a **phase diagram**: a genuine, robust phase transition between a "good economy" with low unemployment and a "bad economy" with high unemployment, plus distinct residual-unemployment and endogenous-crisis phases. So pinned unemployment is not always a bug in the code — it can be a correct simulation of a bad region of parameter space. The practical consequence for this build: **you must be able to tell those two cases apart**, and the only way to do that is to have the invariants clean first, so that when the economy misbehaves you know the arithmetic is right and the parameters are wrong. That ordering — accounting correctness before economic tuning — should drive phase ordering.

---

## Critical Pitfalls — Economics

### Pitfall 1: The deflationary stall (firms accumulate money, households run dry)

**Confidence:** HIGH on the mechanism (it is forced arithmetic in a closed fixed-money economy), MEDIUM on the timescale.

**What goes wrong:**
Aggregate household cash falls monotonically. Because the household spending rule is a fraction of cash, spending falls with it; falling spending is falling firm revenue; firms respond by cutting production, wages and headcount; falling wages are falling household income. The economy shrinks toward a state where households hold almost nothing, nearly nothing trades, and unemployment sits near 100% — but every conservation invariant still passes, because no money was lost. It moved.

**Why it happens:**
In a closed economy with a fixed money stock `M` and no credit, `M = household_cash + firm_cash` identically. Household income is the wage bill; firm income is household spending. If firms are on average profitable — revenue exceeds wage bill — then `firm_cash` grows every tick and `household_cash` shrinks by exactly the same amount. **There is no equilibrating force in the model other than the payout rule.** Long-run stationarity of this economy *requires* aggregate firm profit to average ~zero, and the dividend rule is the only mechanism that can deliver that. Omitting dividends, or paying them from a badly specified buffer, is therefore not a missing feature — it is a missing equilibrium condition.

The reason it is the single most common way a first build dies is that it does not look like a bug. Prices, wages and employment all move smoothly and plausibly for the first year or two. Nothing asserts. It just quietly dies.

**How to avoid:**
1. **Log the sectoral money split from tick 1.** `household_cash_total`, `firm_cash_total`, and `firm_share = firm_cash_total / M`. This is the diagnostic; add it before you add dividends.
2. **Define the working-capital buffer in flow terms, not nominal terms.** A buffer expressed as "keep 50,000 cents" is a *permanent nominal sink* — as the price level drifts, its real size drifts, and it stops being a buffer and starts being a hoard. Express it as a multiple of the firm's recent wage bill (e.g. `buffer = k * last_wage_bill`, `k` in the config). A flow-denominated buffer self-scales with the price level and cannot become a sink.
3. **Pay out the entire excess above the buffer, not a fraction of it.** A rule like "pay out 50% of cash above the buffer" leaves a geometrically-decaying residue that still accumulates in aggregate if firms are persistently profitable. Full drain above a flow-denominated buffer is the version that closes the loop.
4. **Match the payout cadence to the spending cadence.** If firms pay dividends monthly but households spend daily from cash, money spends most of its life parked. Daily or weekly payout is safer for this build.
5. **Route every other firm-side cash exit to a household too.** Bankruptcy residual → owner. Entrant endowment → from a household. Nothing is ever deleted or conjured.

**Warning signs (from the logs):**
- `firm_share` of total money trending upward with a positive slope over any 200-tick window. **This fires 1-2 years before the price level or unemployment shows anything.** It is the earliest available signal by a wide margin.
- Median household cash falling monotonically.
- Aggregate transaction volume (cents changing hands per tick) trending down while `M` is constant — i.e. falling velocity.
- Mean firm cash rising while mean firm inventory also rises (firms are hoarding *and* not selling).

**Catch:** `METRIC` (regression slope of `firm_share` over the retained window must be indistinguishable from zero) + `INVARIANT` (a soft guard: `firm_share < 0.5` — in a healthy run firms hold a small working balance, so crossing half the money stock is prima facie broken) + `DESIGN` (flow-denominated buffer, full drain).

**Phase to address:** P1 (log the split), P5 (dividend rule), P6 (the slope metric).

---

### Pitfall 2: Price spiral up (ratchet inflation)

**Confidence:** MEDIUM-HIGH.

**What goes wrong:**
The price level rises without bound. Wages chase prices, prices chase unit labour cost, unit labour cost chases wages. In integer cents this eventually overflows or hits absurd magnitudes; long before that, real balances are crushed, real demand collapses and the run is worthless.

**Why it happens:**
Almost always because the adjustment rule was implemented as a **deterministic, every-period, full-step** change instead of a **probabilistic, partial** one. In this model class firms adjust price and wage *with a probability* and *by a small random percentage* drawn per firm per adjustment. That stochasticity is not decoration — it is the damping. It means that at any moment only a minority of firms are moving, in both directions, with heterogeneous step sizes. Replace it with "every firm raises price by 2% whenever inventory is below the buffer" and you have converted a damped adjustment into a ratchet: whenever the aggregate is on the low-inventory side, every firm moves up together, and the aggregate stays on the low-inventory side because raising price does not create inventory.

Second cause: an **asymmetric rule**. If the upward step is larger than the downward step (or the upward trigger band is wider), the random walk has positive drift by construction.

Third cause: the price floor being applied to the *offered wage* as well, so wages can only ever go up.

**How to avoid:**
- Implement price and wage adjustment as: *with probability θ, multiply by `1 ± u`, where `u ~ Uniform(0, ū)` drawn fresh*. Both θ and ū in config. This is the published shape; do not simplify it.
- Make the up-step and down-step distributions identical in magnitude. Any asymmetry should be a deliberate, documented, configured choice — not an accident of the code.
- Assert in a test that over many draws the expected log-price change is zero when the trigger fires up and down equally often.
- Use `i64` cents with `overflow-checks = true` in the release profile so that a runaway is a loud panic and not a silent wrap. (Cargo defaults `overflow-checks` to **off** in release — this must be set explicitly, see Pitfall 10.)

**Warning signs (from the logs):**
- Mean price with a positive slope sustained over >500 ticks. In a fixed-money closed economy with no productivity growth, the long-run price level should be roughly trendless.
- Mean nominal wage rising at essentially the same rate as mean price, with real wage flat — the signature of a nominal spiral rather than a real change.
- The fraction of firms adjusting price upward exceeding the fraction adjusting downward, persistently.

**Catch:** `METRIC` (price-level trend test over the retained window; up/down adjustment counts logged per tick and asserted roughly balanced) + `TEST` (symmetry property test on the adjustment function).

**Phase to address:** P2 (price rule), P3 (wage rule), P6 (trend metric).

---

### Pitfall 3: Price collapse to zero, and the unit-labour-cost floor

**Confidence:** HIGH on the mechanism.

**What goes wrong:**
Prices ratchet down until they hit 1 cent or 0. At 0, goods are free: households empty every shelf, firms book zero revenue, cannot make payroll, and the entire firm population goes bankrupt within a handful of ticks. At 1 cent the economy is nominally alive but is really a degenerate barter system.

**Why it happens:**
The downward branch of the price rule has no lower bound, or the bound is wrong. The correct bound is **unit labour cost** — the firm should not price below the cost of producing a unit — and getting it wrong has four distinct modes:

1. **Floor missing entirely.** Straight collapse. Obvious once you look.
2. **Divide-by-zero when output is zero.** `ulc = wage_bill / output` is undefined for a firm with no workers or no production. Integer division by zero panics in Rust (good) — but the "fix" people reach for is `if output == 0 { ulc = 0 }`, which silently removes the floor exactly for the distressed firms that most need it, and they then race to zero and take the whole distribution with them. Correct fallback: carry forward the firm's last valid ULC, or fall back to `wage_rate / productivity` (the *planned* unit cost, which is defined even at zero output).
3. **Wrong denominator — planned vs realised output.** If wages are paid for the whole period but output is only what was actually produced, using planned output understates ULC and lets the firm price below cost. Use realised output, with the fallback above.
4. **Timing: floor computed before wages are set this tick.** In the specified tick order (planning → labour → production → wages → goods market), the price is chosen at planning time, before this tick's wage bill exists. That is fine and correct — but the ULC must then be computed from the *previous* period's realised wage bill and output, and the code must say so explicitly rather than accidentally reading a field that is mid-update.

**The subtle fifth mode, and the important one:** a floor that binds *too often*. If most firms sit exactly at their ULC floor, the cross-sectional price distribution collapses toward a single value. That is catastrophic for this model, because **price dispersion is what makes the goods-market search mean anything.** With all prices equal, "sample 5 firms and buy from the cheapest" is a coin flip; the selection pressure that disciplines firms disappears; the model becomes random matching. The economy looks superficially fine — prices are stable! — but the mechanism is dead. Log `fraction_of_firms_at_price_floor` every tick. In a healthy run it should be low single-digit percent. Near 100% means the price rule is inert and the floor is doing all the work.

**How to avoid:**
- Floor as an explicit, named, tested function with the zero-output fallback specified in config, not inline in the price rule.
- Log ULC per firm alongside price, and log the at-floor count.
- Property test: for any firm state, `new_price >= floor(firm)` and `floor(firm) >= 1` cent.

**Warning signs (from the logs):**
- `min_price` across firms reaching 1 or 2 cents.
- `fraction_at_floor` rising above ~20%.
- Coefficient of variation of firm prices trending toward zero (see also Pitfall 7).
- Mean firm profit persistently negative — firms structurally selling below cost.

**Catch:** `INVARIANT` (`price >= 1` for every firm every tick; halt at 0) + `TEST` (floor property test incl. zero-output case) + `METRIC` (`fraction_at_floor`, price CV).

**Phase to address:** P2 (price rule and floor), P6 (dispersion metrics).

---

### Pitfall 4: Unemployment pinned at 0% or 100%

**Confidence:** MEDIUM-HIGH — grounded in Gualdi/Tarzia/Zamponi/Bouchaud, which finds this to be a *generic* property of minimal macro ABMs, not an implementation quirk.

**What goes wrong:**
Unemployment goes to a corner and stays there with zero variance. Either everyone is employed forever (and wages sit on the floor and nothing ever changes), or nearly everyone is unemployed forever (and the economy deflates into the stall of Pitfall 1).

**Why it happens:**
The tipping-points literature is unambiguous about the dominant driver: the transition between the low-unemployment and high-unemployment phases is **generically induced by an asymmetry between the rate of hiring and the rate of firing of firms.** Their Mark-0 phase diagram identifies four regimes — full employment (FE), full unemployment (FU), residual unemployment (RU), and endogenous crises (EC) — selected by a small number of parameters, with the FU phase also being the deflationary one. Their model has a credit channel this build lacks, so the second axis (the credit ceiling Θ) does not apply here; but the hiring/firing asymmetry axis does, directly.

In this build's terms, the hiring/firing asymmetry lives in the relationship between:
- how many workers a firm adds when inventory is below the lower buffer,
- how many it sheds when inventory is above the upper buffer,
- how likely a posted vacancy is to be filled given the labour-market sample size,
- and how fast reservation wages move.

The reservation-wage parameters are where a first build most often breaks it, and the relationship that matters is **the decay rate per unemployed tick versus the firm's wage-raise step per unfilled-vacancy period.**

- **Decay faster than the firm's wage step → pinned at 0%.** The unemployed undercut themselves faster than firms bid up. Every vacancy is filled instantly at a falling wage, the wage rule never needs to raise, wages settle on the floor, and unemployment is structurally zero. The economy is technically "working" but has no labour-market dynamics at all.
- **Decay slower than the wage step, or zero → pinned high.** Unemployed households never lower their expectations, so offers never clear their reservation wage, so vacancies stay unfilled, so firms cannot produce, so they cannot sell, so they cut wages further — a ratchet into FU.
- **Decay to zero with no floor → pinned at 0% *and* wage collapse.** If the reservation wage decays multiplicatively with no positive floor, after enough unemployed ticks a household will accept 1 cent. Everyone is employable at any price; the labour market stops constraining anything.
- **Employed rise rate exceeding the firm wage step → pinned high.** Employed households' reservation wages outgrow every available offer, they quit or reject, and they join the unemployed pool with expectations above the market.

**How to avoid:**
- Treat `(decay_rate_unemployed, rise_rate_employed, firm_wage_step, firm_wage_adjust_probability)` as **one coupled parameter group**, documented as such in the config with a comment stating the required ordering. They cannot be tuned independently.
- Give the reservation wage a **positive floor** in config — a subsistence-like minimum below which it does not decay. Non-negotiable.
- Take the published parameter values as the starting point rather than inventing them (the brief already commits to this for the rise rate). Where the published value is unavailable, choose values in the same order of magnitude and record that they are provisional.
- Bound hires per firm per period (see Pitfall 8) so hiring cannot be arbitrarily fast relative to firing.
- Run a small parameter sweep over the coupled group as part of P6 and plot the unemployment mean — the point is to *see the phase boundary* and confirm the chosen operating point is not sitting on it. A parameter set two percent from a first-order transition is not a calibration, it is a time bomb.

**Warning signs (from the logs):**
- Unemployment with **zero variance over 30+ consecutive ticks** — the cheapest possible detector and it should be an explicit acceptance failure, at both corners.
- Unfilled vacancies persistently 0 (labour market too easy) or persistently equal to total vacancies posted (too hard).
- Distribution of `reservation_wage / best_offer_seen` — should straddle 1. If it is entirely below 1, everyone always accepts; entirely above, no one ever does.
- Mean reservation wage trending monotonically to its floor.

**Catch:** `METRIC` (unemployment band + a rolling-variance floor; vacancy fill rate; reservation/offer ratio distribution) + `DESIGN` (positive reservation wage floor; the coupled parameter group documented together).

**Phase to address:** P3 (all of it), P6 (the sweep).

---

### Pitfall 5: Dead steady state — the model converges and stops moving

**Confidence:** MEDIUM.

**What goes wrong:**
No crash, no spiral, no pinning — just a fixed point. Unemployment sits at 6.2% every tick. All firms charge the same price. Output is a flat line with tiny numerical jitter. The acceptance criteria on conservation, reproducibility and stability all pass, and the model is worthless.

**Why it happens:**
Endogenous fluctuation in this model class is not free. It is sustained by a specific set of mechanisms, and each one is easy to accidentally disable:

| Mechanism that creates fluctuation | How it gets killed |
|---|---|
| Cross-sectional heterogeneity that search friction prevents from equalising | Sample size too large (Pitfall 6); identical initial conditions; identical shocks |
| Integer lumpiness of hiring/firing — a firm hires whole workers | Fractional/continuous labour, or "adjust headcount toward target smoothly" |
| Stochastic, partial price/wage adjustment (the θ and `u` draws) | Replaced by deterministic full-step adjustment (also causes Pitfall 2) |
| Mismatch between adaptive expectations and realised demand | λ too low → expectations frozen, firms never respond; λ = 1 → expectation is just last sales, no smoothing, can be too jumpy |
| Firm-specific realised demand from actual matches | Replaced by "aggregate demand ÷ N firms", which makes every firm identical forever — the single most destructive accidental simplification |
| Bankruptcy/entry shocks | Suppressed, or so rare they never fire |

The last one in the table deserves emphasis. If at any point a firm's sales are computed from an aggregate rather than from the individual household purchases it actually received, the entire heterogeneity engine is switched off and the model becomes a lagged representative-agent difference equation. It will converge to a fixed point and it will look completely reasonable while doing so.

Also worth stating: **white noise is a failure too.** A series with zero autocorrelation at every lag is not "fluctuating", it is uncorrelated jitter with no propagation. The acceptance target is a *positive, decaying* autocorrelation function — evidence that a deviation persists for a while and then dies, which is what a business cycle is.

**How to avoid:**
- Log cross-sectional dispersion every tick: **standard deviation (or CV) of firm prices, firm employment, firm inventory, and household cash.** These four series are the model's vital signs. A CV trending to zero is the model dying, and it shows up long before the aggregate series flatten.
- Initialise agents with dispersion drawn from the seeded RNG — never all-identical. Identical initial conditions both maximise the burn-in transient and risk permanent lockstep.
- Assert, in a test, that firm `last_sales` is derived from the sum of that firm's logged purchase events, not from any aggregate.
- Sweep λ; check that the output autocorrelation is positive and decaying at both the chosen value and its neighbours.

**Warning signs (from the logs):**
- CV of any of the four dispersion series decaying toward zero.
- Variance of the unemployment / output / price series over the retained window below a threshold.
- Output autocorrelation ≈ 0 at all lags (white noise) or ≈ 1 at all lags (random walk / trend, i.e. non-stationary — see Pitfall 13).
- All 20 firms reporting the same price to the cent.

**Catch:** `METRIC` (dispersion CV series must stay above a floor; output ACF must be positive at lag 1 and decay; variance floors) + `TEST` (`last_sales` provenance) + `DESIGN` (dispersed initialisation).

**Phase to address:** P1 (dispersion logging), P2 (expectations, dispersed init), P6 (ACF and variance metrics).

---

### Pitfall 6: Synchronisation artifacts and activation order

**Confidence:** MEDIUM-HIGH — this is one of the best-documented artefact classes in the ABM literature.

**What goes wrong:**
Two distinct failures, often confused.

**(a) Synchronised decision cadence produces a fake business cycle.** If all 20 firms replan on the same day, then on that day the whole economy moves at once: prices all step, headcounts all step, inventories all step. The aggregate series then has a clean periodic component at exactly the planning period. It looks like a cycle. It is the schedule. This is the classic ABM artefact: Huberman & Glance (1993) famously showed that the spatial patterns in Nowak & May's (1992) work vanished under asynchronous updating — the reported steady states were artefacts of synchronised activation. A replication study of a civil-unrest model found statistically significant differences in emergent population behaviour arising purely from the activation pattern. The brief's weekly stagger is the correct response and should be treated as load-bearing rather than as a detail.

**(b) Fixed within-tick agent ordering produces a systematic distributional artefact.** If households shop in ID order 0..199 every single tick, household 0 always gets first claim on the cheapest firm's inventory and household 199 always eats the stockouts. Over 3,650 ticks that is a systematic wealth transfer perfectly correlated with agent ID. You get a smooth, plausible-looking, entirely spurious wealth distribution — and if anyone ever plots wealth against ID they will see a monotone ramp. The same hazard applies to the labour market (who gets the vacancy) and to wage payment when a firm has insufficient cash (who gets paid). The literature is direct about the magnitude here: one study found the dependence between firm growth rate and firm size **reversed sign** between random activation and uniform activation.

**Does activation order need shuffling? Yes. Does it threaten determinism? No** — provided the shuffle comes from the single seeded RNG. This is precisely the "random order vs. random sequence" distinction: a Fisher-Yates shuffle over a `Vec<u32>` of agent indices, driven by the seeded RNG, is fully reproducible and consumes a fixed, known number of draws (`n-1`). What *does* threaten determinism is shuffling by sorting on a random float key (tie hazards, float comparison) or by iterating a `HashSet`.

**Other synchronisation hazards specific to this build:**
- **Cohort lockstep.** Staggering firms by `id % 7` fixes the aggregate artefact but leaves 7 cohorts of ~3 firms that still move together. Prefer a per-firm offset drawn once at initialisation from the seeded RNG and held fixed for the run — deterministic, but not correlated with ID.
- **Reservation wage updates all firing on the same day** (e.g. monthly) reproduces the same artefact in the labour market.
- **Dividends paid on a common day** creates a periodic demand pulse that shows up in the price series at exactly the dividend period.
- **Bankruptcy checks batched at one point in the week** clusters entry/exit shocks.

**How to avoid:**
- One freshly drawn permutation per market per tick, from the seeded RNG, applied to household order in the goods market, job-seeker order in the labour market, and worker order in wage payment.
- Per-firm planning offsets drawn at init, not derived from ID.
- **Acceptance check:** the autocorrelation function of the aggregate series must not have a distinguishable spike at exactly the planning cadence (7 ticks) or at any other schedule period used in the model (28, dividend period). This is a directly computable, unambiguous artefact detector and belongs in the harness.
- Property test: the generated permutation is a valid permutation of `0..n` (every index exactly once).

**Warning signs (from the logs):**
- ACF or periodogram spike at lag 7 (or whatever the cadence is) that is much larger than neighbouring lags.
- Household terminal wealth correlated with household ID (Spearman correlation materially different from zero).
- Firm outcomes correlated with firm ID.

**Catch:** `METRIC` (ACF spike at cadence; wealth-vs-ID rank correlation ≈ 0) + `TEST` (permutation validity; fixed RNG draw count) + `DESIGN` (shuffle from the seeded RNG; per-firm offsets at init).

**Phase to address:** P1 (shuffle utility + RNG discipline), P2 (stagger), P3/P4 (apply per market), P6 (artefact metrics).

---

### Pitfall 7: Search friction mis-sized — too much information or too little

**Confidence:** MEDIUM.

**What goes wrong:**
The brief is right that search frictions are the point, but the sample size is a genuine knife-edge and both directions fail.

**Sample too large (approaching perfect information):** every household finds the global cheapest firm. Price dispersion collapses, because no firm can hold a price above the minimum and keep customers. Once dispersion is gone the search mechanism is a coin flip and selection pressure disappears (this is the same endpoint as the too-tight price floor in Pitfall 3, reached by a different road). Before it gets there you typically see violent oscillation: the cheapest firm is swamped, stocks out, its customers all cascade simultaneously to the next-cheapest, which stocks out, and so on — a wave sloshing around the firm population. The literature on Diamond-style search is instructive here in the other direction too: even arbitrarily small search costs are enough to change the equilibrium qualitatively, which is why this parameter is so sensitive.

**Sample too small (1, or 2 with no fall-through):** purchase becomes essentially random. Price plays no role in allocation, so a firm that raises price loses nothing and the price rule has no selection pressure behind it. Prices random-walk, dispersion explodes, and the ULC floor becomes the only anchor in the model.

**The threshold.** This model class uses a small constant — order 5 firms sampled for goods and a similar handful for jobs, out of tens to hundreds of firms. Published BAM calibrations use very small numbers (single digits) for both the goods-market and labour-market sample sizes. There is no sharp universal threshold; the operational rule is:

> **The sample size must be a small absolute constant, never a fraction of N.**

If it is coded as `n_firms / 4`, then friction vanishes as the model scales, and the "scaling beyond 200 agents" exercise listed as a later step will silently change the economics rather than just the runtime. Given that this build's whole purpose is to be a foundation, hardcoding a fraction here is a forward-compatibility defect.

**A related and easily-missed point:** the brief's design — sample, buy cheapest-first, fall through to next-cheapest on stockout — implicitly creates *loyalty*, because a household that found a good vendor tends to keep finding it. If instead a fresh independent sample is drawn every tick with no memory, demand at each firm is much noisier and firm-level series become jumpy. Whether preferred-supplier memory is kept is a modelling decision that should be made deliberately and recorded, not left to fall out of the implementation.

**How to avoid:**
- Sample size as an absolute integer in config, with a comment forbidding `N`-dependence.
- The fall-through on stockout is mandatory, not optional — without it, unmet demand vanishes and both the stockout signal and the anti-concentration mechanism (Pitfall 8) are lost.
- Sweep sample size in P6 and observe the price CV; the chosen value should sit in a flat region, not on a cliff.

**Warning signs (from the logs):**
- Price CV → 0 (too much information) or growing without bound (too little).
- Stockout events concentrated in one or two firms and highly clustered in time (the sloshing wave).
- Fraction of household budget left unspent due to stockout — should be small but non-zero. Exactly zero means friction is not binding; large means the fall-through is broken.

**Catch:** `METRIC` (price CV band; unmet-demand fraction band; stockout concentration) + `DESIGN` (constant, not a fraction of N).

**Phase to address:** P3/P4 (sampling), P6 (sweep).

---

### Pitfall 8: Firm size degeneracy — one winner, or twenty clones

**Confidence:** MEDIUM.

**What goes wrong:**
Either employment and sales concentrate into one or two firms while the rest idle at zero, or all firms end up identical and the size distribution has no dispersion at all. Empirically the target is a right-skewed, persistent distribution — this model class is known to reproduce power-law-ish firm size distributions from heterogeneous-agent interaction, and a closed conserved-money economy with agents migrating between firms is a textbook generator of that shape.

**Why concentration happens:**
Cheapest-first search is a positive feedback: cheap firm → more customers → more revenue → hires more → produces more → holds more inventory → less pressure to raise price → stays cheap. With constant returns to scale (output = productivity × workers) there is no cost-side brake at all. The brakes in this model are:

1. **Stockout fall-through.** A firm can only sell what it has. Excess demand cascades to the next-cheapest. This is the primary anti-concentration mechanism and it only exists if fall-through is implemented (see Pitfall 7).
2. **Finite labour supply.** 200 households cap total employment, but that does not stop *one* firm absorbing most of them.
3. **A bound on hires per firm per period.** Without it, a firm whose expected demand jumps can try to hire 150 workers in a single tick and, if the labour market is loose, succeed. Real firms cannot; neither should these. Bound hires per firm per period in config.
4. **The inventory buffer band.** A firm that overshoots demand builds inventory, cuts price and sheds workers — a genuine negative feedback, but only if the upper buffer is reachable.

**Why homogeneity happens:**
Identical initial conditions plus common shocks plus (fatally) firm-level demand computed from an aggregate rather than from realised matches (Pitfall 5). Also: if the price adjustment is deterministic, all firms in the same inventory regime make the identical move and stay identical forever.

**How to avoid:**
- Log three concentration statistics every tick: **Herfindahl index of employment**, **max firm share of employment**, and **Gini (or CV) of firm size**. Cheap to compute, and they diagnose both failure directions with one set of series.
- Bound hires per firm per period.
- Dispersed initialisation of firm size, price, wage, inventory and cash.
- Verify fall-through with a targeted test: a household whose cheapest sampled firm has zero inventory must buy from the next-cheapest in the same tick, not lose the purchase.

**Warning signs (from the logs):**
- Max employment share sustained above ~0.5, or HHI climbing monotonically.
- Gini of firm size trending to 0 (clones) or to 1 (monopoly).
- A firm with zero employment for many consecutive ticks that never recovers — a zombie. Zombies are how concentration presents before it completes.

**Catch:** `METRIC` (HHI, max share, size Gini, all with bands) + `TEST` (stockout fall-through) + `DESIGN` (hire cap, dispersed init).

**Phase to address:** P2 (hire cap, init), P4 (fall-through), P6 (concentration metrics).

---

### Pitfall 9: Bankruptcy and respawn artifacts

**Confidence:** MEDIUM.

**What goes wrong:**
The respawn rule is a phase-1 placeholder, and placeholders distort. Five distinct artefacts, in rough order of severity:

**(a) Respawn as a household→firm cash pump.** If the entrant is endowed from a random household's cash, every bankruptcy moves money from the household sector to the firm sector. That is not neutral — it is the deflationary stall of Pitfall 1 with a second engine bolted on. It compounds, and it is invisible unless you measure it. **Log every bankruptcy as an explicit money-movement event** (residual out to owner, endowment in from funder, both with amounts) so the harness can compute the *net* household→firm flow attributable to respawn over the run and assert it is small relative to `M`.

**(b) Respawn masking systemic distress.** Because firm count is pinned at 20 by construction, firm count can never signal anything. Every headline metric can look healthy while a third of the firm population is churning every year. The substitute signal is the **bankruptcy rate per simulated year**, and it must be an explicit acceptance metric with an upper bound (a few percent of firms per year is plausible; 30%/yr means the economy is not viable and the respawn rule is hiding it).

**(c) Respawn as fraudulent liveness — the most dangerous one.** A bankruptcy dumps its whole workforce into unemployment in one tick. If bankruptcies are frequent, the unemployment series will fluctuate beautifully, satisfy an "unemployment is not pinned" acceptance criterion, and be entirely an artefact of firm churn rather than of labour-market dynamics. **The acceptance harness must decompose this**: compute unemployment variance over ticks that are not within a few days of a bankruptcy event, and require it to still be non-trivial. Without that decomposition the liveness criterion is not testing what it claims to test. (See also Pitfall 16.)

**(d) Dead-on-arrival entrants.** An entrant initialised with `expected_demand = 0` will plan zero production, hire nobody, sell nothing, and go bankrupt again — a churn loop that inflates the bankruptcy rate and produces a permanently-empty firm slot. Seed entrants from the surviving population's median (expected demand, price, wage), not from zero.

**(e) The redraw loop as an RNG-consumption hazard.** PROJECT.md records the decision that respawn *redraws* when the sampled owner cannot fund a firm. That is a variable-length RNG consumption: the number of draws depends on the wealth distribution, so any change anywhere that shifts wealth also shifts the RNG stream from that point onward. Either cap the retries at a fixed count and consume exactly that many draws regardless of when it succeeds, or use a single wealth-weighted draw that consumes exactly one value. See Pitfall 13.

**(f) The ID-reuse trap for the log.** If a bankrupt firm's ID is reused by the entrant, the per-firm series in the log splices two different firms' lives together. The firm-size distribution, firm survival statistics and any per-firm chart are then silently wrong. Fix: log a **(slot_id, incarnation)** pair, incrementing incarnation on every respawn. The harness groups by the pair, not by slot.

**How to avoid:**
Beyond the above: **respawn in place — never remove from the `Vec`.** See Pitfall 12; this single decision eliminates a whole family of bugs at once.

**Warning signs (from the logs):**
- Bankruptcies per year above a small threshold.
- Net respawn-attributable household→firm flow accumulating.
- Firms whose lifetime is consistently short (churn loop).
- Unemployment variance collapsing when bankruptcy-adjacent ticks are excluded.

**Catch:** `METRIC` (bankruptcy rate/yr; net respawn cash flow; bankruptcy-excluded unemployment variance; firm lifetime distribution) + `DESIGN` (respawn in place; incarnation IDs; fixed-draw owner selection; median-seeded entrants).

**Phase to address:** P1 (event logging schema incl. incarnation), P5 (all of the rules), P6 (the decomposition metrics).

---

## Critical Pitfalls — Implementation (Rust, determinism, accounting)

### Pitfall 10: Money conservation leaks — the concrete list

**Confidence:** HIGH.

**What goes wrong:**
`sum(household_cash) + sum(firm_cash) != M`. This is the invariant the project cares most about, so here is the enumeration at the level of specific coding mistakes. Each line is a bug that has a distinct shape and a distinct fix.

| # | Leak mode | Concrete mistake | Fix |
|---|---|---|---|
| 1 | **Truncating integer division on a percentage** | `let raise = wage * 3 / 100;` truncates toward zero. Harmless alone — becomes a leak when the firm is debited an *untruncated* aggregate while workers are credited truncated amounts. | Compute each party's integer amount **first**, then debit exactly `sum` of those amounts. Never compute the aggregate independently. |
| 2 | **Dividend split remainder** | `let per_owner = profit / n;` discards `profit % n` cents. If the firm is debited `profit` and owners credited `n * per_owner`, cents are **destroyed** every payout, every tick. At 20 firms × 3650 ticks that is meaningful. | Largest-remainder method: floor each share, then distribute the leftover cents one at a time by descending remainder (tie-broken by ID for determinism). Assert `sum(shares) == total`. |
| 3 | **Pay before checking affordability** | `firm.cash -= wage_bill;` with insufficient cash. On `u64` this **wraps in release** (Cargo turns `overflow-checks` off in release by default) and conjures ~1.8×10¹⁹ cents. On `i64` it silently goes negative and the "no negative balances" invariant catches it — one tick later, after the state is corrupted. | Use `i64` **and** set `overflow-checks = true` in `[profile.release]`. Every debit goes through one `transfer(from, to, amount)` helper that asserts `amount >= 0` and `balance(from) >= amount` **before** mutating. |
| 4 | **Double-counted transfer** | The goods market debits the household and credits the firm; firm accounting *also* adds a separately-accumulated `sales_revenue` counter. Money is created equal to total sales. | **One mutation point.** All money movement goes through `transfer()`. Derived quantities (revenue, wage bill) are computed *from* recorded transfers, never accumulated in parallel with them. |
| 5 | **Partial payment path** | Firm can only cover part of payroll; loop pays `min(remaining, wage)` to each worker in order and stops. Two bugs at once: (a) the accounting/log records the *full* wage bill as an expense while only part moved — ledger and log disagree; (b) workers early in the iteration order are paid in full and late ones get nothing, which is the activation-order artefact of Pitfall 6 in its most damaging form. | Shuffle worker order from the seeded RNG; record the *actual* per-worker amount paid as a separate event each; derive the wage bill from those events. |
| 6 | **Cash transferred to a removed firm** | `Vec::swap_remove(i)` on bankruptcy moves the last firm into slot `i`. Every ID equal to `len-1` now refers to a different firm; any pending reference is silently wrong. A payment in flight is credited to the wrong firm or dropped. | **Never remove.** Reinitialise the slot in place (respawn in place). If genuine removal is ever needed, tombstone with `alive: bool` and never compact. This is a `DESIGN` decision — retrofitting it is a rewrite. |
| 7 | **Float in the budget fraction** | `let budget = (cash as f64 * 0.35) as i64;` reintroduces float into the money path — the exact drift the project banned. | Integer basis points: `let budget = cash * bps / 10_000;`. Truncation is safe here because a budget is a *cap*, not a transfer — nothing is destroyed by rounding a cap down. |
| 8 | **Rounding a price × quantity** | Exact for integer quantities. Becomes a rounding site the moment goods are divisible. | Keep goods in integer units. Record it as a constraint in the goods table now, before the second good arrives. |
| 9 | **Money conjured or deleted at bankruptcy** | Residual cash deleted with the firm; or the entrant's endowment materialised from nowhere. | Residual → owner via `transfer()`. Endowment → from a funding household via `transfer()`. Both logged as events. |
| 10 | **Goods mixed into the money invariant** | Adding inventory valued at cost into a "total wealth" conservation check. Goods are *created* by production and *destroyed* by consumption; they are not conserved. | Two separate invariants. Money conservation over cash only. Goods conservation as a flow identity: `inventory_end == inventory_start + produced - sold` per firm. |
| 11 | **Log-derived conservation that cannot fail** | The harness computes conservation from logged aggregates that were themselves computed by summing the same array the sim uses. The check is a tautology. | The harness must reconstruct balances by **replaying the logged transfer events** from the initial endowment and compare to the logged balances. Two independent paths to the same number. |

**Catch:** `INVARIANT` (money conservation, zero-sum trade, non-negative balances — every tick, halting, printing tick/agent/transaction) + `TEST` (property test: any sequence of `transfer()` calls preserves the total; largest-remainder split sums to the input) + `METRIC` (harness replays events independently) + `DESIGN` (single transfer helper; no removal; `i64` + overflow checks).

**Phase to address:** P1 — all of it. This is the foundation phase's entire reason to exist. Every later phase must route money through the P1 helper.

---

### Pitfall 11: Integer rounding discipline

**Confidence:** HIGH.

**What goes wrong:**
Integer cents removes float drift but does not remove rounding — it just makes every rounding site explicit. There are exactly four kinds of site in this model, and they need different treatments:

| Site | Example | Correct discipline |
|---|---|---|
| **Splitting one amount among N parties** | Dividends to owners; a partial wage bill across workers | **Largest remainder.** Floor each share; distribute `total - sum(floors)` leftover cents one per recipient in descending-remainder order, tie-broken by agent ID. Guarantees `sum == total`. This is the standard used in payroll and stock dividend distribution. |
| **Scaling one amount by a percentage** | Price/wage adjustment, reservation wage decay | `x * num / den` with integer `num`/`den` from config (basis points). Truncation is fine because the result is a *new state value*, not a transfer — no counterparty is being shorted. But guard the degenerate case: a 1-cent price scaled by 0.98 truncates to 0. **Enforce a minimum absolute step** (change by at least 1 cent when the multiplier is not 1) or the rule silently stops working at low values. |
| **Computing a cap or a threshold** | Household budget; working-capital buffer; ULC floor | Truncate freely. Caps and thresholds are not conserved quantities. |
| **Averaging** | Mean price for logging | Never feed a rounded average back into a decision. Compute it in the harness from the raw logged integers, in Python, where it cannot affect the trajectory. |

**Banker's rounding is the wrong tool here.** It removes systematic bias across many independent roundings, but it does **not** guarantee the parts sum to the whole — which is the property this project actually needs. Use largest-remainder for splits; use truncation for caps.

**Warning signs:** conservation invariant fires on a tick that contains a dividend or a partial payment. The remainder bug is essentially always a split bug.

**Catch:** `TEST` (property test: `split(total, weights).sum() == total` for arbitrary totals and weight vectors, including zero total, one recipient, and totals smaller than the recipient count) + `TEST` (minimum-step property on the adjustment function) + `DESIGN` (one `split` function; no ad-hoc `/n` anywhere).

**Phase to address:** P1 (the `split` and `scale` primitives), P5 (dividends use them).

---

### Pitfall 12: Determinism leaks beyond HashMap iteration

**Confidence:** HIGH on the Rust specifics (verified against rand/std documentation and the Rust 1.81 release notes).

**What goes wrong:**
The byte-identical-log requirement fails, or worse, holds on your machine and fails in CI or after a toolchain bump. Sources, in rough order of how often they bite:

**1. RNG consumption-order drift — the one the brief should worry about most.**
The seeded RNG produces a single stream. Every draw consumes one value. If a change anywhere in the code adds, removes or reorders a draw, **every subsequent draw in the entire run shifts**, and the trajectory changes completely. The consequence is nasty and specific: *you fix a bug, the economy behaves differently, and you cannot tell whether the difference came from the fix or from the reseeding.* Ten simulated years of divergence from one extra `rng.gen()`.

Variable-draw-count code is the usual culprit. Patterns that consume a *variable* number of draws:
- Rejection sampling: "draw a random firm index; if already sampled, draw again."
- The bankruptcy owner-redraw loop (PROJECT.md's recorded decision).
- Any `while` loop with an RNG call in the body.
- Conditional draws: `if firm.needs_workers { let x = rng.gen(); }`.

Mitigations, in order of leverage:
- **`DESIGN`: per-purpose sub-streams.** Derive a child RNG deterministically from `(master_seed, tick, agent_id, purpose_tag)` — e.g. hash them into a `ChaCha8Rng` seed. Then a change in the goods-market draw count cannot perturb the labour market, and adding a new stochastic feature does not invalidate every existing run. This is the single highest-leverage decision available and it must be made before there is code to retrofit.
- **Fixed-draw sampling.** To choose `k` of `n` firms, do a partial Fisher-Yates over a reusable scratch `Vec<u32>` — exactly `k` draws, always, regardless of collisions. Never rejection-sample.
- **Log an RNG draw counter per tick.** Diffing the counter series between two runs localises the tick where the streams diverged, turning a mystifying whole-trajectory difference into a pinpointed line of code. Cheap; enormously useful during debugging.
- **`DESIGN`: record the seed in the log header**, which PROJECT.md already requires — and also record the code's git SHA and the `rustc` version, because of item 3 below.

**2. `StdRng` is explicitly not reproducible across versions.**
The `rand` crate documents `StdRng` as **non-portable**: the underlying algorithm may be replaced in any release and results may be platform-dependent. A `cargo update` can therefore silently invalidate every stored run. Use `rand_chacha::ChaCha8Rng` (or ChaCha12/20), which the crate documents as maintaining reproducibility of output. Pin the version in `Cargo.lock` and commit the lockfile.

**3. `sort_unstable` tie order changes across Rust versions.**
`sort_unstable` does not preserve the relative order of equal elements — and Rust **1.81.0 replaced both sort implementations** (`slice::sort` → driftsort, `slice::sort_unstable` → ipnsort). So the permutation produced for tied keys differs between toolchain versions. In this model, ties are not rare: `sort_unstable_by_key(|f| f.price)` on 20 firms in integer cents will have ties constantly, and which tied firm is "cheapest" determines who gets the sale. A toolchain upgrade would then change the whole trajectory.
**Fix: always sort on a total key.** `sort_unstable_by_key(|f| (f.price, f.id))` has no ties, so stability is irrelevant and the result is version-independent. Make "every comparator ends in an ID tiebreak" a review rule.

**4. `HashMap` / `HashSet` iteration order.**
`RandomState` generates fresh random keys per instance, so iteration order varies within a single process and between runs. PROJECT.md already bans behaviour-affecting hash iteration; the practical trap is that "ownership is a relation, not a field" invites a `HashMap<FirmId, HouseholdId>`. Use `BTreeMap` or a sorted `Vec` for anything iterated. A `HashMap` used only for point lookups is fine — the hazard is iteration.

**5. Floating point anywhere behaviour-affecting.**
Not just money. A float statistic fed back into a decision reintroduces the whole class: `f64` addition is not associative, so changing iteration order changes the sum, and a `<` comparison on a float that differs in the last bit takes a different branch. The ABM literature documents exactly this producing **"ghost" agents** — entities visible to some parts of a program and not others — and shows that mathematically equivalent implementations are not equivalent in floating point, which broke wealth conservation in a replicated stock-market model. Keep all decision inputs integer. Floats belong in Python.

**6. Time, threads, addresses.**
- A wall-clock timestamp in every log line makes byte-identical diffing impossible. Either omit it or have the harness diff only the data columns — decide which, explicitly.
- Any `rayon` / `std::thread` use. PROJECT.md mandates single-threaded; keep `rayon` out of `Cargo.toml` entirely so it cannot creep in.
- Pointer values in `Debug` output.
- `HashMap`'s default hasher seeded from the OS (item 4 is the visible symptom).

**Warning signs:** the two-runs-same-seed diff fails; or it passes on the dev machine and fails in CI (different OS/toolchain → item 2 or 3).

**Catch:** `TEST` (run the sim twice in-process with the same seed, assert byte-identical logs — this is a cheap unit test, run it on every commit) + `TEST` (**mutation test: run with a different seed and assert the logs DIFFER** — otherwise a sim that ignores the RNG passes the reproducibility criterion trivially; see Pitfall 16) + `METRIC` (RNG draw-count series) + `DESIGN` (sub-streams, ChaCha, total-order comparators, no rayon).

**Phase to address:** P1 (all of the design decisions), every phase (the total-order comparator rule as a review checklist item), P6 (CI reproducibility job on a second OS, which the ABM literature recommends explicitly as an artefact detector).

---

### Pitfall 13: Order-of-operations accounting bugs in the tick

**Confidence:** MEDIUM-HIGH.

**What goes wrong:**
The tick order is specified — firm planning → labour → production → wages → goods market → firm accounting → bankruptcy → invariants → log — and it is correct. The bugs are in the *couplings* between steps, which the ordering alone does not pin down:

**(a) Stale vs. fresh reads of `last_sales`.** Firms plan at the start of the tick and therefore see *yesterday's* sales. That lag is intentional and is part of what generates dynamics — adaptive expectations need it. The bug is in the counter's lifecycle: if the per-firm sales accumulator is zeroed at the wrong point it either accumulates across ticks (expected demand explodes) or is zeroed before being read (expected demand decays to zero and firms stop hiring). Symptom in the logs: `expected_demand` drifting monotonically to 0 or upward without bound while actual sales are flat. **Make the lag explicit in the type system** — `last_sales` and `sales_this_tick` as separate fields, with a single `roll_over()` at a named point in the tick.

**(b) Headcount off by one tick.** Production uses the pre-hire headcount, or wages are paid on the post-fire headcount. Either creates a persistent free-lunch or free-loss. Check: `output == productivity * employees_at_production_time` as an invariant, and `wage_bill == sum over the same worker set`.

**(c) ULC floor built from a mid-update wage bill.** Covered in Pitfall 3 — the price is set before this tick's wages exist, so the floor must read last period's realised figures, explicitly.

**(d) Dividends paid before the bankruptcy check.** If a firm pays out and then goes bankrupt in the same tick, it paid out money it needed for payroll. Whether dividends come before or after the solvency check is a real modelling decision with a real effect on the bankruptcy rate. Decide it, document it, and note that "firm accounting → bankruptcy" in the specified order implies dividends happen first — so the buffer must be sized to make that safe.

**(e) Invariants checked mid-transaction.** The most common false alarm and the fastest way to erode trust in the invariant harness. If the goods market runs as two passes (debit all households, then credit all firms), the books are legitimately unbalanced between them. **Fix at the source: make `transfer()` atomic** — debit and credit in one function, so the books are never mid-transaction at a statement boundary. Then the invariant can safely run at the end of the tick *and* optionally after each market for tighter localisation.

**(f) Bankruptcy removing a firm whose transactions are already logged this tick.** The log then contains events for a firm that the end-of-tick state snapshot says does not exist. The harness's event replay (Pitfall 10 item 11) will disagree with the balances. Respawn-in-place plus incarnation IDs solves this.

**Catch:** `INVARIANT` (per-step invariant checks during development, gated by a config flag, narrowed to end-of-tick for the production run; `output == productivity * headcount`) + `TEST` (a golden 20-tick trace with hand-computed expected balances — the only reliable way to catch a systematic one-tick offset) + `DESIGN` (atomic `transfer()`; explicit `last`/`current` field separation).

**Phase to address:** P2 (tick order and the roll-over point), P5 (dividend/bankruptcy ordering), P1 (atomic transfer).

---

### Pitfall 14: Burn-in chosen by assertion rather than by evidence

**Confidence:** MEDIUM.

**What goes wrong:**
Statistics are reported over a window that still contains the transient, so the "10-year run" describes convergence rather than behaviour. The acceptance criteria then pass or fail for reasons unrelated to the economics.

**Why it happens:**
The brief fixes burn-in at 250 ticks (~8 months). That is a reasonable starting guess but it is a guess, and for this model class it may well be short. The relevant literature (Grazzini and the ABM steady-state analysis line) is explicit that steady-state analysis must be **invariant to the transient**, and that stationarity and ergodicity should be *tested* — via runs tests, KS tests or similar — not assumed.

Two things lengthen the transient specifically here:
- **Identical initial conditions.** If all firms start at the same price, wage and size, the heterogeneity that drives everything has to be generated from scratch by the stochastic adjustment, which takes many adjustment periods. With weekly planning, 250 ticks is only ~36 planning rounds per firm. Dispersed initialisation (already recommended under Pitfall 5) shortens the transient *and* improves the dynamics.
- **Slow-moving stocks.** Reservation wages and firm cash balances adjust over hundreds of ticks. A price series can look stationary at tick 250 while the wealth distribution is still spreading.

**How to avoid:**
1. Make burn-in a **config parameter**, not a constant in the harness.
2. **Justify it with a test, not a number.** The harness should: (a) split the retained window in half and compare means and variances of the headline series — they should not differ materially; (b) recompute all acceptance statistics at 2× and 4× the burn-in and assert they move by less than tolerance. If the numbers move, the burn-in is short.
3. Plot the headline series *including* the burn-in in the diagnostic charts, with the cut-off marked. Eyeballing where the transient ends is genuinely informative and costs nothing.
4. Never report a statistic computed over the full run alongside one computed over the retained window without labelling which is which. Mixing them in a summary table is how the transient leaks into conclusions.

**Warning signs:** first-half and second-half means of the retained window differ; statistics change when burn-in is doubled; a visible trend in the early part of the retained window.

**Catch:** `METRIC` (half-window mean/variance comparison; burn-in sensitivity at 2× and 4×) + `DESIGN` (burn-in in config; dispersed init).

**Phase to address:** P6, with the config parameter established in P1.

---

### Pitfall 15: Validation theatre — passing acceptance while being wrong

**Confidence:** MEDIUM-HIGH. The ABM field's own replication crisis is the evidence: models could almost never be replicated, and some published findings turned out to be software bugs.

**What goes wrong:**
Every acceptance criterion in the brief has a degenerate way to pass. Here they are, each with the counter-check that closes the hole:

| Criterion | How it passes while wrong | Counter-check |
|---|---|---|
| **Money conserved to the cent** | Nothing trades. An economy where households never shop and firms never hire conserves money perfectly. | **Liveness invariant:** transaction count and total cents transferred per tick must both be > 0. Also: employment > 0, goods produced > 0, goods consumed > 0. Add these to the every-tick invariant set — they cost nothing and they close the biggest hole in the whole acceptance suite. |
| **Money conserved** (second way) | The harness computes conservation from the same aggregate the sim computed, so the check is a tautology. | Harness **replays logged transfer events** from the initial endowment and compares to logged balances. Two independent derivations. |
| **Byte-identical logs from a seed** | The sim doesn't consume the RNG at all, or the log contains only constants. | **Mutation test:** different seed must produce a *different* log. Plus: assert the log's per-agent columns actually vary within a run. |
| **Unemployment fluctuating** | Fluctuation is bankruptcy churn (Pitfall 9c), not labour-market dynamics. | Recompute unemployment variance excluding bankruptcy-adjacent ticks; require it to remain non-trivial. Separately bound bankruptcies/year. |
| **Unemployment in a band** | It's in the band because it is *pinned* at a value inside the band. | Require a **variance floor** and a maximum run-length of identical consecutive values, not just a mean in a range. |
| **Price level stable** | Stable because every firm is welded to the ULC floor and the price rule is inert. | Require `fraction_at_floor` to be low and price CV to be positive (Pitfall 3). |
| **Output autocorrelation positive** | The series is a slow trend or a random walk — non-stationary, so ACF is high at every lag by construction. | Test stationarity first (or difference the series); require the ACF to **decay**, not merely to be positive. Also require no spike at the planning cadence (Pitfall 6). |
| **Firm size distribution dispersed** | Dispersion comes from a handful of just-respawned tiny firms, not from genuine size heterogeneity among survivors. | Compute the distribution over firms with age above a threshold; check dispersion is stable over the window, not driven by entry. |
| **Invariants pass** | They're `debug_assert!` and were compiled out of the release build that produced the run. | Invariants must be ordinary runtime checks with a hard halt. Add a **negative test**: a deliberately-broken build must halt on the seeded violation. An invariant that has never been observed to fire has never been shown to work. |

**How to avoid:**
Write the counter-checks in the same phase as the criteria. The general principle worth stating explicitly in the roadmap: **for each acceptance criterion, write down the degenerate way to pass it before writing the check, and add the counter-check at the same time.**

**Catch:** `INVARIANT` (liveness) + `TEST` (mutation test on seed; negative test on the invariant harness) + `METRIC` (every counter-check in the table).

**Phase to address:** P6 primarily, but the liveness invariants and the negative test belong in P1.

---

### Pitfall 16: Diagnostics that cannot diagnose

**Confidence:** MEDIUM.

**What goes wrong:**
The invariant fires at tick 1,847 and prints `assertion failed: money conserved`. You now have to reproduce a 1,847-tick run under a debugger to find out what happened, and the run takes seconds so you will — but you will do it fifty times over the project and it will be the dominant cost of the whole build.

PROJECT.md already requires printing tick, agent and transaction on violation. Three additions make that actually sufficient:
- **Print the delta and its sign.** "Expected M = 20,000,000; actual 19,999,997; short by 3 cents" immediately tells you it is a 3-way split remainder (Pitfall 11) rather than a wrapped subtraction.
- **Keep a ring buffer of the last N transfers** and dump it on violation. The violating transaction is rarely the guilty one; the one before it usually is.
- **Support `--halt-at-tick` and `--dump-state-at-tick`** so you can snapshot just before the failure without instrumenting the code each time.

The related trap: **provenance recorded but not queryable.** The brief requires decision provenance from the first tick — record it in a schema the Python harness can actually join on (a flat event table keyed by tick + agent + decision type), not as free-text strings. Retroactively restructuring 800k rows of provenance is possible but tedious; getting the schema right in P1 costs nothing.

**Catch:** `DESIGN` (violation report contents; ring buffer; CLI flags; flat provenance schema).

**Phase to address:** P1.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|---|---|---|---|
| `f64` for the household budget fraction "just this once" | One less integer-arithmetic helper | Reintroduces the entire float-drift class the project was built to avoid; may not break conservation but will break byte-identical reproducibility across platforms | **Never** |
| `Vec::swap_remove` for bankrupt firms | Simple, keeps the `Vec` dense | Invalidates the last element's ID; creates dangling-ID money leaks and splices per-firm log series | **Never** — respawn in place |
| `StdRng` instead of `rand_chacha` | One fewer dependency | A `cargo update` silently invalidates every stored run and every reproducibility test | **Never** |
| One global RNG stream instead of per-purpose sub-streams | Simpler to write | Every bug fix that changes draw count changes the whole trajectory; you can never attribute an economic change to a code change | Acceptable only if the RNG draw-count series is logged so divergence can be localised; sub-streams are strongly preferred |
| `debug_assert!` for invariants | Zero cost in release | The production 10-year run has no invariants at all — the project's core guarantee evaporates in exactly the build that matters | **Never** |
| Deterministic full-step price adjustment instead of probabilistic partial | Simpler, fewer parameters | Converts damped adjustment into a ratchet (Pitfall 2) and kills dispersion (Pitfall 5) | **Never** — this is a mechanism, not an implementation detail |
| Firm demand = aggregate ÷ N instead of realised matches | Avoids threading match results back to firms | Destroys all firm heterogeneity; model silently becomes a representative-agent difference equation | **Never** |
| Sample size as a fraction of N | "Scales naturally" | Friction vanishes as N grows; the later scaling exercise changes the economics rather than the runtime | **Never** |
| Fixed agent iteration order (no shuffle) | Fewer RNG draws, simpler | Systematic ID-correlated wealth artefact accumulating over 3,650 ticks | Acceptable only in a throwaway spike, never in a run that produces reported numbers |
| Fixed nominal working-capital buffer | Easy to configure | Becomes a permanent money sink as the price level drifts; reintroduces the deflationary stall | Never in the final rule; fine as a first-day placeholder if flagged |
| Wall-clock timestamp in every log line | Nice for humans | Makes byte-identical diffing impossible | Acceptable if the log has a separate header line for run metadata and the harness diffs only data columns — decide explicitly |

---

## Reference-Model Fidelity Gotchas

Replaces the template's "integration gotchas" — this project's external dependencies are published models, and the failure mode is mis-porting them.

| Reference | Common mistake | Correct approach |
|---|---|---|
| Lengnick (2013) price/wage adjustment | Dropping the adjustment *probability* and the *random* step size, keeping only the trigger condition | Keep both: adjust with probability θ by a fresh `Uniform(0, ū)` percentage. The stochasticity is the damping |
| Lengnick reservation wage | Implementing decay without a positive floor | Floor it in config; decay to zero pins unemployment at 0% and collapses wages |
| Lengnick / BAM search | Sampling a fraction of firms | Small absolute constant, independent of N |
| BAM bankruptcy/entry | Entrant seeded at zero state | Seed from surviving-population medians; a zero-expectation entrant is dead on arrival |
| BAM credit channel | Importing bankruptcy dynamics calibrated for a model *with* banks | This build has no credit, so BAM's bankruptcy cascade mechanism does not apply; expect a lower and more boring bankruptcy rate and do not tune toward BAM's |
| Caiani et al. SFC | Treating "stock-flow consistent" as a style rather than a check | Adopt the concrete device: build the transaction-flow matrix; every row and column sums to zero; one equation is redundant, and computing it and asserting zero is the canonical leak detector. This is exactly what the money-conservation invariant should be, generalised |
| Gualdi/Bouchaud phase diagram | Assuming a single "correct" parameter set exists | Expect a phase diagram. Sweep the key parameters in P6 and confirm the operating point is in the interior of a good region, not near a boundary |
| Any published model | Assuming the paper's equations fully determine the implementation | The floating-point replication literature is explicit that mathematically equivalent implementations are not computationally equivalent. Where the paper is ambiguous, record the choice as a Key Decision rather than silently picking one |

---

## Performance Traps

At 220 agents and 3,650 ticks this build is not performance-constrained, and the brief is right to say so. Only three things can actually make it slow enough to hurt the debug loop:

| Trap | Symptoms | Prevention | When it breaks |
|---|---|---|---|
| Unbuffered per-tick log flush | Run takes minutes instead of seconds; wall time dominated by syscalls | `BufWriter` with an explicit flush at end of run (and on invariant violation, so the failing tick is on disk) | Immediately, at any N |
| Per-tick allocation in the hot path | Steady GC-free but allocator-bound; noticeable but not fatal here | Reuse scratch `Vec`s for sampling and shuffling across ticks — also required for fixed RNG draw counts | ~10⁴ agents |
| O(N_households × N_firms) market loops | Fine at 200×20 = 4,000; quadratic thereafter | Bounded sampling already makes this O(N × k). Keep it that way — do not "optimise" by pre-sorting all firms globally, which reintroduces perfect information | ~10⁴ agents |
| Full per-agent state written every tick | 3,650 × 220 ≈ 800k rows — perfectly fine; becomes an issue only if each row is wide | Per-tick aggregates + per-event records (as the brief specifies) rather than a full state dump. Keep per-agent snapshots to a periodic cadence if width grows | ~10⁵ agents or ~10² columns |

**Not applicable sections.** The template's *Security Mistakes* and *UX Pitfalls* sections have no meaningful content for a single-user, offline, deterministic simulation binary with no network surface, no untrusted input, and no user interface. The nearest analogues are covered above: the config file is the only external input (validate it — reject out-of-range parameters at load rather than producing a nonsense run), and Pitfall 16 covers diagnostic ergonomics.

---

## "Looks Done But Isn't" Checklist

- [ ] **Money conservation:** passes — but has it ever been observed to *fail*? Seed a deliberate 1-cent leak and confirm the invariant halts. An untested assertion is decoration.
- [ ] **Money conservation:** passes — but is anything trading? Check the liveness invariants exist and are non-trivial.
- [ ] **Reproducibility:** same seed → identical log. Also confirmed that a *different* seed → *different* log?
- [ ] **Reproducibility:** confirmed on a second OS/toolchain, not just the dev machine? (Catches `StdRng`, `sort_unstable` tie order, and any leaked float.)
- [ ] **Dividends:** implemented — but does the payout rule actually *drain* excess, and is the buffer flow-denominated? Check the `firm_share` slope, not just that a dividend event exists in the log.
- [ ] **Price floor:** implemented — but how often does it bind? If most firms sit on it, the price rule is inert.
- [ ] **Price/wage adjustment:** implemented — but is it probabilistic and randomly-sized, or deterministic full-step?
- [ ] **Search sampling:** bounded — but does it fall through to the next-cheapest on stockout, and does unmet demand get recorded?
- [ ] **Firm demand:** is `last_sales` derived from that firm's actual purchase events, or from an aggregate?
- [ ] **Staggering:** firms plan on different days — but is the offset drawn from the RNG, or is it `id % 7` (cohort lockstep)?
- [ ] **Agent ordering:** is each market's agent order shuffled per tick from the seeded RNG, or is it `0..n` forever?
- [ ] **Bankruptcy:** does the residual reach the owner and the endowment come from a household, both via `transfer()`, both logged as events?
- [ ] **Bankruptcy:** does the log distinguish incarnations of a reused firm slot?
- [ ] **Dispersion:** are firm price / size / inventory and household cash CVs logged, and are they non-zero at the end of the run?
- [ ] **Burn-in:** is 250 justified by a stationarity or sensitivity check, or is it just the number in the brief?
- [ ] **Config:** does every parameter in the brief's table actually come from the file, verified by changing one and seeing the run change?
- [ ] **Overflow:** is `overflow-checks = true` set in `[profile.release]`? (Cargo defaults it off.)
- [ ] **Comparators:** does every sort key end in an ID tiebreak?

---

## Recovery Strategies

| Pitfall | Recovery cost | Recovery steps |
|---|---|---|
| Money leak found late | **LOW** — if `transfer()` is the single mutation point | Bisect with the ring-buffer dump and the tick-level invariant; the leak is in one function. If money movement is scattered across the codebase instead, this becomes HIGH and is effectively a refactor of everything |
| Deflationary stall | **LOW-MEDIUM** | It is a parameter/rule problem, not a structural one. Fix the buffer denomination and the drain rule; re-run. Costs one run cycle (seconds) plus reasoning time |
| Unemployment pinned | **MEDIUM** | Sweep the coupled reservation-wage/wage-step group; if no setting works, the labour-market matching itself is wrong (sample size, offer acceptance rule) |
| Dead steady state | **MEDIUM-HIGH** | Diagnose from the dispersion CVs. If the cause is aggregate-derived firm demand, it is a structural fix to the goods market and its logging |
| Determinism broken by RNG drift | **HIGH if discovered late** | Without sub-streams or a draw-count series there is no way to localise it; you re-derive the whole trajectory. With them, it is a bisect. This asymmetry is why sub-streams are a P1 `DESIGN` item |
| `swap_remove` ID corruption | **HIGH** | Every ID-carrying structure and every stored log series is suspect. Prevention (respawn in place) is the only sane strategy |
| Float leaked into a decision path | **MEDIUM** | `grep` for `f64`/`f32` in the sim crate; a CI lint that fails the build on any float type outside the logging layer makes this a non-issue permanently |
| Burn-in too short | **LOW** | Recompute; it is a harness parameter. Only costly if published numbers were already circulated |
| Synchronisation artefact found in results | **MEDIUM** | The fix (stagger, shuffle) is small, but every previously-computed statistic is invalid and must be regenerated |

---

## Pitfall-to-Phase Mapping

| # | Pitfall | Prevention phase | Catch | Verification |
|---|---|---|---|---|
| 1 | Deflationary stall | P5 rule, P1 logging | METRIC + INVARIANT + DESIGN | `firm_share` slope ≈ 0 over retained window; `firm_share < 0.5` every tick |
| 2 | Price spiral up | P2/P3 | METRIC + TEST | Price-level trend ≈ 0; up/down adjustment counts balanced; symmetry unit test |
| 3 | Price collapse / floor wrong | P2 | INVARIANT + TEST + METRIC | `price >= 1` every tick; floor property test incl. zero output; `fraction_at_floor` low |
| 4 | Unemployment pinning | P3, swept in P6 | METRIC + DESIGN | Unemployment variance floor; no 30-tick constant run; vacancy fill rate in band; parameter sweep shows interior operating point |
| 5 | Dead steady state | P2 mechanisms, P1 logging | METRIC + TEST + DESIGN | Four dispersion CVs above floor; output ACF positive and decaying; `last_sales` provenance test |
| 6 | Synchronisation artifacts | P1 shuffle, P2 stagger, P3/P4 apply | METRIC + TEST + DESIGN | No ACF spike at planning cadence; wealth-vs-ID rank correlation ≈ 0; permutation validity test |
| 7 | Search friction mis-sized | P3/P4, swept in P6 | METRIC + DESIGN | Price CV in band; unmet-demand fraction small but non-zero; sample size is a constant |
| 8 | Firm size degeneracy | P2 hire cap + init, P4 fall-through | METRIC + TEST + DESIGN | HHI / max share / size Gini in bands; fall-through unit test |
| 9 | Bankruptcy/respawn artifacts | P5, schema in P1 | METRIC + DESIGN | Bankruptcy rate/yr bounded; net respawn cash flow ≈ 0; bankruptcy-excluded unemployment variance non-trivial |
| 10 | Money conservation leaks | **P1** | INVARIANT + TEST + METRIC + DESIGN | Every-tick conservation halt; `transfer()` property test; harness event-replay reconciliation |
| 11 | Integer rounding | P1 primitives, P5 use | TEST + DESIGN | `split()` sums to total property test; minimum-step property test |
| 12 | Determinism leaks | **P1**, enforced every phase | TEST + METRIC + DESIGN | Same-seed byte-identical test; different-seed-differs mutation test; second-OS CI job; RNG draw-count series |
| 13 | Order-of-operations | P2 order, P5 dividend/bankruptcy order, P1 atomic transfer | INVARIANT + TEST + DESIGN | Golden 20-tick hand-computed trace; `output == productivity × headcount` |
| 14 | Burn-in | P6, param in P1 | METRIC + DESIGN | Half-window mean/variance comparison; statistics stable at 2× and 4× burn-in |
| 15 | Validation theatre | P6, liveness in P1 | INVARIANT + TEST + METRIC | Liveness invariants; negative test on the invariant harness; every counter-check in the table |
| 16 | Undiagnosable diagnostics | P1 | DESIGN | Violation report includes delta + last-N transfers; provenance is a joinable flat table |

**Phase-ordering implication.** Pitfalls 10, 11, 12, 15 (liveness) and 16 all land in P1, and all of them are `DESIGN`-tagged — they cannot be retrofitted cheaply. P1 should be scoped generously and gated hard: no economic mechanism enters the codebase until the money type, the atomic `transfer()`, the split primitive, the RNG sub-stream scheme, the shuffle utility, the invariant harness (with its negative test) and the log schema are all in place and tested. Every economic pitfall in this document assumes those exist; without them, every economic bug is also an accounting mystery.

**Research-flag recommendation for the roadmap.** P3 (labour market) and P5 (accounting/dividends/bankruptcy) are the two phases most likely to need deeper, phase-specific research — P3 because the reservation-wage/wage-step parameter coupling is where the phase boundary lives and the published values were not obtainable in this pass, and P5 because the dividend and respawn rules are where the brief is least specified and where the most damaging economic failure mode originates. P2 and P4 are comparatively well-determined by the brief.

---

## Gaps and Confidence Caveats

Stated plainly, because the downstream roadmap should know where this document is thin:

- **The Lengnick parameter table was not obtained.** The primary PDF, the sim4edu mirror, arxiv, econstor, comses and jasss.org were all blocked by the network egress proxy in this environment, and direct `curl` is blocked too. Everything about Lengnick's specific numeric parameter values in this document is therefore *structural inference from the model class*, not a quotation. **Recommendation: obtaining the actual parameter table (paper, or the `newwayland/baseline-economy` Mesa implementation on GitHub, or `YudiWang/Baseline-Economy`) should be an explicit early task in P1 or P3.** It materially de-risks Pitfall 4, which is the pitfall with the widest parameter-sensitivity.
- **The precise search sample sizes** used in published Lengnick/BAM calibrations are stated here as "small single digits" on the strength of secondary description. Verify against a reference implementation before fixing config defaults.
- **Timescales for the deflationary stall** ("within 1-3 simulated years") are reasoned from the arithmetic of a fixed money stock and a proportional spending rule, not measured. The *mechanism* is certain; the *speed* depends on the profit rate and should be measured in the first runs rather than assumed.
- **The Bouchaud phase-diagram findings** come from Mark-0, which has a credit channel this build lacks. The hiring/firing-asymmetry result is stated in that literature as generic and robust across model modifications, so it is applied here with reasonable confidence; the credit-ceiling axis (Θ) is explicitly not applied.
- All web-derived claims carry LOW-to-MEDIUM confidence per the classification seam (`websearch` provider, cross-checked → MEDIUM). Claims about Rust semantics (HashMap `RandomState`, `StdRng` non-portability, the 1.81 sort replacement, Cargo's release-profile overflow-check default) are the most solid in the document and were corroborated across independent sources.

---

## Sources

**Economics — model class**
- Lengnick, M. (2013), *Agent-based macroeconomics: A baseline model*, Journal of Economic Behavior & Organization 86, 102-120. [IDEAS/RePEc](https://ideas.repec.org/a/eee/jeborg/v86y2013icp102-120.html) — the closest reference model. Full text inaccessible in this environment. **Confidence: LOW** (structural description only).
- Delli Gatti, Desiderio, Gaffeo, Cirillo & Gallegati (2011), *Macroeconomics from the Bottom-up* (the BAM model). [Springer](https://www.springer.com/gp/book/9788847019706), [BAM codebase on CoMSES](https://www.comses.net/codebases/9dacc220-8d7f-4038-b618-92bb9b1333f0/releases/1.1.0/). **Confidence: LOW.**
- Caiani, Godin, Caverzasi, Gallegati, Kinsella & Stiglitz (2016), *Agent based-stock flow consistent macroeconomics: Towards a benchmark model*, JEDC 69, 375-408. [PDF mirror](https://faculty.sites.iastate.edu/tesfatsi/archive/tesfatsi/ABMSFCMacroModelBenchmark.CainiEtAl2016.pdf) — source of the quadruple-bookkeeping / redundant-equation discipline behind the invariants. **Confidence: MEDIUM.**
- Gualdi, Tarzia, Zamponi & Bouchaud (2015), *Tipping points in macroeconomic agent-based models*, JEDC. [arXiv:1307.5319](https://arxiv.org/abs/1307.5319) — the phase-transition result; hiring/firing asymmetry as the generic driver of the unemployment corner. **Confidence: MEDIUM.** Follow-up: [Exploration of the Parameter Space in Macroeconomic ABMs](https://arxiv.org/pdf/2111.08654), [Navigating through Economic Complexity: Phase Diagrams & Parameter Sloppiness](https://arxiv.org/html/2412.11259).

**ABM methodology — errors, artefacts, replication**
- Galán & Izquierdo et al. (2009), *Errors and Artefacts in Agent-Based Modelling*, JASSS 12(1)1. [JASSS](https://jasss.soc.surrey.ac.uk/12/1/1.html) — the canonical error/artefact taxonomy; recommends re-running identical code on different machines, OSes and PRNGs as a detector. **Confidence: MEDIUM.**
- Polhill, Izquierdo & Gotts (2005), *The Ghost in the Model (and Other Effects of Floating Point Arithmetic)*, JASSS 8(1)5. [JASSS](https://jasss.soc.surrey.ac.uk/8/1/5.html) — float comparison in branches producing "ghost" agents; mathematically equivalent implementations not equivalent in floating point; wealth conservation broken in a replicated stock-market model. **Confidence: MEDIUM.** Companion: [Is Your Model Susceptible to Floating-Point Errors?](https://jasss.soc.surrey.ac.uk/9/4/4.html).
- Rand & Wilensky, *Verification, Validation, and Replication Methods for Agent-Based Modeling and Simulation: Lessons Learned the Hard Way!* [Springer](https://link.springer.com/chapter/10.1007/978-3-319-15096-3_10) — the ABM replication crisis; findings that turned out to be software bugs. **Confidence: MEDIUM.**
- *Activation Regimes in Opinion Dynamics*, JASSS 18(3)8. [JASSS](https://jasss.soc.surrey.ac.uk/18/3/8.html) — synchronous/asynchronous/uniform/random/Poisson taxonomy; Huberman & Glance vs Nowak & May; firm growth/size dependence reversing with activation regime. **Confidence: MEDIUM.**
- *Scheduler Dependencies in Agent-Based Models: A Case-Study Using a Contagion Model*. [Springer](https://link.springer.com/chapter/10.1007/978-3-030-96188-6_5). **Confidence: LOW.**
- *Timing Matters: Lessons From The CA Literature On Updating*. [arXiv:1008.0941](https://arxiv.org/pdf/1008.0941). **Confidence: LOW.**
- Grazzini (2012) on stationarity/ergodicity testing in ABMs, via *Automated and distributed statistical analysis of economic agent-based models*. [arXiv:2102.05405](https://arxiv.org/pdf/2102.05405) — Wald-Wolfowitz runs test and KS test for non-stationarity; steady-state analysis must be invariant to the transient. **Confidence: MEDIUM.**

**Rust / implementation**
- [`rand::rngs::StdRng` docs](https://docs.rs/rand/latest/rand/rngs/struct.StdRng.html) and [The Rust Rand Book — Reproducibility](https://rust-random.github.io/book/crate-reprod.html) — `StdRng` explicitly non-portable; ChaCha8/12/20Rng documented as maintaining output reproducibility. **Confidence: MEDIUM-HIGH.**
- [Rust 1.81.0 release notes](https://blog.rust-lang.org/2024/09/05/Rust-1.81.0/) and [PR #124032 "Replace sort implementations"](https://github.com/rust-lang/rust/pull/124032) — `slice::sort` → driftsort, `slice::sort_unstable` → ipnsort; tie-order permutation is version-dependent. **Confidence: MEDIUM-HIGH.**
- [The stable HashMap trap](https://morestina.net/1843/the-stable-hashmap-trap) and [rust-lang/rust#36481](https://github.com/rust-lang/rust/issues/36481) — `RandomState` generates fresh random keys per instance; iteration order nondeterministic; sort keys or use `BTreeMap`. **Confidence: MEDIUM-HIGH.**
- Largest-remainder / penny-allocation discipline, cross-checked against [Banker's Rounding (Lippert)](https://ericlippert.com/2003/09/26/bankers-rounding/) and [`rusty-money` docs](https://docs.rs/rusty-money/latest/rusty_money/) (whose default truncation demonstrates the failure: $100.00 split 3 ways → $99.99). **Confidence: MEDIUM.**

**Firm size distributions**
- Delli Gatti et al., *A new approach to business fluctuations: heterogeneous interacting agents, scaling laws and financial fragility*. [arXiv:cond-mat/0312096](https://arxiv.org/pdf/cond-mat/0312096). **Confidence: LOW-MEDIUM.**
- *Validating an agent-based model of Zipf's Law: A discrete Markov-chain approach*, JEDC. [ScienceDirect](https://www.sciencedirect.com/science/article/pii/S0165188914000360). **Confidence: LOW.**

---
*Pitfalls research for: minimal closed-economy agent-based macroeconomic simulation in Rust*
*Researched: 2026-08-30*
