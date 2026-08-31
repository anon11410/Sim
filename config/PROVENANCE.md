# Parameter Provenance

**Status of this document: no value in an `UNVERIFIED` row below has been read from a primary
source by any agent — not in this session, and not in the previous research round.** Every such
value is inherited from an annotated replication that cites the paper's table and equation numbers
inline. That is what grade B means, and marking those rows honestly is the whole point of this
file. A number that looks sourced and is not is worse than one that admits it is not.

This document is the companion to the per-key `# GRADE: … | SOURCE: … | CADENCE: …` annotation
blocks in `config/baseline.toml`. The annotations make a value's provenance legible at the point of
use; this file makes it enumerable, gives it a verification state, and carries the procedure that
closes the gap. `tests/provenance.rs` asserts the two stay in step: every key in the config has a
row here, and no row attributed to the published literature can be marked verified without a
deliberate edit to that test.

Requirement: **CORE-11**, in two separately gated clauses. Clause (a) — annotation — is delivered
here in Phase 1. Clause (b) — checking the baseline-model paper's Table 1 — is a **blocking gate on
Phase 6**, the first phase that consumes those values (`01-CONTEXT.md` D-19).

---

## 1. The grade vocabulary

This vocabulary is **reused, not invented here**. It is defined at
`.planning/research/SUMMARY.md:169`, quoted verbatim:

> Every value carries its source grade: **A** = model authors' own code, **B** = annotated
> replication citing the paper's table/equation numbers, **C** = derived arithmetic, **PROJECT**
> = a choice with no published precedent. These close the gaps in the project's own parameter
> table and are the single most valuable research output.

A grade letter therefore means exactly the same thing in `config/baseline.toml`, in
`.planning/research/SUMMARY.md` and in this file. There is no fifth letter and no local
redefinition.

The same source also records, at `.planning/research/SUMMARY.md:211`, what must **not** be treated
as sourced:

> **Do NOT treat as sourced:** the weekly/daily conversions (grade C arithmetic), and everything in
> the PROJECT rows. The Lengnick numbers are grade **B** — recovered from an annotated replication
> that cites Table 1 and Eq(5)–(12) inline, not read from the PDF.

**Cadence vocabulary.** `day` — the parameter acts once per tick. `month` — once per accounting
month, 21 ticks. `period` — once per the source model's own planning period, whose mapping onto
this model's month is a Phase 5+ question (see V-5). `none` — the parameter is a level, a bound or
a ratio with no cadence of its own.

**Verification states.** `UNVERIFIED` — grade B, inherited from a replication, never read from the
published article; closed by the procedure in section 3. `VERIFIED — authors' code` — grade A, the
value is the model authors' own committed source, which is itself the primary artefact.
`UNVERIFIED — derived from a grade-B source` — grade C, the arithmetic is checkable here but its
input is not. `N/A — project choice` — grade PROJECT, there is no external source to check it
against; what such a row needs is calibration, not verification, and CAL-01 / CAL-02 own that.

---

## 2. The per-key table

One row per leaf key in `config/baseline.toml`. 41 keys, 41 rows — the count is set by the schema
in `src/config.rs`, never by the graded table's row count.

| Key | Value | Grade | Source | Cadence | Verification state |
|---|---|---|---|---|---|
| `sim.ticks` | 3650 | PROJECT | PROJECT.md run length (10 simulated years x 365 days) | day | N/A — project choice |
| `sim.seed` | 42 | PROJECT | Project choice, arbitrary — any seed is as valid as any other | none | N/A — project choice |
| `sim.households` | 200 | PROJECT | PROJECT.md sizing; graded table row "Single-owner firms" | none | N/A — project choice |
| `sim.firms` | 20 | PROJECT | PROJECT.md sizing; graded table row "Single-owner firms" | none | N/A — project choice |
| `sim.month_days` | 21 | B | Lengnick 2013 JEBO, month length, via annotated replication | none | UNVERIFIED |
| `money.total_money_cents` | 2000000 | PROJECT | Project choice; free parameter, CAL-02 defers calibration to Phase 11 | none | N/A — project choice |
| `household.consumption_exponent_ppm` | 900000 (α = 0.9) | B | Lengnick 2013 JEBO Table 1 / Eq(11), via annotated replication | none | UNVERIFIED |
| `household.supplier_list_size` | 7 (n) | B | Lengnick 2013 JEBO Table 1, via annotated replication | none | UNVERIFIED |
| `household.supplier_switch_threshold_ppm` | 10000 (ζ = 0.01) | B | Lengnick 2013 JEBO Table 1, via annotated replication | none | UNVERIFIED |
| `household.price_search_prob_ppm` | 250000 (ψ_price = 0.25) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `household.rationing_search_prob_ppm` | 250000 (ψ_quant = 0.25) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `household.firms_sampled_consumer` | 5 | A | Caiani et al. jmab, `nbSellers` in the authors' own code | period | VERIFIED — authors' code |
| `household.firms_sampled_unemployed` | 5 (β) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `household.firms_sampled_employed` | 1 | B | Lengnick 2013 JEBO baseline model, via annotated replication | month | UNVERIFIED |
| `household.employed_search_prob_ppm` | 100000 (π = 0.1) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `household.reservation_wage_decay_ppm` | 900000 (×0.9) | B | Lengnick 2013 JEBO baseline model, via annotated replication | month | UNVERIFIED |
| `household.reservation_wage_floor_cents` | 1000 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `household.initial_liquidity_cents` | 5000 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `household.initial_reservation_wage_cents` | 6300 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `firm.productivity_units_per_worker_day` | 3 (λ_prod) | B | Lengnick 2013 JEBO Table 1, via annotated replication | day | UNVERIFIED |
| `firm.demand_smoothing_ppm` | 250000 (λ = 0.25) | A | Caiani et al. jmab, SimpleAdaptiveExpectation `adaptiveParam` | period | VERIFIED — authors' code |
| `firm.price_step_bound_ppm` | 20000 (υ = 0.02) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `firm.price_inaction_prob_ppm` | 750000 (θ = 0.75) | B | Lengnick 2013 JEBO Table 1, via annotated replication — **sense flagged, see V-4** | month | UNVERIFIED |
| `firm.inventory_floor_ppm` | 250000 (φ_l = 0.25) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `firm.inventory_ceiling_ppm` | 1000000 (φ_u = 1.0) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `firm.price_floor_over_mc_ppm` | 1025000 (ϑ_l = 1.025) | B | Lengnick 2013 JEBO Table 1, via annotated replication | none | UNVERIFIED |
| `firm.price_ceiling_over_mc_ppm` | 1150000 (ϑ_u = 1.15) | B | Lengnick 2013 JEBO Table 1, via annotated replication | none | UNVERIFIED |
| `firm.wage_step_bound_ppm` | 19000 (δ = 0.019) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `firm.full_staff_cycles_before_wage_cut` | 24 (γ) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `firm.dividend_buffer_ppm` | 100000 (χ = 0.1) | B | Lengnick 2013 JEBO Table 1, via annotated replication | month | UNVERIFIED |
| `firm.demand_satisfaction_ppm` | 950000 (0.95) | B | Lengnick 2013 JEBO baseline model, via annotated replication | day | UNVERIFIED |
| `firm.wage_floor_cents` | 1000 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `firm.initial_price_cents` | 105 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `firm.initial_wage_cents` | 6300 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `firm.initial_inventory_units` | 165 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `firm.initial_expected_demand` | 330.0 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 — **cadence unpinned, see V-5** | none | N/A — project choice |
| `firm.initial_liquidity_cents` | 50000 | PROJECT | Project choice, no published precedent; CAL-01 defers calibration to Phase 11 | none | N/A — project choice |
| `bankruptcy.entrant_size_ratio_ppm` | 800000 (0.8×) | B | BAM `size-replacing-firms` = 0.2, via annotated replication | none | UNVERIFIED |
| `bankruptcy.entrant_price_ratio_ppm` | 1260000 (1.26×) | B | BAM `replace-bankrupt`, via annotated replication | none | UNVERIFIED |
| `bankruptcy.incumbent_trim_per_tail` | 1 | C | Derived arithmetic — 5% of 20 firms = 1, from BAM submodel 41 (itself grade B) | none | UNVERIFIED — derived from a grade-B source |
| `ownership.firms_per_owner` | 1 | PROJECT | Project choice, no published precedent; graded table row "Single-owner firms" | none | N/A — project choice |

**Counts.** 41 rows: 2 grade A, 23 grade B, 1 grade C, 15 grade PROJECT. Of the 23 grade-B rows,
**21 are attributed to the baseline-model paper (Lengnick 2013)** and 2 to BAM. `01-CONTEXT.md`
D-19 states 18; that number counts *rows of the graded table*, whereas this table counts *config
keys*, and one graded row can expand into two keys (`ψ_price` / `ψ_quant` is a single graded row
and two keys here). The larger number is the one to work from: it is the set of keys a person must
actually check.

---

## 3. The verification procedure

**Who executes this:** a person with journal access. No domain knowledge is required — the
procedure is a lookup and a transcription, not a modelling judgement.

**When:** this is a **blocking** gate on **Phase 6**, the first phase that consumes these values
(`01-CONTEXT.md` D-19, `.planning/REQUIREMENTS.md` CORE-11 clause (b)). It does not block Phase 1.

**Do not schedule another automated fetch.** Egress was denied on all six candidate hosts —
`sciencedirect.com`, `legacy.econ.tuwien.ac.at`, `macau.uni-kiel.de`, `econstor.eu` /
`ideas.repec.org`, `sim4edu.com`, and the replication repository's own source — across **two
independent research passes**. A third agent attempt is waste.

### Step 1 — open the document

Open the **published article**: Lengnick, *Agent-based macroeconomics: a baseline model*, Journal
of Economic Behavior & Organization **86** (2013) 102–120. The published article is grade A for
this purpose; a replication is not.

An open-access mirror URL is recorded in the research assumptions log
(`01-RESEARCH.md`, assumption A2:
`legacy.econ.tuwien.ac.at/lva/compeco.se/artikel/jebo_2013_agent_based_macroeconomics_a_baseline_model.pdf`).
**That URL came from a search-result title, not from a successful fetch, and may not resolve.**
If it does not, journal access is the fallback. Do not substitute a replication, a lecture slide, a
dissertation chapter or a summary for the published article.

### Step 2 — for each `UNVERIFIED` row, record exactly one outcome

Work down the 21 Lengnick-attributed rows in the table above. For each, find the parameter in the
paper's Table 1 (or the cited equation) and write down exactly one of:

- **`agrees`** — the paper's value matches the value in this table.
- **`differs`** — *and write down the value the paper actually gives.* The paper's number goes in
  this document, in writing, next to the row.
- **`not in Table 1`** — the parameter is not in the paper's table at all; record where it was
  looked for.

Record the outcome, the date, and who checked it. A row with no recorded outcome stays
`UNVERIFIED`.

### Step 3 — the rule for a discrepancy

**A differing value is written down AND the config is updated with a note pointing at this row.
It is never silently overwritten.** The record of the discrepancy is as much a deliverable as the
corrected number: a future reader must be able to see that the project once held a different value
and why it changed. `.planning/ROADMAP.md` Phase 1 criterion 5 and `01-CONTEXT.md` D-20 both
require exactly this.

### Step 4 — release the rows

Change the row's verification state from `UNVERIFIED` to a recorded outcome, **and update
`tests/provenance.rs::attributed_rows_are_still_marked_unverified` in the same commit**. That test
fails the moment a row is upgraded, by design: it forces the upgrade and the evidence to arrive
together, in one reviewable change. It is not an obstacle to route around.

### Open items to check alongside Table 1

These are questions this project raised that a paper read can settle cheaply. They are not
blockers on their own.

- **V-1 — the ψ split.** The graded table gives `P(price search)` and `P(rationing search)` as one
  row, `0.25 / 0.25`. Confirm the paper has two distinct parameters and both are 0.25.
- **V-2 — reservation wage, employed.** The graded table records a *ratchet*
  (`max(w_r, wage_received)`), not a rate, and no config key carries it. Confirm the rule.
- **V-3 — the BAM rows.** `bankruptcy.entrant_size_ratio_ppm` and
  `bankruptcy.entrant_price_ratio_ppm` come from a different paper (BAM), not from Lengnick, and
  are equally unread. If BAM is to hand, check them on the same pass.

- **V-3a — `entrant_size_ratio_ppm`: the shipped value is not the cited value.** The row states a
  SOURCE value of **0.2** (`BAM size-replacing-firms = 0.2`) and ships **800000 ppm = 0.8**, with
  the config annotation reading "Entrant size against the trimmed mean of incumbents: 0.8x". No
  derivation is recorded anywhere, so exactly one of the following is true and the repository does
  not currently say which:

  1. the shipped value is a **transcription error** and should be 200000 ppm; or
  2. the shipped value is **derived** from the source as `1 − 0.2`, in which case the row is grade
     **C** (derived arithmetic), not grade B, and the derivation belongs in the SOURCE field —
     exactly as `incumbent_trim_per_tail` already does it ("derived arithmetic — 5% of 20 firms
     = 1"); or
  3. `size-replacing-firms` does not mean what the row assumes, and the 0.8 came from elsewhere.

  **This has deliberately not been resolved from model memory** (D-20): picking whichever reading
  looks plausible is precisely the failure the grading scheme exists to prevent, and the value
  changes how large every replacement firm enters at. Settle it on the same BAM pass as V-3, then
  apply section 3 step 3 — write the outcome down, update the config annotation to match, and
  regrade the row to C if reading 2 is the right one.

  None of the six checks in `tests/provenance.rs` can see this: test 5 checks only that a row
  *exists* for the key, and test 6 checks only that grade-B rows stay `UNVERIFIED`. It is recorded
  here because a numeric mismatch that no test can catch is exactly what an open item is for.
- **V-4 — the sense of θ.** The graded table reads θ = 0.75 as **P(firm considers a price
  change)**, while the config key is named `price_inaction_prob_ppm` — the complementary event.
  One of the two readings is wrong, and which one changes how often prices move by a factor of
  three. **Check the sense of θ in the paper explicitly**, and record it. Until then the value is
  transcribed as the graded table gives it; per D-20 it has not been "corrected" from memory.
- **V-5 — the cadence of the demand expectation.** `firm.initial_expected_demand` is a per-month
  quantity while `firm.productivity_units_per_worker_day` is per-day and the smoothing λ is per
  *period*. The cadence of the demand expectation is pinned nowhere in the repository. This is a
  Phase 5+ modelling question, recorded here because it bears on how a reader interprets the
  `period` cadence in the table above. Not a provenance defect.

---

## 4. The project-grade code constants

`.planning/REQUIREMENTS.md` CORE-10 scopes "parameter" to *simulation and economic* parameters and
carves out the non-economic numerical-method constants, on the condition that each is
*"recorded with a `GRADE: PROJECT` entry in `config/PROVENANCE.md` stating why they are not
configuration"*. **This section is that record** — it is the clause the amended CORE-10 points at.

These three constants live in `src/numeric.rs` as `const` items and are deliberately **not**
configuration keys. They therefore have no row in section 2, and `tests/provenance.rs` does not
expect one.

| Constant | Value | Grade | Why it is code, not configuration |
|---|---|---|---|
| `POW_FRAC_BITS` | 40 | GRADE: PROJECT | Bits of the fractional exponent consumed by `pow_frac_det`. It is a numerical-method iteration count, not an economic quantity: putting it in an economics config invites someone to tune it, and tuning it silently changes every trajectory. 40 bits gives a worst relative error of about 2e-12 against the standard library's power routine, far below any economically meaningful resolution. |
| `PPM_SCALE` | 1000000 | GRADE: PROJECT | The parts-per-million scale on which every probability and ratio enters the model as an integer. It is a *representation*, not a parameter — changing it would not express a different economy, it would silently rescale every threshold key in the config at once. |
| `MILLI_SCALE` | 1000 | GRADE: PROJECT | The thousandths scale, the model's second integer rate scale. Same argument as `PPM_SCALE`. |

**The caveat that keeps this honest.** `POW_FRAC_BITS` is nonetheless a **committed constant whose
change alters every run exactly as an economic parameter would** — every golden run and every
snapshot would have to be regenerated. Being code rather than config does not make it free to
change; it makes changing it a deliberate, reviewable, source-level act rather than a config tweak.
That is the whole reason for the carve-out, and stating it plainly is the price of taking it.

`PPM_SCALE` and `MILLI_SCALE` are weaker cases and rest on the representation argument above
(`01-CONTEXT.md` D-14): a scale factor is the unit a parameter is written in, not the parameter.

---

## Provenance of this document

Every grade, source and cadence above was transcribed **in session** by reading the graded table at
`.planning/research/SUMMARY.md:171-209` and the vocabulary at `.planning/research/SUMMARY.md:169`.
None was recalled from model memory, and no attributed value was written, corrected or upgraded
from memory (`01-CONTEXT.md` D-20; threat T-1-26 in `01-08-PLAN.md`).
