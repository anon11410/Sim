---
status: complete
phase: 01-primitives-and-the-determinism-spine
source: [01-VERIFICATION.md]
started: 2026-08-31T00:00:00Z
updated: 2026-08-31T07:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. CORE-06 field spelling — accept `generation` or amend the requirement text
expected: The spelling is forced (reserved keyword in edition 2024). Decide whether to amend CORE-06 / RESEARCH Pattern 5 / D-03 to match the code, or record the divergence deliberately. Must be settled before Phase 3 writes the log schema.
result: pass
decision: |
  Amend the requirement text; the code keeps `generation`. The spelling is not
  a choice — `gen` is a reserved keyword in edition 2024 and does not parse as
  an identifier — so the only question was which record moves, and leaving the
  requirement saying `gen` would have had the Phase 3 planner write `gen` into
  the log schema and hit the same compile error, or reach for `r#gen` and carry
  the escape through ten phases.

  Applied:
  - `REQUIREMENTS.md` CORE-06 now reads `FirmId { slot, generation }`, with a
    dated rationale in the same style as the CORE-10 amendment. It records that
    the type shape, derived total order and `Option` accessors are unchanged,
    that Phase 3 bakes `(slot, generation)` into every logged row, and that
    D-03 is untouched (the RNG key's `agent` field still carries the slot only).
  - `ROADMAP.md` Phase 10 criterion 2 aligned: `(slot, generation)` log groups,
    respawn in place at `generation+1`.
  - `01-CONTEXT.md` D-03 and `01-RESEARCH.md` Pattern 5 annotated rather than
    rewritten — they are dated records of what the phase knew, so each carries a
    note that the field ships as `generation` and points at the CORE-06
    rationale. Their original text stands.

### 2. V-3a — decide the tracking route for `entrant_size_ratio_ppm` (not the value)
expected: Decide where the check gets forced, not what the value is. Nothing currently forces it — Phase 6 SC6 scopes to Lengnick Table 1 rows and this is a BAM row, while ROADMAP Phase 10 SC5 asserted 0.8x as settled fact.
result: pass
decision: |
  Gate it at the consumer, and remove the assertion that contradicted it. The
  value itself is NOT decided here — D-20 forbids resolving it from model
  memory, and that prohibition is the reason V-3a exists.

  The real defect was that ROADMAP Phase 10 SC5 asserted "sized at 0.8x" as
  settled fact — the roadmap was stating as true the exact thing V-3a holds
  open, so the phase could have passed its own criterion without anyone reading
  BAM.

  Applied:
  - Phase 10 criterion 5 rewritten to reference `bankruptcy.entrant_size_ratio_ppm`
    (the config key) instead of the literal 0.8x, so the criterion cannot be
    satisfied by asserting an unchecked number.
  - Phase 10 criterion 6 added as a blocking gate: V-3a adjudicated before
    BANK-04 consumes the value, with all three readings enumerated and the D-20
    prohibition restated.
  - Phase 6 criterion 6 cross-referenced: if BAM is to hand on the journal pass,
    check V-3 and V-3a then — that is the phase where someone has access, even
    though the blocking gate sits in Phase 10.
  - `STATE.md` V-3a entry updated to record that the gate now exists, and that
    the gate forces the check rather than answering it.

### 3. Confirm the CI workflow executes green on GitHub Actions
expected: Push the branch and confirm all seven steps green on a real runner, watching step 1 — rustup 1.28+ no longer auto-installs the toolchain named by rust-toolchain.toml, which could make `rustup component add` error on an uninstalled toolchain.
result: pass
evidence: |
  Settled empirically, not by judgment. Five CI runs on `ubuntu-latest`, all
  `conclusion: success` (runs 1-5, the latest being 33365487139 at commit
  dabe6ba). The named risk did not materialise: step 3, "Install the pinned
  toolchain", completed in 8 seconds and every subsequent step used the
  resolved toolchain.

  All ten job steps green, including the four that carry the phase's guarantees:
  Test (release), Lint (the determinism bans), the CORE-09 reproducibility
  guard, and the CORE-07 / CORE-03(b) lint-gate guard — the last reporting
  "all 60 resolvable method bans fired" and "no alias, exemption or
  non-portable generator escapes it" on the runner rather than only locally.

  https://github.com/anon11410/Sim/actions/runs/33365487139

### 4. Backstop truth — `overflow-checks` applies uniformly to every arithmetic site
expected: Either add a held-out release-profile test that overflows a raw i64 across an inlined call boundary and inside a generic, or downgrade the claim to the single-site statement the existing evidence supports.
result: pass
decision: |
  Added the held-out tests rather than narrowing the claim. The verifier's
  abstention was correct — one `black_box`ed site in a test function's own body
  is evidence about that site, not a universal quantifier — and the fix costs
  ~30 lines, so buying evidence beat weakening the claim.

  Two structurally distinct sites added to `tests/tracer_end_to_end.rs`, chosen
  because `-C overflow-checks` is applied when MIR is BUILT rather than when it
  is codegen'd, which is the mechanism by which a check could fail to follow a
  call site:
  - `raw_i64_overflow_panics_across_an_inline_boundary` — the addition is
    written in an `#[inline(always)]` fn and executed in the caller's frame.
  - `raw_i64_overflow_panics_inside_a_generic` — the addition is written once
    against `T: Add` and monomorphised at `i64` by the caller.
  Plus `the_held_out_sites_do_not_panic_one_step_below_the_edge`, the negative
  half that distinguishes "overflow is detected here" from "these sites panic
  on any addition".

  FALSIFIABILITY CHECKED, which is what makes this evidence rather than three
  more green ticks: with `overflow-checks` commented out of `[profile.release]`,
  the release run fails 3 of 8 — both new sites plus the original. Restored;
  `Cargo.toml` is byte-identical to before the check. Release suite now 8
  passed.

  Scope stated honestly in the test's own header comment: both sites are
  same-crate, so what they establish is that the profile setting reaches inlined
  and monomorphised MIR in THIS crate. That is the claim the project depends on
  — goods units, headcounts and tick counters are raw `i64` and will be added
  inside small helpers and generic code, not only in straight-line bodies. It is
  still not a proof over every site in the dependency graph, and the comment
  says so.

## Summary

total: 4
passed: 4
issues: 0
pending: 0
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
