---
phase: 01-primitives-and-the-determinism-spine
plan: 02
subsystem: planning-contract
tags: [requirements, roadmap, rand-0.10, determinism, provenance, gate-integrity]

# Dependency graph
requires:
  - phase: 01-primitives-and-the-determinism-spine
    provides: "01-RESEARCH.md Pitfall 1 (crate-source + compile evidence that SmallRng is unconditional in rand 0.10.2) and 01-CONTEXT.md D-14/D-17/D-18/D-19/D-20"
provides:
  - "CORE-03 restated as two separately testable clauses: (a) StdRng/SysRng absent from the dependency graph via the rand feature set; (b) SmallRng/Xoshiro never used, via clippy disallowed-types plus a source grep"
  - "CORE-10 scoped to simulation and economic parameters, with the numerical-method constants carve-out named and pointed at config/PROVENANCE.md"
  - "CORE-11 split into a Phase-1 annotation clause and a Phase-6-gated paper-verification clause, with the deferral recorded in the requirement itself rather than only in the decision log"
  - "Phase 1 Success Criterion 5 restated to match the amended CORE-11, and a new Phase 6 Success Criterion 6 that gives the deferred gate a phase that will actually run it"
  - "Corrected rand 0.10 SmallRng claim in .claude/CLAUDE.md, so the technology contract and the requirements agree"
affects: [01-04 (RNG sub-streams), 01-06 (config loader), 01-07 (lint wall), 01-08 (provenance machinery), Phase 6 (labour market — carries the CORE-11 clause (b) gate)]

actuals:
  tokens: 3020
  tasks: 4
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Amendment-with-inline-rationale: every restated requirement carries an indented *Rationale* line naming its authority (a CONTEXT.md decision) and its evidence (a RESEARCH.md section), committed in the same diff as the change"
    - "Two-clause requirement form: a gate that mixes an achievable claim with an unachievable one is split into separately lettered, separately testable clauses, each naming its own enforcement mechanism"

key-files:
  created:
    - .planning/phases/01-primitives-and-the-determinism-spine/01-02-SUMMARY.md
  modified:
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - .claude/CLAUDE.md

key-decisions:
  - "CORE-03's absence claim is narrowed to StdRng and SysRng (the two the feature set genuinely removes) and SmallRng moves to a never-used clause — absence is unachievable without forking rand, and clause (b) bans use, which is what the gate was always for"
  - "CORE-10's carve-out names the three constants explicitly (POW_FRAC_BITS, PPM_SCALE, MILLI_SCALE) rather than describing a category, so a later reader cannot widen the exception by interpretation"
  - "CORE-11 stays owned by Phase 1 in the traceability table; only the gate on clause (b) moves. Splitting the row across two phases would break the file's stated one-requirement-one-phase invariant and the coverage arithmetic beneath it"
  - "The Phase 6 gate is added as a new criterion 6 rather than folded into Phase 6's existing criterion 5, preserving the file's six `5.` criterion lines as a structural check against an accidental restructure"

patterns-established:
  - "Gate moves are visible, never quiet: a deferred clause is restated in both the requirement and the roadmap criterion that grades it, each citing the same decision ID, and the receiving phase gains the gate in the same diff"
  - "Structural counts (87 requirement rows, 11 phase headings, 6 criterion-5 lines) are asserted before and after every scoped edit to shared planning artifacts, so an accidental whole-file rewrite is caught mechanically"

requirements-completed: [CORE-03, CORE-10, CORE-11]

coverage:
  - id: D1
    description: "CORE-03 restated as two separately testable clauses, with the unpassable original absence claim about SmallRng removed rather than supplemented"
    requirement: "CORE-03"
    verification:
      - kind: other
        ref: "grep -cE 'SmallRng. are absent from the dependency graph' .planning/REQUIREMENTS.md == 0 (was 1)"
        status: pass
      - kind: other
        ref: "grep -c '(b)' .planning/REQUIREMENTS.md == 2 (was 0)"
        status: pass
      - kind: other
        ref: "grep -A6 'CORE-03' .planning/REQUIREMENTS.md | grep -ci 'rationale' == 1 (was 0)"
        status: pass
    human_judgment: false
  - id: D2
    description: "CORE-10 scoped to simulation/economic parameters with the numerical-method carve-out named and its grade home stated"
    requirement: "CORE-10"
    verification:
      - kind: other
        ref: "grep -A6 'CORE-10' .planning/REQUIREMENTS.md | grep -qi 'rationale'"
        status: pass
      - kind: other
        ref: "grep -q 'PROVENANCE.md' .planning/REQUIREMENTS.md"
        status: pass
    human_judgment: false
  - id: D3
    description: "rand 0.10 SmallRng claim corrected in both CLAUDE.md tables, each corrected cell citing 01-RESEARCH.md Pitfall 1"
    verification:
      - kind: other
        ref: "grep -c 'Removed / not portable' ./.claude/CLAUDE.md == 0 (was 1)"
        status: pass
      - kind: other
        ref: "grep -c 'was removed in 0.10 entirely' ./.claude/CLAUDE.md == 0 (was 1)"
        status: pass
      - kind: other
        ref: "grep -c 'std_rng' ./.claude/CLAUDE.md == 2 (was 1); grep -qi 'Pitfall 1' passes"
        status: pass
      - kind: other
        ref: "git diff --stat .claude/CLAUDE.md == 4 changed lines (bound: <30)"
        status: pass
    human_judgment: false
  - id: D4
    description: "CORE-11 and Phase 1 Success Criterion 5 split so the deferred paper-verification clause is gated on Phase 6, with Phase 6 gaining the gate"
    requirement: "CORE-11"
    verification:
      - kind: other
        ref: "grep -c 'D-19' .planning/REQUIREMENTS.md == 1 and .planning/ROADMAP.md == 2 (both were 0)"
        status: pass
      - kind: other
        ref: "grep -A8 'CORE-11' .planning/REQUIREMENTS.md | grep -ci 'rationale' == 1"
        status: pass
      - kind: other
        ref: "grep -c '^### Phase ' .planning/ROADMAP.md == 11 and grep -c '^  5\\. ' == 6, both unchanged"
        status: pass
      - kind: other
        ref: "git diff --stat .planning/ROADMAP.md == 3 changed lines (bound: <20)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The three amendments are an auditable restatement of the user's own contract, not a quiet loosening of gates"
    verification: []
    human_judgment: true
    rationale: "Whether a restated gate still means what the user intended is a judgment about intent, not a property a grep can assert. The mechanical checks prove each amendment shipped with a rationale citing its authority and evidence; they cannot prove the restatement is faithful. The user delegated these three amendments explicitly (D-17, D-18, D-19), so the sign-off is on the wording, not on whether to amend."

duration: 4 min
completed: 2026-08-30
status: complete
---

# Phase 01 Plan 02: Requirement Amendments and the rand 0.10 Correction Summary

**Three Phase-1 gates that could not pass as written — one unachievable, one over-broad, one graded against work no plan in this phase does — restated as separately testable clauses, each shipping with its rationale and evidence in the same commit, plus the crate-source correction to CLAUDE.md that produced the first of them.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-08-30T23:12:12Z
- **Completed:** 2026-08-30T23:16:40Z
- **Tasks:** 4 of 4
- **Files modified:** 3

## Accomplishments

- **CORE-03 is now passable.** The original demanded `StdRng` *and* `SmallRng` be absent from the dependency graph. `rand` 0.10.2 makes `SmallRng` unconditional (`rngs/mod.rs:97-108` re-exports it and the Xoshiro generators with no `cfg` guard; only `StdRng` sits behind `#[cfg(feature = "std_rng")]`), so absence is unachievable without forking the crate. Clause (a) now names only the two generators the feature set genuinely removes; clause (b) bans `SmallRng`/Xoshiro by *use*, via clippy `disallowed-types` plus a source grep. The unpassable wording is gone from the file, not merely supplemented.
- **CORE-10 now says which values must be configuration and which must not.** Scope is stated as every *simulation or economic* parameter; `POW_FRAC_BITS`, `PPM_SCALE` and `MILLI_SCALE` are named as `const` items in `src/numeric.rs` with their justification recorded as a `GRADE: PROJECT` entry in `config/PROVENANCE.md`. Both strictness clauses that carry the requirement's weight — `deny_unknown_fields`, no serde defaults — are untouched.
- **CORE-11's deferral is now visible in the files that grade it, not only in the decision log.** Clause (a) (source-grade annotation, enforced by an automated test) is Phase 1; clause (b) (paper verification by a person with journal access) is a blocking gate on Phase 6, with the affected rows standing `UNVERIFIED` until then. Phase 1 Success Criterion 5 was restated to match — keeping the paper-verification language rather than deleting it — and **Phase 6 gained a criterion 6**, so the deferred gate has a phase that will actually run it.
- **The technology contract now states what was verified from crate source.** Both CLAUDE.md rows asserting `SmallRng` was removed in rand 0.10 are corrected: the *feature* went, the *type* became unconditional. Each corrected cell cites `01-RESEARCH.md` Pitfall 1, so the correction carries its evidence rather than swapping one assertion for another.

## Task Commits

Each task was committed atomically:

1. **Task 1: Amend CORE-03 into two separately testable clauses** — `7508d1c` (docs)
2. **Task 2: Scope CORE-10 to simulation parameters and name the carve-out** — `2d6dc06` (docs)
3. **Task 3: Correct the rand 0.10 small-RNG claim in the technology contract** — `95a2f51` (docs)
4. **Task 4: Split CORE-11 and Phase 1 criterion 5 so the deferred clause is visibly gated** — `c196193` (docs)

_Note: `925ee2d` sits between commits 3 and 4 in the log. It is the orchestrator's own `.planning/config.json` change (`use_worktrees: false`), not part of this plan._

## Files Created/Modified

- `.planning/REQUIREMENTS.md` — CORE-03, CORE-10 and CORE-11 restated, each followed by an indented `*Rationale*` line naming its authority (a CONTEXT.md decision) and its evidence. 87 requirement rows and the `v1 requirements: 87 total` line unchanged; all three traceability rows still map to Phase 1.
- `.planning/ROADMAP.md` — Phase 1 Success Criterion 5 restated to match the amended CORE-11 and to name the enforcement that makes its first half checkable; Phase 6 Success Criteria gained criterion 6 carrying the deferred gate. 11 phase headings and 6 `5.` criterion lines unchanged.
- `.claude/CLAUDE.md` — the rand 0.10 breakage-table verdict cell and the "What NOT to Use" `SmallRng` row replaced outright (4 changed lines).

## Decisions Made

- **CORE-11 stays mapped to Phase 1 in the traceability table.** The file states every v1 requirement maps to exactly one phase; splitting the row across Phase 1 and Phase 6 would break that invariant and the coverage arithmetic beneath it. What moves is the gate on clause (b), and the amended bullet is where that move is recorded — which is exactly what the plan's prohibition asks for.
- **The Phase 6 gate is a new criterion 6, not an edit to Phase 6's criterion 5.** This preserves the six `5.` criterion lines that the plan uses as a structural tripwire against an accidental restructure, and keeps the gate legible as an addition rather than a rewrite of an existing labour-market criterion.
- **Both corrected CLAUDE.md cells were replaced outright rather than appended to.** An appended correction leaves the disproved sentence in the file, where a future reader (or agent) can still act on it — and the plan's verify commands assert both original strings are absent, so appending would have failed the task.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Stale baseline in an acceptance criterion] `grep -c '^- \[ \] \*\*'` reports 84, not the 87 the plan asserts**

- **Found during:** Task 1 (repeated in Task 4, which asserts the same number)
- **Issue:** Tasks 1 and 4 both assert that `grep -c '^- \[ \] \*\*' .planning/REQUIREMENTS.md` reports 87 before and after the edit, as a tripwire against a whole-file rewrite destroying requirement rows. Measured at execution time it reports **84**. The pattern counts only *unchecked* rows (`- [ ] **`), and plan 01-01 — which ran earlier in this same wave — checked off CORE-02, CORE-08 and CORE-09 to `- [x] **`. The plan's baseline was measured before that happened. Taken literally the criterion could never pass, and "fixing" the file to make it report 87 would mean un-checking three genuinely-completed requirements.
- **Fix:** Kept the tripwire's intent — no requirement row added or destroyed — and asserted it on both the stated pattern and a checkbox-agnostic one: unchecked rows stayed at **84** across every edit, and total rows (`^- \[[ x]\] \*\*`) stayed at **87**, matching the plan's intended number and the `v1 requirements: 87 total` line, which still reports 1. No file content was changed to satisfy the literal criterion.
- **Files modified:** None (verification-only adjustment)
- **Verification:** `grep -c '^- \[ \] \*\*'` = 84 and `grep -c '^- \[[ x]\] \*\*'` = 87 before Task 1, after each of the four commits, and at final verification. `grep -c 'v1 requirements: 87 total'` = 1 throughout.
- **Committed in:** No content change; recorded here and in `.planning/WINDOWS.md`.

---

**Total deviations:** 1 auto-fixed (1 × Rule 1 — stale measurement in an acceptance criterion, no code or content defect).
**Impact on plan:** None on scope or output. All four tasks landed exactly as written; the adjustment affects only how one structural tripwire is measured, and the tripwire's actual guarantee (no requirement row added or destroyed) was verified more strictly than the plan asked, on both patterns.

## Issues Encountered

None. All four tasks' `<verify>` commands and every acceptance criterion passed on the first attempt; no fix cycles were needed.

## Findings Passed Forward (informational — not acted on)

- **The prior-wave CORE-09 note from plan 01-01 was not in this plan's scope and was deliberately not acted on.** 01-01 reported that the phase's CORE-09 acceptance form (`cargo tree | grep -c getrandom` equals 0) cannot pass, because `proptest` and `tempfile` pull `getrandom` as dev-dependencies, and that it narrowed its own guard to `cargo tree --edges normal`. That is the same *class* of defect this plan repairs — an unpassable gate — but CORE-09 is not among this plan's `requirements` and its text is not in `files_modified`. Amending it here would have widened scope on the strength of a note. **Recorded for the phase verifier:** CORE-09's stated form still carries the unsatisfiable broad wording, and the same amendment-with-rationale treatment applied to CORE-03 here would resolve it.
- **`.planning/config.json` was modified in the working tree at execution start** (`use_worktrees: false`, orchestrator-owned) and was committed by the orchestrator as `925ee2d` mid-run. Correctly left untouched — it is outside this plan's `files_modified`.

## Threat Flags

None. All three edits are to planning and instruction artifacts; no network endpoint, auth path, file-access pattern or schema at a trust boundary was introduced. The plan's own threat register (T-1-07, T-1-29, T-1-08, T-1-09) was mitigated as specified: every amendment ships an inline rationale in the same diff; CORE-11's deferral cites D-19 in both files and adds the gate to Phase 6; the CLAUDE.md corrections cite reproducible crate-source and compiler evidence; and every edit was a scoped replacement whose diff size and structural counts were asserted (4 lines in CLAUDE.md against a 30-line bound, 3 in ROADMAP.md against a 20-line bound, 87 rows / 11 headings / 6 criterion lines all unchanged).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Wave 1 is complete; Wave 2 (`01-03`, `01-04`, `01-05`, `01-06`) is unblocked.

The three amended requirements are now stated in a form their implementing plans can actually test:

- **`01-04`** implements CORE-03's two clauses — clause (a) is a compile-failure assertion, clause (b) is a `clippy.toml` `disallowed-types` entry plus a source grep. Both mechanisms are now named in the requirement itself.
- **`01-06`** implements CORE-10 against a scope that no longer forces `POW_FRAC_BITS` into the TOML.
- **`01-07`** builds the lint wall that enforces CORE-03 clause (b).
- **`01-08`** implements CORE-11 clause (a) and ships the `UNVERIFIED` marking plus the verification procedure that Phase 6's new criterion 6 will execute.

**Carried forward:** CORE-03, CORE-10 and CORE-11 remain `Pending` in the traceability table — correctly. This plan amended their text; the plans above own their predicates, and each is declared by a sibling plan that has not yet produced a SUMMARY, so the shared-ID gate holds all three from being marked Complete.

**Open concern for the verifier:** CORE-09's acceptance form is still stated unpassably (see Findings Passed Forward). It is the one remaining instance in this phase of the defect class this plan was created to fix.

## Self-Check: PASSED

- `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.claude/CLAUDE.md` — all FOUND on disk.
- `.planning/phases/01-primitives-and-the-determinism-spine/01-02-SUMMARY.md` — created.
- Commits `7508d1c`, `2d6dc06`, `95a2f51`, `c196193` — all FOUND in `git log --oneline --all`.
- `git diff --diff-filter=D --name-only 7508d1c~1 c196193` — empty; no file deleted.
- My four commits touch only the three files in `files_modified`.
- Plan-level `<verification>` block re-run at completion: 87 requirement rows; CORE-03/10/11 all still mapped to Phase 1; each amended bullet immediately followed by a `*Rationale*` line; both disproved CLAUDE.md claims absent with `std_rng` at 2 and `Pitfall 1` cited twice; `D-19` present in both REQUIREMENTS.md and ROADMAP.md; 11 phase headings intact.
- No stubs, no skipped tests, no unrun `<verify>` commands.

---
*Phase: 01-primitives-and-the-determinism-spine*
*Completed: 2026-08-30*
