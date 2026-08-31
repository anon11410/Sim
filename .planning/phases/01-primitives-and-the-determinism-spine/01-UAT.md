---
status: testing
phase: 01-primitives-and-the-determinism-spine
source: [01-VERIFICATION.md]
started: 2026-08-31T00:00:00Z
updated: 2026-08-31T00:00:00Z
---

## Current Test

number: 1
name: CORE-06 field spelling — accept `generation` or amend the requirement text
expected: |
  A decision, not a code change. The generational field is spelled `generation`
  because `gen` is a reserved keyword in Rust edition 2024 and does not parse as
  an identifier (verified by compile error). CORE-06, 01-RESEARCH.md Pattern 5
  and decision D-03 all say `gen`.

  The spelling is forced by the language, so the only question is which record
  moves. Either amend CORE-06 / Pattern 5 / D-03 to say `generation`, or accept
  the divergence and note it.

  Settle this BEFORE Phase 3. `(slot, generation)` becomes the firm's identity
  in the log schema there, and the field name is baked into every logged row
  from that point on.
awaiting: user response

## Tests

### 1. CORE-06 field spelling — accept `generation` or amend the requirement text
expected: The spelling is forced (reserved keyword in edition 2024). Decide whether to amend CORE-06 / RESEARCH Pattern 5 / D-03 to match the code, or record the divergence deliberately. Must be settled before Phase 3 writes the log schema.
result: [pending]

### 2. V-3a — decide the tracking route for `entrant_size_ratio_ppm` (not the value)
expected: |
  The key ships 800000 (0.8x) while its own SOURCE field cites
  size-replacing-firms = 0.2. Three readings, not equivalent: transcription
  error; a deliberate 1 - 0.2 derivation (which would regrade the row B to C);
  or a misread source parameter.

  You are NOT being asked to pick the value — D-20 forbids resolving a
  parameter's meaning from memory, and that prohibition held here under
  pressure. You are being asked where the check gets forced, because right now
  nothing forces it: Phase 6 SC6 scopes to "Lengnick Table 1 rows" and this is
  a BAM row, while ROADMAP Phase 10 SC5 already asserts 0.8x as settled fact.
  It must be decided before BANK-04 consumes it.
result: [pending]

### 3. Confirm the CI workflow executes green on GitHub Actions
expected: |
  `.github/workflows/ci.yml` has never run on a real runner. All seven steps
  were executed locally in order and each exits 0, and the YAML parses, but the
  runner image is unwitnessed.

  The specific named risk: rustup 1.28+ no longer auto-installs a toolchain on
  `rustup show active-toolchain`, so `rustup component add` could error on an
  uninstalled toolchain. Push the branch and confirm the run is green.
result: [pending]

### 4. Backstop truth — `overflow-checks` applies uniformly to every arithmetic site
expected: |
  The verifier ABSTAINED on this one rather than passing it, citing
  insufficient_spec, and the reasoning is worth preserving: a single
  `black_box`ed test site is evidence about that site, not evidence for a
  universal quantifier over every integer arithmetic site compiled under the
  release profile.

  Decide whether to accept it as a backstop truth on the strength of the
  documented rustc profile semantics, or to specify a narrower claim that can
  actually be tested.
result: [pending]

## Summary

total: 4
passed: 0
issues: 0
pending: 4
skipped: 0
blocked: 0

## Gaps

None. The verifier reported 18/20 must-haves verified with no FAILED items and
no gaps — 1 present-but-behavior-unverified (the CI runner, item 3) and 1
deliberate abstention (item 4).

## Carry-forward warnings (not blocking, recorded so they are not rediscovered)

- **W1** — V-3a was tracked only in `config/PROVENANCE.md` while V-4 had been
  escalated to STATE.md, and `deferred-items.md` claimed no items remained open.
  Both corrected during phase close-out.
- **W2** — no guard detects drift in the generated `clippy.toml` on a toolchain
  pin bump, and `rust-src` is not listed in `rust-toolchain.toml` components.
- **W3** — nothing guards `config/PROVENANCE.md` section 4; deleting it fails
  no test, so the CORE-10 carve-out clause could silently reopen.
- **W4** — the RNG re-entry guard is debug-only by design (it is the entire
  139-vs-137 test delta) and starts to matter from Phase 3, when the tick loop
  opens sub-stream keys at volume.
- **W5** — plan 01-01's prose "`cargo tree` shows no OS-entropy crate" is looser
  than its own guard, which correctly scopes to `--edges normal`. Fix the
  sentence if anything; do NOT "fix" the guard to match the sentence.
