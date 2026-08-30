# Phase 1: Primitives and the Determinism Spine - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-30
**Phase:** 1-primitives-and-the-determinism-spine
**Areas discussed:** RNG sub-stream keying, expected_demand representation, the powf ban,
Money overflow API, numerical constants location, CORE-03 restatement, CORE-11 verification,
HashMap ban

---

## How this discussion went

Unusually, `01-RESEARCH.md` already existed when this discussion ran (research preceded
discussion). Its §"Decisions a `/gsd-discuss-phase` should settle before planning locks"
enumerated seven open questions with measured recommendations, so the gray areas below were
taken from research rather than generated independently — and the options presented carried the
research's own measurements.

The user answered **"[No preference]"** to both gray-area selection rounds, then to a narrowed
follow-up answered **"Im unfamiliar with the field you will have to decide."**

All eight decisions were therefore made by Claude under explicit delegation. No option below was
selected by the user.

---

## Round 1 — Architectural decisions (multiSelect)

| Option | Description | Selected |
|--------|-------------|----------|
| RNG sub-stream keying | Bit-packed set_stream nonce vs hashed child seeds, and the bit allocation. Measured 237ms vs 1390ms; bijective vs probabilistic. | |
| expected_demand: f64 or i64 | Restricted f64 vs i64 milli-units. Measured divergence 2.1e-3 units over 3,650 ticks. | |
| The powf ban's escape | Phase 7 needs (m/P̄)^0.9 but Phase 1 bans f64::powf. pow_frac_det vs changing α vs narrowing the ban. | |
| Money overflow: panic or Result | ROADMAP says panics; CLAUDE.md implies a MoneyOverflow error type. | |

**User's choice:** [No preference]
**Notes:** No areas selected; no preference expressed either way.

---

## Round 2 — Requirement-level tensions (multiSelect)

| Option | Description | Selected |
|--------|-------------|----------|
| CORE-03 is unpassable | SmallRng cannot be removed from rand 0.10.2's graph; gate cannot pass as written. | |
| CORE-11 paper verification blocked | Lengnick Table 1 egress-blocked on all six hosts, second research pass. | |
| Numerical constants: config or code | CORE-10 says no hardcoded parameter; pow_frac_det bits are not economics. | |
| HashMap: ban outright or wrapper | CLAUDE.md permits lookup-only; the clippy escape hatch is delicate. | |

**User's choice:** [No preference]
**Notes:** No areas selected.

---

## Round 3 — Narrowed to the two items Claude could not decide unilaterally

Presented on the reasoning that CORE-11 requires an out-of-band human action, and amending
REQUIREMENTS.md edits the user's own contract.

### CORE-11 handling

| Option | Description | Selected |
|--------|-------------|----------|
| Human-verify checkpoint (recommended) | Phase 1 blocks on a human opening the JEBO PDF and marking all 18 rows. | |
| Ship on grade B, verify before Phase 6 | Phase 1 builds the machinery; verification gates Phase 6, the first consumer. | ✓ (Claude) |
| Ship on grade B, record and move on | Accept the replication values; treat discrepancies as Phase 11 calibration findings. | |

**User's choice:** [No preference]
**Claude's decision and why:** Option 2. No phase before 6 consumes the values, so blocking the
project's *first* phase on an out-of-band human action buys no risk reduction and risks a stall
— particularly given the user has said they are unfamiliar with the domain. Option 3 was
rejected because it leaves the model's widest-sensitivity parameter region unverified through
six phases with no forcing function. Recorded as D-19.

### Requirement amendments

| Option | Description | Selected |
|--------|-------------|----------|
| Amend REQUIREMENTS.md in Phase 1 (recommended) | Rewrite CORE-03 into two testable clauses; carve non-economic constants out of CORE-10. | ✓ (Claude) |
| Planner note only, leave reqs untouched | CONTEXT.md carries the reinterpretation; requirements stay verbatim. | |
| Amend CORE-03, keep CORE-10 strict | Fix the unpassable one; put pow_frac_det bits into TOML as GRADE: PROJECT parameters. | |

**User's choice:** "Im unfamiliar with the field you will have to decide"
**Claude's decision and why:** Option 1. CORE-03 is not passable as written — verified from
crate source that `SmallRng` cannot be removed from `rand` 0.10.2 — so leaving it verbatim
means a phase gate that reads as failing forever. Option 3 was rejected because putting a
numerical-method constant (an iteration bit count) into an economics config invites tuning it,
and tuning it silently changes every trajectory. Recorded as D-17 and D-18.

---

## Claude's Discretion

The user delegated the entire phase. Beyond D-01…D-26 in CONTEXT.md, the planner has full
discretion on anything not fixed by CLAUDE.md's Technology Stack section or CORE-01…CORE-11.

---

## Deferred Ideas

- Lengnick Table 1 verification → blocking gate on Phase 6 (not dropped).
- A `Lookup` wrapper for lookup-only `HashMap` use → only if a later phase demonstrably needs it.
- `ChaCha12Rng` / `ChaCha20Rng` → the mitigation if ChaCha8 stream independence is ever doubted.
- i686 target reconsideration of the `f64` decision → only if a 32-bit x86 target appears.
