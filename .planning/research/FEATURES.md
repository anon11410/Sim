# Feature Research

**Domain:** Minimal closed-economy agent-based macroeconomics (Lengnick / BAM class)
**Researched:** 2026-08-30
**Confidence:** MEDIUM overall — see "Evidence Grades" below. Individual published parameters range from HIGH (recovered from the model authors' own source repositories) to GAP (genuinely not recoverable in this session).

---

## 0. Evidence Grades and Provenance

The network egress policy in this session blocked every academic host (ScienceDirect, EconStor, arXiv, RePEc, SSRN, iastate.edu, uv.es, sim4edu, macau.uni-kiel.de). **The primary paper PDFs could not be opened.** What *was* reachable was `github.com` / `raw.githubusercontent.com`, so the specifications below were recovered from reference implementations, two of which are written by the model's own authors.

The GSD confidence seam (`gsd_run query classify-confidence`) returns `LOW` for `webfetch` and `MEDIUM` for verified `websearch`. Those provider-level tiers are recorded in the research cache. Because provider tier alone understates the difference between "a blog summarised the paper" and "I read the authors' own source code", each claim below also carries an **evidence grade**:

| Grade | Meaning | Sources at this grade |
|-------|---------|----------------------|
| **A — author's own code** | Repository authored/copyrighted by the paper's authors. Equivalent to an appendix. | `S120/jmab`, `S120/benchmark` (© Alessandro Caiani and Antoine Godin, the paper's 1st and 2nd authors) |
| **B — annotated replication** | Third-party replication that cites the paper's own table/equation numbers inline and documents its deviations. | `newwayland/baseline-economy` (Lengnick, Mesa); `alexplatasl/BAMmodel` (BAM, NetLogo + full ODD protocol) |
| **C — inferred** | Derived by me from an A/B source by arithmetic or logic, not stated anywhere. | Marked inline as *(derived)* |
| **GAP** | Not recovered. Stated as a gap, **not** filled with invention. | Marked `GAP` |

**Every value in Section 1 is grade A or B unless labelled otherwise.** Nothing in this document is invented economics. Where I recommend a value that is not in any paper, it is labelled **PROJECT CHOICE** and the reasoning is given.

**Verification note.** The Lengnick numbers are grade B, not A. The replication repo annotates them `# Calibration values (Table 1)` and cross-references `Eq(5)`, `Eq(6)`, `Eq(7)`, `Eq(10)`, `Eq(11)`, `Eq(12)` — i.e. the author of the replication was reading the table directly. It also ships a `notes/issues.md` cataloguing exactly where the paper is silent, which is itself strong evidence of fidelity. **Before final calibration, someone with journal access should spot-check Lengnick Table 1 against Section 1.3 below.** That is the single highest-value verification action remaining.

---

## 1. PUBLISHED SPECIFICATIONS — the priority deliverable

This is the section that fills the project's parameter table. It is organised by the nine questions asked.

### 1.1 Model scale and cadence (context for every rate below)

| Model | Households | Firms | Ratio | Tick | Decision cadence |
|-------|-----------|-------|-------|------|------------------|
| **Lengnick** | 1000 | 100 | 10:1 | 1 day | **1 month = 21 days** |
| **BAM** | 500 | 100 (+10 banks) | 5:1 | 1 period | every period |
| **Caiani et al.** | 8000 | 100 C-firms, 20 K-firms (+10 banks) | ~67:1 | 1 period (quarter) | every period |
| **This project** | 200 | 20 | 10:1 | 1 day | weekly (brief) |

Grade A (Caiani `modelBenchmark_light.xml` `size` values), grade B (Lengnick `model.py` defaults `num_households=1000, num_firms=100, month_length=21`; BAM `README.md` initialisation table).

> **⚠ LOAD-BEARING FINDING — rate rescaling.** The project matches Lengnick's 10:1 agent ratio (good), but **every Lengnick adjustment rate is per 21-day month, and the brief plans a weekly (7-day) cadence.** Applying Lengnick's monthly numbers at a weekly cadence runs the whole adjustment side of the economy **3× too fast** and is a plausible cause of price spirals. Either adopt a 21-day cadence, or rescale. Conversions *(derived, grade C — arithmetic on grade-B values)*:
>
> | Lengnick, per month (21d) | Equivalent per week (7d) | Equivalent per day |
> |---|---|---|
> | reservation wage ×0.9 | ×0.9^(1/3) = **×0.9655** (−3.45%) | ×0.9^(1/21) = **×0.99500** (−0.50%) |
> | price step U(0, 0.020) | U(0, **0.0067**) | U(0, 0.00095) |
> | wage step U(0, 0.019) | U(0, **0.0063**) | U(0, 0.00090) |
> | γ = 24 months | **72 weeks** | 504 days |
> | χ = 0.1 × monthly payroll | **0.30 × weekly payroll** | 2.1 days of payroll |
>
> Note in particular that the brief's stated **1%/day reservation-wage decay compounds to 1 − 0.99²¹ = 19% per month, ~2× Lengnick's published 10%.** Recommend 0.5%/day (or apply ×0.9 once per month). This is the single most likely cause of an unemployment/wage spiral, and the brief itself names reservation wage decay as a prime suspect.

---

### 1.2 Question 1 — Household consumption rule ✅ RECOVERED

**Lengnick, Eq (11) + Table 1** — grade B (`BaselineEconomy/household.py`, function annotated `Table 2 - Consumption Function`, `# ... in the range 0 < alpha < 1  Eq(11)`):

```
c_h = ( m_h / P̄_h ) ^ α           with α = 0.9
```

- `m_h` = household liquidity (money, all of it — cash on hand, not income)
- `P̄_h` = **arithmetic mean price across the household's own n=7 preferred suppliers** — not the economy-wide price index. This matters: it is local information.
- `c_h` is **monthly planned consumption in units of goods**. Daily demand = `floor(c_h / 21)`.
- Purchases are additionally capped at what is affordable, `floor(m_h / p_firm)`, evaluated per firm at purchase time.

**Answer to "do not let households spend to zero": the concave rule does this by itself. No extra floor is needed.**

*(derived, grade C):* nominal spending is `P̄ · (m/P̄)^0.9 = m^0.9 · P̄^0.1`, so
```
savings = m − m^0.9 · P̄^0.1 = m · (1 − (P̄/m)^0.1) > 0   ⟺   m > P̄
```
i.e. a household retains a strictly positive balance whenever its liquidity exceeds the price of a single good. The average propensity to spend is `m^(α−1) = m^(−0.1)`, decreasing in wealth — exactly the "higher fraction at low wealth, lower at high wealth" shape the brief describes. **The brief's shape and Lengnick's rule are the same rule.** Adopt `α = 0.9` and drop any separate "minimum retained cash" clause; adding one on top would be inventing economics.

`GAP:` Lengnick Eq (12) is a further bound on consumption that I could not recover verbatim. The replication's `notes/issues.md` issue #1 reports it "seems redundant — it can only apply when the planned consumption is less than one unit of goods", and the replication omits it with no observable effect. **Treat Eq (12) as safe to omit; do not invent a substitute.**

**Alternatives, for contrast (do not mix):**

| Model | Rule | Params | Grade |
|---|---|---|---|
| **BAM** submodel 33 | `c_j = 1 / (1 + [tanh(SA_j / SA_avg)]^β)`, spend `c_j × (income + savings)` | β = 0.87 | B |
| **Caiani** `ConsumptionFixedPropensitiesOOIWWithPersistency` | `c = c₁·E[real income] + c₂·real wealth` | c₁ = 0.385808, c₂ = 0.25, persistency = 0 | A |

**Recommendation: Lengnick's `(m/P̄)^0.9`.** It is the only one of the three that needs a single state variable the project already has (liquidity), needs no income expectation, and is closest to the brief's stated shape. BAM's `tanh` rule needs a cross-sectional average savings figure (global information, mild anti-feature). Caiani's is a Modigliani two-propensity rule, not concave, and depends on an income expectation the project has not budgeted for.

---

### 1.3 Questions 2 & 3 — Reservation wage dynamics and wage-contract semantics ✅ RECOVERED (and it contradicts a project decision)

#### Lengnick (grade B — `household.py::adjust_reservation_wage`, run at month end)

```python
if unemployed:  reservation_wage *= 0.9          # -10% per month, "Value fixed at 10% reduction in paper"
else:           reservation_wage = max(reservation_wage, employer.wage_rate)
```

**The published rise rate while employed is not a rate at all — it is a ratchet.** The reservation wage is set to the wage you are actually receiving whenever that exceeds it. Trigger: employment, evaluated monthly. It never falls while employed and never rises above what someone has actually paid you. This is a materially different mechanism from a fixed percentage growth rate, and it is self-limiting — it cannot outrun the wage distribution, so it cannot drive a spiral on its own.

The reservation wage is used in exactly two places:
- `is_paid_too_little()`: `employer.wage_rate < reservation_wage` → triggers job search
- `is_acceptable_job_offer()`: accept if `new_employer.wage_rate > reservation_wage` **or** `new_employer.wage_rate > current_employer.wage_rate`

#### Caiani et al. (grade A — `jmab.strategies.AdaptiveWageStrategy` + `Households.java` + `modelBenchmark_light.xml`)

An independent, fully-specified alternative:

```
microRef  = share of last 4 periods spent unemployed   (0 if currently employed)
macroRef  = last period's aggregate unemployment rate  (0 if currently unemployed)

if   microRef > 0.49 :  w ← w · (1 − ζ)          # cut
elif macroRef ≤ 0.08 :  w ← w · (1 + ζ)          # raise
w ← max(w, wageLowerBound)

ζ ~ FoldedNormal(mean = 0.0, stdev = 0.0094)     # i.e. |N(0, 0.0094)|
```

Parameters: `microThreshold = 0.49`, `macroThreshold = 0.08`, `microAdaptiveParameter = 1`, `macroAdaptiveParameter = 1`, `employmentWageLag = 4`, `wageLowerBound = 0` in the benchmark (the dole in the `HouseholdsWithDole` variant).

*(derived, grade C)* `E|N(0, 0.0094)| = 0.0094·√(2/π) ≈ 0.0075`, so the expected step is **±0.75% per period, symmetric in magnitude, asymmetric in trigger**: you cut only after being unemployed for more than half of the last four periods; you raise only while employed *and* while aggregate unemployment is below 8%.

#### BAM (grade B — `alexplatasl/BAMmodel`)

**BAM has no reservation wage at all.** Its ODD protocol and reference NetLogo contain no such variable. Unemployed workers simply `move-to max-one-of my-potential-firms [wage-offered-Wb]` — they take the best of the M offers they can see, unconditionally. The downward floor on wages is instead a **statutory minimum wage** `ŵ_t`, recomputed every 4 periods as the minimum across incumbent firms. Do not cite BAM as a source for reservation wage dynamics; it is not one.

#### Wage-contract semantics — **⚠ PROJECT DECISION CONFLICT**

> PROJECT.md Key Decisions states: *"Wages contracted at hire, not floating to the firm's current offer | **Matches Lengnick**"*.
>
> **This attribution is incorrect.** In Lengnick, the wage floats. Grade-B evidence, three independent confirmations in the replication:
> 1. `BaselineEconomyHousehold` has **no wage field**. Every read is `self.employer.wage_rate`.
> 2. `firm.pay_wages()` pays `self.wage_rate` — the firm's *current* rate — to every worker in `self.workers`.
> 3. `notes/issues.md` states the assumption explicitly: *"Wage rises and cuts apply to all workers in a firm immediately."*
>
> There is no contract length in Lengnick. There is only a one-month firing notice (`worker_on_notice`), and workers can quit and move instantaneously.

| Model | Wage semantics | Contract length | On firm wage change |
|---|---|---|---|
| **Lengnick** | Floats — worker is always paid the firm's current `wage_rate` | none (1-month firing notice only) | All incumbents' pay changes immediately, up **and down** |
| **BAM** | **Fixed at hire** (`set my-wage wage-employees` at the moment of hiring) | `contract = 8 + Poisson(10)` periods in the reference NetLogo; **the book's stated θ I could not verify — `GAP`** | Incumbents unaffected; only new hires get the new offer |
| **Caiani** | Firm posts, worker has an asked wage; matching resolves | `turnoverLabor = 0.05` (5% of matches churn per period) | n/a |

**Recommendation: keep the project's contracted-at-hire decision, but re-attribute it to BAM, not Lengnick.** Reasons:
1. The project already has separate `Household.wage` and `Firm.offered_wage` fields; contracting is the only semantics consistent with that data layout.
2. A floating wage **combined with** a reservation-wage ratchet (`w_r ← max(w_r, employer.wage)`) creates a same-tick feedback loop: a firm raising its offer to attract one hire instantly raises the reservation wage of its entire existing workforce, which raises the wage they will demand elsewhere. Lengnick survives this because γ = 24 makes wage cuts almost never happen; a project with a shorter γ would not.
3. Contracting also fixes Lengnick's ugliest edge case — the "interns" bug, where an insolvent firm sets `wage_rate = 0` and its whole workforce works for free (see §1.9).

**Recommended contract length: PROJECT CHOICE.** BAM's `8 + Poisson(10)` averages 18 *periods*; there is no principled day-conversion. Recommend a fixed contract that ends only on quit, fire, or bankruptcy (i.e. indefinite), because the project has no unemployment benefit and no credit market, so forced periodic unemployment would drain household cash with nothing to cushion it. Document as a deviation.

---

### 1.4 Question 4 — Price and wage adjustment rules ✅ RECOVERED

#### Lengnick price rule (grade B — `firm.py::set_goods_price`, params annotated `Table 1`)

Evaluated at month start, **and only with probability θ = 0.75** (`if self.with_probability(FirmConfig.theta)`):

```
marginal_cost mc = wage_rate / (λ · l · month_length)        # λ=3, l=1, month_length=21  ⇒ mc = w/63

if   inventory <  φ_l · D  and  price ≤ ϑ_u · mc :  price ← price · (1 + U(0, υ))   # raise
elif inventory >  φ_u · D  and  price >  ϑ_l · mc :  price ← price · (1 − U(0, υ))   # cut
```

| Symbol | Meaning | Value |
|---|---|---|
| υ (`upsilon`) | upper bound of price adjustment | **0.02** |
| θ (`theta`) | probability the firm considers a price change at all | **0.75** |
| φ_l (`inventory_lphi`) | inventory floor, as a share of last month's demand | **0.25** |
| φ_u (`inventory_uphi`) | inventory ceiling, as a share of last month's demand | **1.0** |
| ϑ_l (`goods_price_lphi`) | price floor, as a multiple of marginal cost | **1.025** |
| ϑ_u (`goods_price_uphi`) | price ceiling, as a multiple of marginal cost | **1.15** |
| λ (`lambda_val`) | units of output per unit of labour per day | **3** |

> **⚠ Correction to the brief.** The brief says the price is *"floored at unit labour cost"* — i.e. a 1.0× multiplier. **Lengnick's floor is 1.025 × marginal cost and there is also a ceiling at 1.15 × marginal cost.** A 1.0× floor permits zero-margin pricing, which means a firm can price at cost indefinitely, never accumulate the χ buffer, never pay dividends, and drag the price level down — precisely the deflationary stall the brief warns about. Use **ϑ_l = 1.025, ϑ_u = 1.15**, and note the ceiling is a genuinely separate mechanism the brief omits entirely: a firm with low inventory will **not** raise its price if it is already at 1.15× cost; it must hire instead.

**Asymmetry:** the *magnitude* distribution is symmetric — `U(0, υ)` both ways, and θ = 0.75 applies to both directions. **All the asymmetry is in the trigger**, via the two different bounds. Do not add a magnitude asymmetry; none is published.

#### Lengnick wage rule (grade B — `firm.py::set_wage_rate`, run every month start, unconditionally — θ does *not* gate it)

```
if   has_open_position                        :  wage ← ceil( wage · (1 + U(0, δ)) ),  min 1
elif months_since_hire_failure ≥ γ            :  wage ← floor( wage · (1 − U(0, δ)) )
```

| Symbol | Meaning | Value |
|---|---|---|
| δ (`delta`) | upper bound of wage adjustment | **0.019** |
| γ (`gamma`) | consecutive months with **no** unfilled position before a wage cut | **24** |

> **⚠ This is the most important number in the entire calibration and the brief does not have it.** γ = 24 means a firm must have had zero unfilled vacancies for **two full simulated years** before it will cut its wage. This is an enormous downward nominal wage rigidity, and it is the mechanism that prevents the wage–price level from collapsing. `months_since_hire_failure` resets to 0 the moment a position is open. The counter logic is inverted-sounding but literal: `if has_open_position: counter = 0 else: counter += 1`.
>
> Rounding is also asymmetric and deliberate: **rises `ceil` (min 1), cuts `floor`** — an integer-arithmetic ratchet that biases wages very slightly upward. Copy this; it is exactly the kind of integer discipline the project wants.

#### BAM price and wage rules (grade B — for contrast)

```
wage:   V > 0 :  w_it = max( ŵ_t , w_{i,t−1}·(1 + ξ) ),  ξ ~ U(0, h_ξ),  h_ξ = 0.05
        V = 0 :  w_it = max( ŵ_t , w_{i,t−1} )

price:  S=0 and P_{i,t−1} <  P̄ :  P_it = max( P_it^l , P_{i,t−1}·(1 + η) )
        S>0 and P_{i,t−1} ≥  P̄ :  P_it = max( P_it^l , P_{i,t−1}·(1 − η) )
        η ~ U(0, h_η),  h_η = 0.1,  P^l = (W + Σ rB)/Y     # unit cost, 1.0× floor

qty:    S=0 and P_{i,t−1} ≥ P̄ :  D^e = Y_{t−1}·(1 + ρ)
        S>0 and P_{i,t−1} <  P̄ :  D^e = Y_{t−1}·(1 − ρ)
        ρ ~ U(0, h_ρ),  h_ρ = 0.1
```

Two structural differences worth knowing: (a) **BAM adjusts price OR quantity, never both in the same period** — the four cases above are mutually exclusive and jointly cover only 4 of the 4 (S, P vs P̄) combinations, one adjustment each; (b) both triggers use **P̄, the economy-wide average price, which BAM declares "common knowledge (global variable)"**. That is a real information shortcut in a published model — see Anti-Features §4.

#### Caiani price rule (grade A — `jmab.strategies.AdaptiveMarkUpOnAC`)

An adaptive markup on average cost, structurally different again:
```
if referenceVariable > threshold :  markUp -= adaptiveParameter · markUp · ζ
else                             :  markUp += adaptiveParameter · markUp · ζ
price = priceLowerBound · (1 + markUp)          # floored at priceLowerBound
```
`threshold = 0.1` (an inventory-to-sales ratio), `adaptiveParameter = 1`, initial `markUp = 0.318857`, `ζ ~ FoldedNormal(0, 0.0094)`.

**Recommendation: Lengnick's price and wage rules verbatim.** They are the only set of the three that needs no economy-wide price index (§4 anti-features), and they are the calibration the brief is already shaped around.

---

### 1.5 Question 5 — Demand expectation ✅ RECOVERED — the brief is exactly right

**Caiani et al., `jmab.expectations.SimpleAdaptiveExpectation` (grade A — authors' own source):**

```java
public void updateExpectation() {
    double result = adaptiveParam * passedValues[0][0] + (1 - adaptiveParam) * passedValues[0][1];
    this.expectation = result;
}
```
with `passedValues[i] = new double[nbVariables + 1]  // +1 because we put also past expectations`, i.e. `[0][0]` = last observed value, `[0][1]` = last expectation. Configured in `modelBenchmark_light.xml`:

```xml
<bean id="expSales" class="jmab.expectations.SimpleAdaptiveExpectation">
  <property name="numberPeriod" value="4"/>
  <property name="adaptiveParam" value="0.25"/>
</bean>
```

So: `E_t = 0.25 · observed_{t−1} + 0.75 · E_{t−1}`, which is algebraically identical to the brief's `expected_demand += λ · (last_sales − expected_demand)` with **λ = 0.25**.

> ✅ **The brief's functional form and its λ = 0.25 are both published, and both are Caiani et al.'s.** The identical bean (λ = 0.25, 4 lags) is reused across `expSales`, `expRSales`, `expWages`, `expDeposits`, `expConsumptionPrice` — it is the framework's standard expectation. No change needed.

**Where the papers disagree.** Lengnick does **not** smooth: `firm.current_demand` is reset to 0 at month start and accumulates actual sales, and the inventory bounds are evaluated against that raw last-month figure — i.e. **naive expectations, λ = 1**. BAM likewise adapts from `Y_{t−1}` directly with a random step, not an EWMA.

**Recommendation: keep λ = 0.25 (Caiani).** With only 20 firms, per-firm sales are noisy and naive expectations would inject that noise straight into the hiring and pricing rules. Note the interaction: if you adopt λ = 0.25 you should keep Lengnick's inventory bounds **relative to expected demand** rather than last month's raw demand — otherwise you have two different demand notions driving price and quantity, which is a subtle desynchronisation bug.

**Companion parameter, also published (grade A):** `TargetExpectedInventoriesOutputStrategy` with `inventoryShare = 0.1`:
```java
double expInv = expSales * inventoryShare;              // 0.1
return Math.max(0, expSales + (expInv - invQuantity));  // desired output
```
Caiani's buffer target is **10% of expected sales**; Lengnick's band is **[25%, 100%]** of monthly demand. These are not comparable (one is a point target, one is a band, and the periods differ). **Use Lengnick's band** — the brief's price rule is a band rule.

---

### 1.6 Question 6 — Dividend / profit distribution ✅ RECOVERED — and the literature disagrees sharply

| Model | Rule | Parameter | Basis | Grade |
|---|---|---|---|---|
| **Lengnick** | Pay out **everything above a working-capital buffer**: `if liquidity > χ·w·L: pay (liquidity − χ·w·L)`. Buffer `= ceil(0.1 · wage_rate · num_workers)` | **χ = 0.1** | **stock** target | B |
| **BAM** | `Div = δ · π` when `π > 0`; retained `= (1−δ)·π` | **δ = 0.15** | **flow** share | B |
| **Caiani** | `FixedShareOfProfitsToPopulationAsShareOfWealthDividends` on after-tax profit | **profitShare = 0.90** (firms); 0.6 (banks) | **flow** share | A |

**Recipients.** Notably, **none of the three papers has single-owner firms.** All three distribute to the whole household population pro-rata to wealth/liquidity:
- Lengnick's scheduler: `shareholding = [(hh, hh.liquidity) for hh in households]`, then `dividend = floor(share · profits / total_shares)` per household. `notes/issues.md`: *"Profit is distributed to all households relative to their current liquidity (as a proxy for wealth)."*
- Caiani: the strategy class name says it — `...ToPopulationAsShareOfWealth...`.

The project's "each firm owned by exactly one household" is a **PROJECT CHOICE** with no published precedent in these three models. It is defensible (it is the seed of the later stock-market milestone) but it is *not* the published approach, and it will produce a far more unequal wealth distribution than any of the three papers — worth remembering when the acceptance harness checks the firm-size and wealth distributions.

> ### 🔴 Strong recommendation: use **Lengnick's stock-target buffer rule (χ = 0.1)**, not a flow share.
>
> The brief names the failure mode precisely: *"Without profits flowing back to owning households, cash accumulates inside firms... the economy deflates into a stall."* The three rules differ in whether they can *fail* that way:
>
> - A **flow** rule (`Div = δ·π`) bounds the *rate* at which cash leaves a firm but places **no bound on the stock**. Retained earnings `(1−δ)·π` accumulate every profitable period. BAM survives δ = 0.15 only because it has a credit market: firm cash is recycled as loan repayments and bank capital, and bankruptcy periodically flushes the accumulated stock. **This project has no banks and no such flush.** At δ = 0.15, 85% of every profit stays inside firms forever, and the brief's stall is not a risk — it is the guaranteed outcome.
> - Caiani's δ = 0.90 avoids that, but it is calibrated against a full SFC model with a government deficit injecting money and a banking system providing working capital. With no credit market, paying out 90% of profits leaves firms unable to fund next period's payroll.
> - **Lengnick's rule is a stock target and is therefore self-correcting by construction.** Firm liquidity is mechanically pinned to `0.1 × payroll` at every month end. It *cannot* accumulate. It simultaneously guarantees the firm can always meet payroll-ish obligations. This is the rule that makes a bankless closed economy work, and it is the rule Lengnick — the model closest to this build — actually uses.
>
> **Adopt: `buffer = ceil(0.1 × offered_wage × num_workers)`; pay `liquidity − buffer` to the owner when positive; χ exposed in config.** Rescale χ for cadence per §1.1 (χ = 0.1 of a *monthly* payroll ≈ 2.1 days of payroll).

**Integer-money implementation note (grade B, directly reusable).** Lengnick's replication is integer-money and conserves exactly, via a pattern worth copying verbatim:
```python
total_paid = 0
for shareholder in shareholding:
    dividend = math.floor(shareholder[1] * dividend_per_share)   # round DOWN per recipient
    shareholder[0].liquidity += dividend
    total_paid += dividend
self.liquidity -= total_paid                                      # subtract what was ACTUALLY paid
```
The firm subtracts `total_paid`, **not** the intended amount. Rounding residue stays with the firm and money is conserved to the unit. Same discipline in `pay_wages` (`liquidity -= num_workers * wage_rate`) and in affordability (`liquidity // price`). With a single owner the rounding problem largely vanishes, but keep the "subtract what was actually transferred" invariant — it is the difference between a conservation check that passes and one that drifts.

---

### 1.7 Question 7 — Firm planning cadence and staggering ⚠ PARTIALLY RECOVERED — staggering is NOT published

**No published model in this class staggers agent decisions.** Both reference implementations are explicitly synchronous:

Lengnick (grade B, `schedule.py`) — the comment says *"according to the precise ordering in the paper"*:
```
if is_month_start():   for firm in firms: firm.month_start()      # ALL firms, same day
                       for hh   in hhs:   hh.month_start()
                       for hh   in hhs:   hh.day()                # households first
                       for firm in firms: firm.day()
if is_month_end():     for firm in firms: firm.month_end()        # pay wages (all firms)
                       shareholders = calculate_shareholdings()   # ONE snapshot, after all wages
                       for firm in firms: firm.distribute_profits(shareholders)
                       for hh   in hhs:   hh.month_end()          # reservation wages last
```
Month start = day 1, 22, 43…; month end = day 21, 42, 63…. BAM is likewise a flat `go` procedure over all firms each period.

**How the papers avoid the lockstep oscillation the brief is worried about:**
1. **Lengnick: θ = 0.75.** Only ~75% of firms consider a price change in any month, drawn independently. This is a Calvo-style random-inaction desynchroniser, and it is the *published* answer to the problem the brief solves with staggering. Note it gates **price only** — the wage rule and the hire/fire rule run every month for every firm.
2. **Household ordering is reshuffled every single day** (`self.model.random.shuffle(self.households)`), so no household has a persistent first-mover advantage in the goods market. `notes/issues.md` #16: the order is drawn once per day and reused for the month-start pass, so processing order changes exactly once per day.
3. **Firms are deliberately NOT shuffled** for the dividend pass (`notes/issues.md` #15) — because the shareholding snapshot is taken once, after all wages are paid, so firm order cannot matter. That is a correctness argument the project should replicate: *if order can matter, shuffle; if you can make order not matter, prove it and don't.*

**Recommendation.** Implement **θ = 0.75 on the price decision** — it is published, it is cheap, and it is the mechanism the paper actually relies on. Keep the weekly stagger too if desired, but **log it as a PROJECT CHOICE deviation, not as "following the literature."** `GAP:` no paper specifies a stagger assignment, so any scheme (`firm_id % 7`, or a seeded random offset drawn once at init) is an invention. Recommend `firm_id % planning_period` for determinism and even load; record it in the decisions table.

> **Interaction warning.** Staggering + Lengnick's γ = 24 counter is subtle: `months_since_hire_failure` must count *planning cycles*, not ticks. If a staggered firm plans every 7 ticks, γ = 24 cycles = 168 ticks, not 24 ticks. Getting this wrong by a factor of 7 destroys the downward wage rigidity that keeps the price level up.

---

### 1.8 Question 8 — Search friction parameters ✅ RECOVERED — the brief's guesses are well supported

| Market | Lengnick | BAM | Caiani | Grade |
|---|---|---|---|---|
| **Firms sampled by a consumer** | 1 new firm/month tested against the incumbent list; a persistent list of **n = 7** preferred suppliers is visited each day | **Z = 2** per period | **nbSellers = 5** (consumption goods market mixer) | B / B / A |
| **Firms sampled by a job seeker** | **β = 5** if unemployed, **1** if employed | **M = 4** per period | 10 (but firm-side: each *firm* samples 10 workers) | B / B / A |

> ✅ **The brief's guess of 5 for both is directly supported.** Caiani samples exactly **5** sellers in the consumption goods market; Lengnick's unemployed job seeker samples exactly **β = 5** firms. Adopt 5 and 5 with citations. Add the published refinement: **an employed job seeker samples only 1** (Lengnick), and searches at all only if unemployed, underpaid (`employer.wage < reservation_wage`), or with probability **π = 0.1**.

#### Memory / preferential attachment — yes, all three have it, and it is not optional

**Lengnick (grade B)** — the richest scheme, and the one the brief alludes to. Households hold a fixed-size list of **n = 7** preferred suppliers ("Type A connections") and buy only from that list, in random order, every day. The list is revised at month start by two independent mechanisms:

```python
# (a) price competition — probability ψ_price = 0.25
target = random.choice(preferred_suppliers)
new    = random firm not already in the list
if new.price < target.price * (1 - ζ):        # ζ = 0.01
    replace target with new

# (b) rationing response — probability ψ_quant = 0.25
#     "blackmarked" = a supplier that ran out of stock this month
target = random.choices(blackmarked, weights = shortfall_amounts)   # weighted by how badly it failed
replace target with a random new firm
```

| Symbol | Meaning | Value |
|---|---|---|
| n (`num_preferred_suppliers`) | size of the preferred-supplier list | **7** |
| ζ (`zeta`) | price improvement required to switch | **0.01** (1%) |
| ψ_price | probability of running the price search | **0.25** |
| ψ_quant | probability of running the rationing search | **0.25** |
| π (`pi`) | probability an employed household job-searches anyway | **0.1** |
| β (`beta`) | firms sampled by an unemployed job seeker | **5** |
| satisfaction fraction | demand is "satisfied" at 95%; the residual 5% is not chased | **0.95** |

**BAM (grade B)** — memory by *size*, not price: `my-large-store` (the largest firm visited last period) is retained and only `Z−1` new firms are drawn. Job seekers retain `my-firm` (previous employer) and draw `M−1` new. So BAM has preferential attachment to large sellers — a deliberate mechanism generating the firm-size distribution.

**Caiani (grade A)** — `CheapestGoodSupplierWithSwitching` + `SwitchingStrategySimple` with `thresholdMean = 0.15` (a probabilistic switching threshold), i.e. switch to a cheaper supplier only if the improvement clears a random threshold with mean 15%.

#### Purchase ordering

- **BAM (grade B)** is cheapest-first with fall-through, exactly as the brief specifies: `min-one-of my-stores [individual-price-P]`, buy `min(money, inventory)`, then `set trials trials - 1` and drop the exhausted store, loop while `trials > 0 and money > 0`. Leftover money → savings.
- **Lengnick (grade B) is NOT cheapest-first**: `self.model.random.shuffle(self.preferred_suppliers)` then buy in that random order. `notes/issues.md`: *"Consumption is undertaken from preferred firms randomly regardless of price."* Price enters only through *which firms are on the list*, via ζ and ψ_price.

**Recommendation for the brief's "cheapest-first with fall-through to next-cheapest on stockout": follow BAM.** It is published, it matches the brief, and it is the stronger competitive channel. But be aware that combining BAM's cheapest-first with Lengnick's persistent 7-firm list makes price competition considerably fiercer than in either paper — if prices deflate in testing, this combination is the first place to look, and reverting the goods market to Lengnick's random ordering within the preferred list is the published fallback.

---

### 1.9 Question 9 — Bankruptcy and entry ✅ RECOVERED — but note that Lengnick has none

**Lengnick has no bankruptcy, no exit, and no entry.** The firm count is fixed forever. Insolvency is handled by cutting wages instead (grade B, `firm.py::pay_wages`):
```python
if self.liquidity < num_workers:                       # can't pay anyone even 1 unit
    self.wage_rate = 0; return                         # "Interns"
if self.liquidity < num_workers * self.wage_rate:
    self.wage_rate = self.liquidity // num_workers     # cut to what's affordable
```
`notes/issues.md`: *"Employees of a failing firm will take an immediate pay cut to keep their jobs."* This is only coherent *because* wages float in Lengnick; it is not portable to a contracted-wage model, and it produces the degenerate "interns work for zero" state (issue #19: *"Wages can be driven to zero (Interns)"*). **The project is right to replace this with bankruptcy.**

**BAM is the published source for bankruptcy and entry (grade B):**

```
FAILURE:   net_worth A ≤ 0  OR  output Y ≤ 0   →  firm dies
           all employees released immediately: employed? = false, wage = 0, contract = 0

INCUMBENT SET for sizing the entrant:  firms trimmed at the 5% tails (a robust mean)

ENTRANT (firm count held constant at J):
  net_worth  = (1 − s) · mean(A     of incumbents)     s = size-replacing-firms = 0.2
  wage_offer = (1 − s) · mean(w^b   of incumbents)
  output     = ceil(     mean(Y     of incumbents))
  price      = 1.26 · average_market_price
  inventory  = 0 ;  employees = none ;  productivity α = 1
```

ODD submodels 39–41: *"Firm that goes bankrupt is replaced with another one of smaller size than the average of incumbent firms. Non-incumbent firms are those whose size is above and below 5%, [trimming] is used to calculate a more robust estimator of the average."*

**Funding of the entrant.** BAM creates the entrant's net worth **ex nihilo** — bank capital absorbs the failed firm's bad debt, and the new firm is endowed. **This is a money-creation event and it would break the project's conservation invariant.** The project's stated approach — *"transfer residual cash to owner, remove firm, respawn a smaller firm owned by a random household"* — is the conservation-preserving adaptation and is correct. The two published pieces the project should take are:
1. **Entrant size = (1 − 0.2) × a 5%-trimmed mean of incumbents** — i.e. 80% of the robust average, not an arbitrary "smaller". The trimming matters: with only 20 firms an untrimmed mean is easily dominated by one outlier.
2. **Entrant price = 1.26 × the average market price.** A conspicuous number, and the direction is counter-intuitive (entrants price *above* the market). It buys the entrant a margin to accumulate net worth from a standing start. Recommend adopting it; if the project instead prices entrants at the market, expect entrants to fail immediately and the bankruptcy rate to run away.

`GAP:` The project's stated edge case — *"redraw when the sampled owner cannot fund a firm"* — has no published counterpart, since no published model funds entrants from a household. It is a reasonable PROJECT CHOICE. Note it interacts with the ownership model: with 20 firms and 200 households, repeated redraws concentrate ownership among the wealthy, which is realistic but will show up in the wealth-distribution acceptance check.

**Caiani (grade A)** uses `FirmBankruptcyFireSales` with `haircut = 0.5` and `FixedShareOfColateralLossComputer` with `shareLoss = 0.5` — a fire-sale of capital at 50% of book. Not applicable: this build has no capital.

---

### 1.10 Consolidated parameter table — drop-in for the project config

Cadence column states the period the published rate refers to. **Rescale per §1.1 before use if the project cadence differs.**

| Project parameter | Published value | Symbol | Source | Cadence | Grade |
|---|---|---|---|---|---|
| Consumption exponent | **0.9** | α | Lengnick T1, Eq(11) | — | B |
| Preferred-supplier list size | **7** | n | Lengnick T1 | — | B |
| Supplier switch price threshold | **0.01** | ζ | Lengnick T1 | — | B |
| P(price search) | **0.25** | ψ_price | Lengnick T1 | month | B |
| P(rationing search) | **0.25** | ψ_quant | Lengnick T1 | month | B |
| Demand satisfaction fraction | **0.95** | — | Lengnick | day | B |
| Firms sampled, unemployed seeker | **5** | β | Lengnick T1 | month | B |
| Firms sampled, employed seeker | **1** | — | Lengnick | month | B |
| P(employed searches anyway) | **0.1** | π | Lengnick T1 | month | B |
| Reservation wage decay, unemployed | **×0.9** | — | Lengnick | month | B |
| Reservation wage rise, employed | **`max(w_r, wage_received)`** (ratchet, not a rate) | — | Lengnick | month | B |
| — alternative | cut/raise by `\|N(0,0.0094)\|`; cut if unemployed >0.49 of last 4; raise if employed and U ≤ 0.08 | — | Caiani `AdaptiveWageStrategy` | period | **A** |
| Price adjustment bound | **0.02** | υ | Lengnick T1 | month | B |
| P(consider price change) | **0.75** | θ | Lengnick T1 | month | B |
| Inventory floor / demand | **0.25** | φ_l | Lengnick T1 | month | B |
| Inventory ceiling / demand | **1.0** | φ_u | Lengnick T1 | month | B |
| Price floor / marginal cost | **1.025** | ϑ_l | Lengnick T1 | — | B |
| Price ceiling / marginal cost | **1.15** | ϑ_u | Lengnick T1 | — | B |
| Wage adjustment bound | **0.019** | δ | Lengnick T1 | month | B |
| Months of full staffing before wage cut | **24** | γ | Lengnick T1 | month | B |
| Productivity (goods per worker-day) | **3** | λ | Lengnick T1 | day | B |
| Dividend buffer / payroll | **0.1** | χ | Lengnick T1 | month | B |
| Demand-expectation smoothing | **0.25** | λ | Caiani `adaptiveParam` | period | **A** |
| Inventory target / expected sales (alt.) | **0.1** | — | Caiani `inventoryShare` | period | **A** |
| Firms sampled by a consumer (alt.) | **5** | nbSellers | Caiani | period | **A** |
| Entrant size vs trimmed mean | **0.8×** | 1−s | BAM `size-replacing-firms`=0.2 | — | B |
| Incumbent trim for the mean | **5% tails** | — | BAM submodel 41 | — | B |
| Entrant price vs market average | **1.26×** | — | BAM `replace-bankrupt` | — | B |
| Bankruptcy trigger | **net worth ≤ 0 or output ≤ 0** | — | BAM submodel 39 | — | B |
| Month length | **21 days** | — | Lengnick | — | B |

**Values the project needs that no paper supplies (`GAP` — must be PROJECT CHOICE, documented as such):**

| Parameter | Why no paper has it |
|---|---|
| Initial household liquidity | Lengnick replication: *"Value not stated in paper"* |
| Initial firm liquidity | *"Value not stated in paper"* |
| Initial goods price | *"Value not stated in paper"* (replication uses 30) |
| Initial wage rate | *"Value not stated in paper"* (replication uses `63 × price`, i.e. one month's output of one worker) |
| Initial reservation wage | *"Value not stated in paper"* (replication uses 0) |
| Initial inventory | *"Value not stated in paper"* (replication uses 0) |
| Initial expected demand | *"Value not stated in paper"* — but must be **> 0**, or the inventory-band and price rules divide by zero (`Eq(6)`, `Eq(7)`) |
| Total money stock | Exogenous and unspecified; the replication exposes it as a slider and its README frames the whole exercise as *"see if you can stop the economy inflating or deflating"* |
| Stagger assignment | No published model staggers |
| Owner-funding of entrants | No published model funds entrants from a household |

> **⚠ The initial conditions are the single largest genuine gap in the literature, and they are load-bearing.** The Lengnick replication devotes the first half of `notes/issues.md` to exactly this, and its interactive README exists specifically to let a user hunt for a money supply that neither inflates nor deflates. **Budget an explicit calibration/burn-in task; do not expect published initial conditions to exist.** The brief's 250-tick burn-in discard is the right instinct but will not substitute for choosing a coherent initial money stock. A defensible starting point *(derived, grade C, from the replication's own bootstrap)*: set the initial wage so that one month's wage buys roughly one month's output of one worker (`w = 63 × p` at λ=3, l=1, 21 days), and set initial household liquidity to a small multiple of the monthly wage.

---

## 2. Feature Landscape

### Table Stakes — without these, this model class does not produce believable dynamics

| Mechanism | Why non-negotiable | Complexity | Notes |
|---|---|---|---|
| **Decentralised goods market with bounded sampling** | The core of the class. Removing it collapses the model to a market-clearing identity — no rationing, no inventories, no price dispersion, no business cycle. | MEDIUM | Lengnick n=7 persistent list; BAM Z=2 w/ memory; Caiani 5. Use 5 + persistence. |
| **Decentralised labour market with bounded sampling** | Involuntary unemployment is *generated by* the matching friction. With full information there is no unemployment to explain. | MEDIUM | β=5 unemployed, 1 employed (Lengnick). |
| **Inventory-band-driven price rule with cost bounds** | The only feedback from quantity signals to prices. Without the cost bounds it deflates through the floor. | LOW | υ=0.02, θ=0.75, φ∈[0.25,1], ϑ∈[1.025,1.15]. |
| **Downward-rigid wage rule (γ)** | γ=24 is what stops the nominal wage–price level collapsing. Symmetric wage adjustment is a known failure mode. | LOW | δ=0.019, γ=24 *cycles*. |
| **Reservation wage with asymmetric dynamics** | The worker-side reservation price. Without it, unemployed workers accept anything and wages have no floor. | LOW | ×0.9/month down; ratchet up. |
| **Dividends via a stock buffer** | The brief's own "single most common way a first build dies". Named a table stake for that reason. | LOW | χ=0.1 of payroll. See §1.6 — must be a **stock** rule. |
| **Adaptive demand expectation** | Firms need a demand signal to size production and staff. | LOW | λ=0.25 (Caiani, grade A). |
| **Inventories as a real, carried stock** | Inventories are the buffer that makes rationing and stockouts possible; they *are* the price signal. | LOW | Must be conserved alongside money. |
| **Hire/fire keyed to the inventory band** | Closes the loop from goods market back to labour market. | MEDIUM | Lengnick: ±1 worker/month. See interdependency note below. |
| **Per-tick reshuffled agent activation order** | Fixed order gives permanent first-mover advantage in a rationed market — a systematic, invisible bias. | LOW | Shuffle households daily (Lengnick). Seeded, so determinism holds. |
| **Integer money with "subtract what was actually paid"** | Conservation to the cent is the project's Core Value. | LOW | Copy Lengnick's `total_paid` pattern (§1.6). |
| **Stock-flow consistency discipline (Caiani)** | Every flow leaves one balance sheet and enters another. This is what the four invariants operationalise. | MEDIUM | The reason Caiani is in the brief's reference list. |
| **Bankruptcy + replacement entry** | Without exit, insolvent firms persist as zombies (Lengnick's "interns") and the firm-size distribution never forms. | MEDIUM | BAM submodels 39–41. |

### Differentiators — present in some models, materially change behaviour

| Mechanism | Value proposition | Complexity | Notes |
|---|---|---|---|
| **Preferred-supplier list with ζ/ψ revision** (Lengnick) | Produces persistent customer–firm networks, price dispersion, and firm-size persistence. The strongest single source of realistic heterogeneity in the class. | MEDIUM | n=7, ζ=0.01, ψ=0.25/0.25. **Recommended.** |
| **Rationing-driven supplier switching** ("blackmarking") | Makes stockouts *costly* to the firm, giving inventories a strategic role beyond a price trigger. Weighted by shortfall size. | MEDIUM | Lengnick only. High value for the brief's "believable dynamics". |
| **θ = 0.75 price inaction** | The published desynchroniser; also produces price stickiness as an emergent property. | LOW | **Recommended over (or alongside) staggering.** |
| **Size-based preferential attachment** (BAM `my-large-store`) | Generates the power-law-ish firm size distribution the acceptance harness checks. | LOW | Alternative to Lengnick's price-based switching; do not stack both. |
| **Cheapest-first purchase with fall-through** | Sharper competition, faster price convergence. | LOW | BAM. Brief already specifies it. Note it is *not* Lengnick. |
| **Statutory minimum wage** (BAM ŵ, updated every 4 periods) | An alternative wage floor to the reservation wage; robust anti-deflation device. | LOW | Redundant with γ=24 + reservation wages. **Skip.** |
| **Price-or-quantity (not both)** adjustment (BAM) | Prevents a firm over-correcting on both margins at once — a real oscillation damper. | LOW | Worth trying if output oscillates. |
| **Employed-vs-unemployed search intensity** (5 vs 1) | Produces on-the-job search and realistic job-to-job flows. | LOW | Lengnick. Cheap, recommended. |
| **Entrant priced at 1.26× market** | Gives entrants a survivable margin; changes the bankruptcy rate a lot. | LOW | BAM. |
| **Trimmed-mean entrant sizing** | Robustness with only 20 firms. | LOW | BAM 5% tails, 0.8× factor. |
| **Wealth-proportional dividends to all households** | The *published* ownership model (all three papers). Less unequal than single ownership. | LOW | Project deviates deliberately. Keep the deviation; know it is one. |
| **Firing notice period** (Lengnick: 1 month) | Smooths employment adjustment; cancellable if inventories fall again. | LOW | Nice damper; note issue #13 (workers on notice keep searching). |

### Anti-Features — do not build, and why each destroys the dynamics

| Anti-feature | Why it gets requested | **Mechanism of destruction** | Alternative |
|---|---|---|---|
| **Walrasian auctioneer / market clearing** | "Just solve for the price where supply = demand — it's simpler and it always balances." | Clearing is a *fixed point computed outside the agents*. It sets excess demand to zero **by construction**, so inventories are always exactly zero and the inventory band `φ ∈ [0.25, 1]` never fires — the price rule has no input. Rationing cannot occur, so stockouts, blackmarking, and supplier switching all become dead code. Unemployment goes to zero because the wage adjusts to clear. Every quantity becomes a deterministic function of the current price vector, so the model loses its state: no path dependence, no autocorrelation in output, no business cycle. You are left with a slow numerical solver for a market equilibrium the agents were supposed to *fail* to reach. **The disequilibrium — the gap between plans and outcomes — is the entire object of study.** | Bilateral matching over a bounded random sample; let trades fail; carry the failure as inventory or unsatisfied demand into the next tick. |
| **Perfect information over prices** | "Households should obviously buy the cheapest — anything else is irrational." | Global price knowledge means every household buys from the single cheapest firm. Price dispersion collapses to a point within one tick, so `ζ = 0.01` (the switching threshold) is never binding and the preferred-supplier network never forms. Demand becomes a step function of price: an ε undercut captures 100% of the market, so the price rule's small `U(0, 0.02)` steps produce enormous discontinuous demand swings — firms whipsaw between stockout and glut, and the price level oscillates violently rather than fluctuating. It also destroys firm-size heterogeneity: the acceptance harness's firm-size distribution check will show one giant and 19 corpses. **This is why n=7 is a *fixed* list, not a scan.** | Bounded sample (5); persistent preferred list; a switching threshold ζ so switching is costly. |
| **Perfect information over vacancies** | "Job seekers should see the whole market." | Every unemployed worker applies to the highest-wage vacancy simultaneously. Matching becomes a global assignment problem, and unemployment becomes purely *voluntary* — anyone unmatched is unmatched only because there are literally no jobs, not because they didn't find one. The model loses frictional unemployment entirely, so the brief's unemployment-band acceptance criterion becomes a knife edge: either ~0% or a mass layoff. The reservation wage also becomes inert, since everyone sees the max offer and there is nothing to be uncertain about. | β = 5 sampled firms (unemployed), 1 (employed); firms hire the first acceptable applicant. |
| **Representative agent shortcuts** ("one average household", "a firm-sector aggregate") | "200 identical households are wasteful; just scale one up." | Aggregation is only valid when the aggregate rule equals the average of the individual rules. **None of the rules here are linear**: `(m/P)^0.9` is concave, so a representative household with mean wealth consumes strictly *more* than the mean of individual consumptions (Jensen); the bankruptcy trigger `A ≤ 0` is a threshold, so a representative firm with positive average net worth never fails even when half the firms are insolvent; hire/fire is a threshold on the inventory band. Aggregating also deletes the distribution, and the distribution *is* the output — firm-size distribution and wealth inequality are acceptance criteria. **You cannot recover a distribution from its mean.** | Keep all 200 and 20 as individuals. At this scale a decade runs in seconds; there is no performance argument. |
| **Global average price as a decision input** | "BAM does it — `P̄` is a global variable." | Honest caveat: **BAM really does this** (ODD submodel 26: *"Aggregate price P_t is common knowledge"*), and BAM works. But it is a deliberate simplification with a cost: it synchronises all firms onto one signal, so they adjust in the same direction at the same time, producing correlated price movements that look like a business cycle but are an artefact of the shared variable. Lengnick avoids it — its households use `P̄` over their **own 7 suppliers** and its firms use only **own inventory and own marginal cost**. | Follow Lengnick: every decision input is local. The reporting layer may compute a price index; no agent may read it. |
| **Overdrafts / negative balances** | "Just let the firm go negative for a tick; it'll recover." | Silently creates money and breaks the conservation invariant — the project's Core Value. Worse, it is an *implicit credit market*, which is explicitly out of scope: the firm is borrowing from nobody at zero interest, so the model quietly becomes a different (and unpublished) model. It also masks the bankruptcy signal that the entry/exit mechanism depends on. | Hard-floor every balance at 0. Insufficient funds → the transaction is truncated or the firm goes bankrupt. Assert `balance ≥ 0` in the invariant block. |
| **Floating-point money** | "Cents are fiddly; f64 has plenty of precision." | Repeated add/subtract over ~3,650 ticks × thousands of transactions accumulates representation error that no epsilon-tolerant assertion will catch cleanly. The conservation check then has to be `abs(total - M0) < ε`, and choosing ε is choosing how much money you are willing to lose. **A conservation invariant with a tolerance is not an invariant.** | Integer minor units throughout; float only in intermediate ratios, rounded at the point of transfer (Lengnick `notes/issues.md` #10 does exactly this). |
| **Rational / model-consistent expectations** | "Adaptive expectations are naive — agents should not be systematically wrong." | Requires agents to know the model's own equilibrium, i.e. to solve the model inside the model. Beyond the computational absurdity at 220 agents, it removes the expectation *errors* that drive inventory accumulation and hence the price rule. The whole class is built on bounded rationality; substituting rational expectations is substituting a DSGE model, which is the thing this literature exists to contrast against. | `E += 0.25·(observed − E)` (Caiani, grade A). |
| **Simultaneous / synchronous state update ("double buffering")** | "Reading half-updated state is a race; snapshot everything and apply at once." | It replaces the sequential-trade semantics with a simultaneous system, and simultaneity re-introduces the clearing problem: two households can each buy the last unit of inventory from the snapshot, and the model must then invent a rationing rule to resolve it. Money and goods conservation become properties you have to *enforce* rather than properties that hold by construction. The brief's tick order (each step completes for all agents before the next begins) is the right design; within a step, trades must still resolve one at a time. | Sequential resolution within each step, over a per-tick shuffled order. Conservation then holds trade-by-trade and the zero-sum-trade invariant is checkable at the transaction level. |
| **Clamping or smoothing "unrealistic" output** | "Prices are spiralling, so let's bound them." | The brief already states the correct policy: unrealistic output is a **defect, not a discovery**. A clamp converts a loud, diagnosable bug into a quiet, permanent distortion, and destroys the acceptance harness's ability to detect it. | Halt and print the tick, agent, and transaction. Fix the price rule, reservation-wage decay, or dividends — the brief's three named suspects, all of which §1 now supplies published values for. |
| **Utility maximisation / intertemporal optimisation** | "Agents should optimise, not follow rules." | Every one of the three papers uses fixed rules of thumb, and the BAM ODD says so explicitly (*"Firms do not have learning"*). Optimisation requires a horizon, a discount rate, and beliefs about future prices — none of which exist here — and would make each agent step orders of magnitude more expensive, defeating the "decade in seconds" constraint that makes debugging possible. | Published rules of thumb, verbatim, from §1. |
| **Multi-threading the agent loop** | "220 agents × 3,650 ticks is embarrassingly parallel." | It is not parallel: sequential trade resolution is inherently ordered, and any parallel reduction over floats or hash maps introduces nondeterminism. Byte-identical logs for a seed are a *test* in this project; threading forfeits it. | Single-threaded, single seeded RNG. The performance budget does not require anything else. |

---

## 3. Feature Dependencies

```
Money conservation invariant
    └──requires──> Integer cents + "subtract what was actually transferred"
    └──requires──> No overdrafts (hard 0 floor on every balance)
    └──conflicts──> Ex-nihilo entrant funding (BAM's own approach)
                         └──resolved by──> Fund entrant from owner household's cash

Dividend payout (χ buffer)
    └──requires──> Ownership relation (firm → owning household)
    └──requires──> Firm accounting step (revenue − wage bill) ordered AFTER wages
    └──requires──> Wage payment completed for ALL firms first
                   (Lengnick takes ONE shareholding snapshot after all wages —
                    otherwise firm order changes who gets paid what)
    └──enables───> Household liquidity ──> consumption budget (m/P̄)^0.9
                   ^^^ THE LOAD-BEARING CYCLE. Break it and the economy stalls.

Consumption budget (m/P̄)^0.9
    └──requires──> Preferred-supplier list (P̄ is the mean over THAT list, not global)
    └──requires──> Household liquidity ← wages + dividends
    └──feeds─────> Firm sales ──> demand expectation (λ=0.25) ──> inventory band
                                                                       └──> price rule
                                                                       └──> hire/fire
                                                                             └──> wages
                                                                                   └──> household liquidity  (closes the loop)

Reservation wage
    └──requires──> Employment state + the wage actually received (ratchet)
    └──interacts──> Labour market acceptance test (accept if offer > w_r OR > current wage)
    └──interacts──> Wage rule via vacancies:
                    w_r too high ──> offers rejected ──> vacancy stays open
                                 ──> firm raises wage (δ) ──> γ counter resets to 0
                                 ──> wage cuts become impossible ──> wage/price spiral
                    *** This is why the decay rate is a prime suspect for a spiral.
                        At the brief's 1%/day (19%/month) instead of Lengnick's 10%/month,
                        the reservation wage falls FASTER, which pushes the other way:
                        offers accepted too readily, wages sag, deflation. Either error
                        is diagnosable only if the published value is the reference. ***

Wage contract semantics (contracted at hire)
    └──conflicts──> Lengnick's insolvency handling (cut wage_rate to affordable)
                    which only works when wages float
    └──requires──> Bankruptcy mechanism as the replacement (BAM)
    └──requires──> Household.wage separate from Firm.offered_wage (project already has this)

Bankruptcy + entry
    └──requires──> Firm net worth / liquidity tracking
    └──requires──> Trimmed-mean statistics over incumbents (needs ≥ ~5 survivors;
                    with only 20 firms a 5% trim removes exactly 1 from each tail)
    └──requires──> Worker release path (contracts terminate)
    └──requires──> Ownership relation (residual cash → owner)
    └──enhances──> Firm-size distribution (an acceptance criterion)

Search frictions (bounded sampling)
    └──enables───> Price dispersion ──> supplier switching (ζ) ──> firm-size heterogeneity
    └──enables───> Frictional unemployment ──> reservation wage relevance
    └──conflicts──> ANY global price/vacancy visibility

θ = 0.75 price inaction  ──enhances──> desynchronisation
Weekly stagger           ──enhances──> desynchronisation   (redundant with θ; keep both, know one is a deviation)
    └──requires──> γ counted in PLANNING CYCLES, not ticks   *** off-by-7 destroys wage rigidity ***

Determinism (byte-identical logs)
    └──requires──> Single seeded RNG, single-threaded
    └──requires──> No iteration over hash maps
    └──requires──> Per-tick shuffle to be SEEDED (shuffle is required for correctness,
                    determinism is required by the brief — these are compatible, but only
                    if the shuffle draws from the same RNG stream)
    └──conflicts──> Multi-threading, float reductions
```

### Dependency Notes

- **Dividends → consumption is the model's only cycle-closing flow.** In a bankless economy the *only* routes from firm cash back to households are wages and dividends. Wages are bounded below by the firm's ability to pay and above by γ-rigidity; dividends are the residual. Cut them and firm liquidity is a one-way sink. Build dividends in the same phase as firm accounting, never later.
- **Reservation wage decay ↔ labour market are a tight two-way coupling** in both directions (see the ASCII note above). Because either sign of error produces a plausible-looking pathology, the *published* rate is the only reliable anchor. Take Lengnick's 10%/month, converted for cadence.
- **γ must count planning cycles.** With weekly planning, γ = 24 cycles = 24 weeks ≈ 5.5 months — already much weaker than Lengnick's 24 months. If the project wants Lengnick's rigidity it needs γ = 72 weeks. Flag this as an explicit calibration decision.
- **Hire/fire rule choice is not free.** Lengnick adjusts by **±1 worker per month** keyed to the inventory band; BAM computes `L^d = Y^d/α` and posts `V = max(L^d − L, 0)`. With 20 firms and 200 households the average firm has 10 workers, so Lengnick's ±1 is a 10% workforce swing per cycle — at a weekly cadence that is very fast. **Recommend BAM's `L^d = expected_demand / λ`** since the project has already committed to `expected_demand`, and it scales sensibly with firm size. Note the deviation: Lengnick's firm-size distribution is partly an artefact of the ±1 rule.
- **Preferred-supplier list conflicts with pure cheapest-first.** Not fatally, but stacking Lengnick's persistent list *and* BAM's cheapest-first ordering *and* Caiani-style switching produces stronger price competition than any published model. Pick the brief's cheapest-first-within-the-list, and keep Lengnick's random-order-within-list as the documented fallback if prices deflate.
- **Trimmed-mean entry is marginal at 20 firms.** A 5% trim of 19 survivors removes ~1 firm from each tail. It still helps (one bankrupt-adjacent outlier can halve the mean), but consider trimming a fixed 1 from each tail rather than a percentage.

---

## 4. MVP Definition

### Launch With (v1) — the brief's milestone

Ordered so each item's dependencies precede it.

- [ ] **Integer-cent money + ID-based agent vectors + seeded RNG** — every later item depends on these; retrofitting is a rewrite.
- [ ] **Four invariants + halt-on-violation** — must exist before the first trade, or early bugs are invisible.
- [ ] **Fixed tick order with per-tick seeded household shuffle** — the shuffle is a correctness requirement (§2 table stakes), not a nicety.
- [ ] **Firm planning: adaptive demand expectation (λ=0.25) → desired output → labour demand** — the head of the causal chain.
- [ ] **Price rule (υ=0.02, θ=0.75, φ∈[0.25,1.0], ϑ∈[1.025,1.15])** — note **both** bounds.
- [ ] **Wage rule (δ=0.019, γ in planning cycles)** — with `ceil` on rises, `floor` on cuts.
- [ ] **Decentralised labour market (β=5 / 1, π=0.1) + reservation wage (decay ×0.9/month, ratchet up)** — wage contracted at hire.
- [ ] **Production + inventory as a conserved stock.**
- [ ] **Decentralised goods market (5 sampled, persistent list n=7, ζ=0.01, ψ=0.25/0.25, cheapest-first with fall-through).**
- [ ] **Household consumption `(m/P̄)^0.9`** where `P̄` is the mean over the household's own list.
- [ ] **Firm accounting → dividends above `χ=0.1 × payroll` to the owning household** — same phase as accounting. **This is the item most likely to be deferred and most fatal to defer.**
- [ ] **Bankruptcy + replacement entry (0.8 × 5%-trimmed mean, price 1.26 × market, funded by owner).**
- [ ] **Structured per-tick + per-event logging with decision provenance.**
- [ ] **Python acceptance harness** — conservation, unemployment band, price stability, output autocorrelation, firm-size distribution, seed-reproducibility diff.

### Add After Validation (v1.x) — trigger: the 3,650-tick run passes

- [ ] **Rationing-driven supplier switching (blackmarking, shortfall-weighted)** — trigger: firm-size distribution is too uniform, or stockouts have no consequence.
- [ ] **BAM's price-or-quantity (never both)** — trigger: output oscillates at the planning frequency.
- [ ] **Firing notice period (1 cycle, cancellable)** — trigger: employment is too jumpy.
- [ ] **Sensitivity sweep over γ, χ, α, λ, decay rate** — trigger: the run passes but only at one point in parameter space.
- [ ] **Cross-validation against Lengnick's published stylised facts** — trigger: acceptance passes; this is the real external validity check.

### Future Consideration (v2+) — explicitly out of scope

Banks/credit, government/taxes, multiple goods, capital, R&D, demographics, stock market, geography, endogenous founding, GUI, plotting toolkit, scaling. **Per the brief, build no scaffolding for these.** The three forward-compatibility constraints already in PROJECT.md (goods as data, ownership as a relation, provenance from tick 1) are sufficient and should not be extended.

---

## 5. Feature Prioritization Matrix

| Mechanism | Model Value | Implementation Cost | Priority |
|---|---|---|---|
| Integer money + invariants + determinism | HIGH | LOW | **P1** |
| Dividends via χ buffer | HIGH | LOW | **P1** |
| Reservation wage (published rates) | HIGH | LOW | **P1** |
| Price rule with **both** cost bounds | HIGH | LOW | **P1** |
| Wage rule with γ rigidity | HIGH | LOW | **P1** |
| Bounded sampling, both markets | HIGH | MEDIUM | **P1** |
| Adaptive expectation λ=0.25 | HIGH | LOW | **P1** |
| Per-tick seeded shuffle | HIGH | LOW | **P1** |
| Bankruptcy + trimmed-mean entry | HIGH | MEDIUM | **P1** |
| Preferred-supplier list (n=7, ζ, ψ) | HIGH | MEDIUM | **P1** |
| Provenance logging | MEDIUM | MEDIUM | P1 (brief requires) |
| θ = 0.75 price inaction | MEDIUM | LOW | **P1** (published desynchroniser) |
| Weekly stagger | MEDIUM | LOW | P2 (project choice, redundant with θ) |
| Blackmarking / rationing switch | MEDIUM | MEDIUM | P2 |
| Employed-vs-unemployed search intensity | MEDIUM | LOW | P2 |
| Entrant price 1.26× market | MEDIUM | LOW | P2 |
| Firing notice period | LOW | LOW | P2 |
| Price-or-quantity (BAM) | MEDIUM | LOW | P3 (diagnostic tool) |
| Statutory minimum wage | LOW | LOW | P3 (redundant) |
| Wealth-proportional dividends to all | LOW | LOW | P3 (project deliberately deviates) |
| Size-based preferential attachment | LOW | LOW | P3 (conflicts with ζ switching) |

---

## 6. Cross-Model Comparison — where the literature disagrees, and what to take

| Mechanism | Lengnick | BAM | Caiani | **Recommendation** |
|---|---|---|---|---|
| Consumption | `(m/P̄)^0.9`, concave in own real balances | `1/(1+tanh(SA/SA̅)^0.87)` × wealth | `0.386·E[Y] + 0.25·W` | **Lengnick** — only local info, matches brief's shape |
| Wage contract | **Floats** with firm's rate | **Fixed at hire**, `8+Poisson(10)` | matched/posted | **BAM (contract at hire)** — required by the project's field layout; re-attribute the decision |
| Reservation wage | ×0.9/mo; ratchet to own wage | **none** | ±\|N(0,0.0094)\| on 0.49/0.08 triggers | **Lengnick** — simplest, self-limiting; Caiani's is the grade-A fallback |
| Wage floor | γ=24 rigidity + reservation wage | statutory minimum wage | wageLowerBound | **Lengnick** |
| Price floor | **1.025 × MC** | 1.0 × unit cost | adaptive markup on AC | **Lengnick 1.025** — a 1.0 floor invites zero-margin deflation |
| Price ceiling | **1.15 × MC** | none | none | **Lengnick** — the brief omits it entirely |
| Price trigger | own inventory band only | own inventory **and** global P̄ | inventory/sales ratio | **Lengnick** — no global variable |
| Expectations | naive (λ=1) | random step on Y_{t−1} | **EWMA λ=0.25** | **Caiani λ=0.25** — 20 firms is too noisy for naive |
| Inventory target | band [0.25, 1.0] × demand | S=0 vs S>0 (binary) | point target 0.1 × expSales | **Lengnick band** — brief's rule is a band rule |
| Dividends | **stock buffer, χ=0.1** | flow, δ=0.15 | flow, 0.90 | **Lengnick** — only a stock rule survives without banks (§1.6) |
| Dividend recipients | all households ∝ liquidity | shareholders | all households ∝ wealth | **Project deviates** (single owner) — deliberate, documented |
| Goods sampling | 7 persistent, +1 test/mo | Z=2 + largest-store memory | 5 + switching threshold | **5 sampled from a persistent 7-list** |
| Labour sampling | β=5 / 1 employed | M=4 | 10 (firm-side) | **Lengnick 5 / 1** |
| Purchase order | **random within list** | **cheapest-first, fall-through** | cheapest w/ switching | **BAM cheapest-first** (brief specifies it); Lengnick random-order is the deflation fallback |
| Labour demand | ±1 worker per month | `L^d = Y^d/α`, `V = max(L^d−L,0)` | expectation-driven | **BAM** — scales with firm size; brief already has expected_demand |
| Bankruptcy | **none** (wage→0 "interns") | A≤0 or Y≤0 → die + replace | fire sales, haircut 0.5 | **BAM** |
| Entry sizing | n/a | 0.8 × 5%-trimmed mean; price 1.26× | n/a | **BAM** |
| Entry funding | n/a | ex nihilo | recapitalisation | **PROJECT: from owner's cash** (conservation) |
| Desynchronisation | **θ=0.75** on price + daily reshuffle | none | none | **Lengnick θ=0.75**; stagger is an extra |
| Staggering | **none** | none | none | **PROJECT CHOICE** — document as a deviation |

**Net recommendation: a Lengnick core, with BAM's contract-at-hire, cheapest-first buying, labour demand, and bankruptcy/entry, and Caiani's λ=0.25 expectation.** This is a coherent synthesis, not a pick-and-mix: Lengnick supplies everything that must be *local*, BAM supplies everything Lengnick lacks (exit/entry) or handles degenerately (insolvency), and Caiani supplies the one thing both handle naively (expectations) plus the accounting discipline. Every substitution above is justified by a structural difference between this build and the source model — chiefly the absence of banks and the presence of explicit single ownership.

---

## 7. Open Gaps

Stated as gaps. **Not filled with invention.**

1. **Lengnick Table 1 not read from the paper.** All Lengnick values are grade B, from an annotated replication. Highest-value remaining verification: spot-check α, γ, θ, υ, δ, φ, ϑ, χ, n, ζ, ψ, β, π against the published Table 1. Suggested source: `https://www.econstor.eu/bitstream/10419/45012/1/654079951.pdf` (open access; blocked in this session).
2. **Lengnick Eq (12)** — exact text not recovered. The replication reports it near-vacuous and omits it. Omit; do not substitute.
3. **BAM's book-stated labour contract length θ** — not recovered. The reference NetLogo uses `8 + Poisson(10)`; I could not confirm this equals the book's value. Treat contract length as a PROJECT CHOICE.
4. **All initial conditions** — genuinely unspecified in Lengnick (the replication says so, item by item). Requires a calibration task.
5. **Total money stock** — exogenous and unspecified in every source. This is the free parameter that decides whether the economy inflates or deflates. Budget explicit exploration.
6. **Stagger assignment scheme** — no published model staggers, so no published scheme exists.
7. **Owner-funded entry** — no published counterpart; a conservation-driven PROJECT CHOICE.
8. **Rate rescaling for a weekly cadence** — the conversions in §1.1 are my arithmetic (grade C), not published. If the project adopts a 21-day cadence instead, no rescaling is needed and this gap closes.
9. **Whether the Lengnick calibration survives at 200/20 rather than 1000/100** — unknown. The 10:1 ratio is preserved, but small-N effects (20 firms, 5% trims, ±1 hiring granularity) are not addressed anywhere in the literature. Expect calibration work; this is the most likely source of "unrealistic output" that is *not* a coding defect.

---

## Sources

**Grade A — model authors' own repositories**
- `S120/jmab` (JMAB framework, © Alessandro Caiani and Antoine Godin) — `src/jmab/expectations/SimpleAdaptiveExpectation.java`, `src/jmab/strategies/{AdaptiveMarkUpOnAC, AdaptiveWageStrategy, TargetExpectedInventoriesOutputStrategy, SwitchingStrategySimple, ConsumptionFixedPropensitiesOOIWWithPersistency}.java` — https://github.com/S120/jmab
- `S120/benchmark` — `benchmark/Model/modelBenchmark_light.xml` (complete parameterisation), `benchmark/src/benchmark/agents/Households.java` — https://github.com/S120/benchmark
  - For: Caiani, Godin, Caverzasi, Gallegati, Kinsella & Stiglitz (2016), *Agent based-stock flow consistent macroeconomics: Towards a benchmark model*, JEDC 69: 375–408.

**Grade B — annotated third-party replications**
- `newwayland/baseline-economy` — `BaselineEconomy/{household,firm,model,schedule}.py` (parameters annotated `# Calibration values (Table 1)`, equations cited `Eq(5)–(12)`) and `notes/issues.md` (documented deviations and paper silences) — https://github.com/newwayland/baseline-economy
  - For: Lengnick, M. (2013), *Agent-based macroeconomics: A baseline model*, JEBO 86: 102–120. doi:10.1016/j.jebo.2012.12.021
- `alexplatasl/BAMmodel` — `README.md` (full ODD protocol: initialisation table + submodels 1–44) and `DelliBAM_.nlogo` (reference implementation + slider defaults) — https://github.com/alexplatasl/BAMmodel
  - For: Delli Gatti, Desiderio, Gaffeo, Cirillo & Gallegati (2011), *Macroeconomics from the Bottom-up*, Springer (New Economic Windows).

**Blocked in this session (listed so the verification step knows where to look)**
- Lengnick working-paper PDF (open access): https://www.econstor.eu/bitstream/10419/45012/1/654079951.pdf
- Lengnick JEBO version: https://www.sciencedirect.com/science/article/abs/pii/S0167268112002806
- Caiani et al. PDF: https://faculty.sites.iastate.edu/tesfatsi/archive/tesfatsi/ABMSFCMacroModelBenchmark.CainiEtAl2016.pdf
- Platas-López et al., *Micro-foundations of macroeconomic dynamics: the agent-based BAM model* (CCIA 2019): https://www.uv.es/grimo/publications/ccia2019.pdf
- Lengnick model description: https://sim4edu.com/sims/20/description

**Confidence seam output (recorded in the research cache):** `classify-confidence --provider webfetch` → `LOW`; `--provider websearch --verified` → `MEDIUM`. Evidence grades in §0 supplement these provider-level tiers.

---
*Feature research for: minimal closed-economy agent-based macroeconomics*
*Researched: 2026-08-30*
