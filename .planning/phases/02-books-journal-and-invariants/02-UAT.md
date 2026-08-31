---
status: complete
phase: 02-books-journal-and-invariants
source: [02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md, 02-04-SUMMARY.md, 02-05-SUMMARY.md, 02-06-SUMMARY.md, 02-07-SUMMARY.md]
started: 2026-08-31T12:00:00Z
updated: 2026-08-31T12:00:00Z
---

## Current Test

[testing complete]

## Tests

All 47 deliverables across the phase's seven summaries classified `mode: coverage` with
`all_auto_covered: true` and **zero `present[]` entries** — that is, `uat.classify-coverage`
found no deliverable requiring human judgment. Recorded here as automated passes rather than
presented as checkpoints, per the coverage-aware classification rule.

This is a pure-infrastructure phase: a Rust library with no UI, no network, no external
service and no user-observable behaviour. There is nothing a person can exercise by hand that
the suite does not already exercise deterministically.

### 1. Plan 02-01 — liveness config gate, ledger address rendering, source amendments
expected: The `[invariants] liveness_enabled` leaf lands across all five parts of the config
agreement; every ledger address renders through `Display` so a halt can name the offending
agent; ROADMAP criterion 2 and LEDG-09 describe a linear scan, and the cross-phase obligations
have owners.
result: pass
source: automated
coverage_id: 02-01 (4 deliverables)

### 2. Plan 02-02 — TRACER: Books, journal, CheckSet
expected: A tick loop halts on a tick that traded nothing, returning
`Violation::Liveness { tick: 4, counted: 0, required: 1 }` by whole-value equality, and is
proved never to have begun tick 5; with the gate off the identical loop runs all ten ticks.
result: pass
source: automated
coverage_id: 02-02 (6 deliverables)

### 3. Plan 02-03 — goods, exchange, the one-shape identity
expected: The books own every unit; a cash-for-units swap is one posting and cannot be
half-applied; `produced − consumed − Σ stock == 0` holds every tick, checked against the
residual accumulated from the posting legs.
result: pass
source: automated
coverage_id: 02-03 (6 deliverables)

### 4. Plan 02-04 — non-negativity, zero-sum, headcount, check order
expected: Five checks in a contracted order, the sequence asserted from `ALL_CHECKS` with an
exhaustive match turning a missing table entry into a compile error.
result: pass
source: automated
coverage_id: 02-04 (5 deliverables)

### 5. Plan 02-05 — the negative tests
expected: Four seeded corruptions each halt the run with the right variant; localisation names
the FIRST non-conserving posting across a cancelling residual (broken 50 / healed 120 / broken
200 → answer 50); zero production surface.
result: pass
source: automated
coverage_id: 02-05 (10 deliverables)

### 6. Plan 02-06 — LEDG-02's four legs
expected: E0502 borrow probe asserting the diagnostic code, no-callback signature rule, no
interior mutability, and panic-atomicity proved by a positive test plus a mutant that fails it
(−400 against an opening 100); eight fixture-first source guards each watched firing before
being trusted silent.
result: pass
source: automated
coverage_id: 02-06 (9 deliverables)

### 7. Plan 02-07 — properties
expected: Conservation under arbitrary posting sequences, transfer-return agreement, the goods
identity, and two-source agreement — with generators that hit non-dividing amounts deliberately.
result: pass
source: automated
coverage_id: 02-07 (7 deliverables)

### 8. Code review findings and the verification gap
expected: The blocker (a zero-cent `transfer` bypassing liveness) closed at both the boundary
and the check; the eight warnings resolved or explicitly justified; and the single verification
gap — `check_goods` never observed to fire — closed with two negative tests, one per arm.
result: pass
source: automated
coverage_id: post-execution gates

## Summary

total: 8
passed: 8
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

None open. The one blocking gap found by verification (`check_goods` never observed to fire)
was closed by `e0ee1b4` and independently re-verified by the orchestrator: re-applying the
exact mutation to a clean `git archive HEAD` copy failed the two new tests (164 passed, 2
failed) where before the closure it left all 239 green.

## Notes

**Why this phase has zero human checkpoints and Phase 1 had four.** Phase 1's UAT items were
genuine judgment calls — a field-spelling choice and the V-3a source contradiction, neither
decidable from the code. This phase produced no such item: every claim it makes is a property
of the ledger that a test either does or does not demonstrate.

**What the suite could not tell us.** Five defects of one shape were found here — an assertion
whose stated claim is not what it measures — and the 242-test suite was green through every one
of them. All five were caught by mutation. A UAT that read this suite as evidence would have
passed the phase five times over while the invariants sat inert.
