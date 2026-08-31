---
phase: 02-books-journal-and-invariants
plan: 01
subsystem: infra
tags: [config, serde, toml, provenance, display, thiserror, roadmap, requirements]

# Dependency graph
requires:
  - phase: 01-primitives-and-the-determinism-spine
    provides: "`Params` with `deny_unknown_fields`, the three config-strictness tests, the `# GRADE:` annotation contract and `config/PROVENANCE.md`, and the five `src/ids.rs` address newtypes"
provides:
  - "`invariants.liveness_enabled` — a required, annotated, provenance-recorded boolean config key with no serde default (LEDG-08's switch)"
  - "`sim::config::Invariants` and `Params::invariants`, read once at check-set construction and never on the per-tick path"
  - "`Display` for `HouseholdId`, `FirmSlot`, `GoodId`, `FirmId` and `Account` — the rendered forms `household:12`, `firm-slot:3`, `good:0`, `firm:3:0`"
  - "ROADMAP Phase 2 criterion 2 and REQUIREMENTS LEDG-09 both describing localisation as a linear scan for the first non-conserving posting"
  - "ROADMAP Phase 3 criteria 6 and 7, and Phase 6 criterion 7 — owners for three previously unowned cross-phase obligations"
affects: [02-02, 02-03, 02-04, 02-05, 02-06, phase-03-tick-pipeline, phase-06-labour-market]

actuals:
  tokens: 35100
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A new config leaf is a four-part change — TOML key + two-line annotation, schema field, provenance row, schema-leaf agreement — landed in one commit because three existing tests watch the agreement"
    - "Address types render through `Display` so a `thiserror` format string can interpolate an agent inline; rendered forms are pinned by full-string equality, never `contains`"
    - "A superseded mechanism is corrected on the requirement's own line and explained in an indented `*Rationale (amended …)*` bullet — the superseded verb survives only inside the rationale"

key-files:
  created: []
  modified:
    - config/baseline.toml
    - src/config.rs
    - config/PROVENANCE.md
    - src/ids.rs
    - .planning/ROADMAP.md
    - .planning/REQUIREMENTS.md

key-decisions:
  - "`invariants.liveness_enabled` ships `false` and its flip to `true` is owned by ROADMAP Phase 6 criterion 7, not by a comment or a date"
  - "The firm rendered form carries the generation (`firm:3:0`), so a halt is unambiguous across a Phase 10 respawn"
  - "`Account`'s `Display` delegates to the inner identity rather than re-spelling it, leaving one place either address shape can drift"
  - "The superseded localisation verb is scrubbed entirely from ROADMAP (the phase gate reads it) but retained in REQUIREMENTS inside the rationale that explains its supersession"
  - "`config/PROVENANCE.md`'s prose key and grade counts were corrected 41 -> 42 and 15 -> 16 PROJECT rows; no test reads them, but a stale count is a defect in the document the verification procedure is run from"

patterns-established:
  - "Config leaf agreement: baseline.toml + config.rs + PROVENANCE.md + the schema-leaf test move together or three tests stay red"
  - "Rendered-address contract: integer identifiers only — no path, host name, wall-clock reading or process id (TICK-06)"

requirements-completed: [LEDG-08, LEDG-09, LEDG-10]

coverage:
  - id: D1
    description: "`invariants.liveness_enabled` is a required, annotated, provenance-recorded boolean config key with no serde default"
    requirement: "LEDG-08"
    verification:
      - kind: integration
        ref: "tests/config_strict.rs#every_key_is_required"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#the_schema_and_the_shipped_config_name_the_same_leaves"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#no_optional_fields_in_the_config_schema"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#no_serde_defaults_anywhere_in_src"
        status: pass
      - kind: integration
        ref: "tests/provenance.rs#no_annotation_is_orphaned"
        status: pass
      - kind: integration
        ref: "tests/provenance.rs#every_config_key_has_a_provenance_row"
        status: pass
      - kind: integration
        ref: "tests/provenance.rs#every_key_has_a_source_grade"
        status: pass
    human_judgment: false
  - id: D2
    description: "Five ledger address types render through `Display` in forms pinned by string-equality tests, with the generation carried in the firm form"
    requirement: "LEDG-09"
    verification:
      - kind: unit
        ref: "src/ids.rs#every_address_renders_in_its_pinned_form"
        status: pass
      - kind: unit
        ref: "src/ids.rs#two_generations_of_one_slot_render_differently"
        status: pass
      - kind: other
        ref: "cargo test --locked --release --lib ids"
        status: pass
      - kind: other
        ref: "grep -c 'impl std::fmt::Display' src/ids.rs == 5; grep -cE '\\bf16\\b|\\bf32\\b|\\bf64\\b|\\bf128\\b' src/ids.rs == 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "ROADMAP Phase 2 criterion 2 and REQUIREMENTS LEDG-09 describe localisation as a linear scan for the first non-conserving posting; LEDG-09 carries a rationale block in the CORE-03/CORE-06 shape with the measured counterexample"
    requirement: "LEDG-09"
    verification:
      - kind: other
        ref: "grep -ci 'bisect' .planning/ROADMAP.md == 0"
        status: pass
      - kind: other
        ref: "grep -cE '^- \\[.\\] \\*\\*LEDG-09\\*\\*.*bisect' .planning/REQUIREMENTS.md == 0 AND grep -cE '^- \\[.\\] \\*\\*LEDG-09\\*\\*.*linear scan' .planning/REQUIREMENTS.md == 1"
        status: pass
      - kind: other
        ref: "sed -n '/### Phase 2:/,/### Phase 3:/p' .planning/ROADMAP.md | grep -cE '^  2\\..*linear scan' == 1"
        status: pass
      - kind: other
        ref: "grep -cE '^  - \\*Rationale \\(amended 2026-08-31; authority: 02-RESEARCH\\.md; evidence: broken #50 / healed #120 / broken #200 — bisect answers 200, linear scan answers 50\\)\\.\\*' .planning/REQUIREMENTS.md == 1; grep -c 'Rationale (amended' == 5"
        status: pass
    human_judgment: false
  - id: D4
    description: "Three previously unowned cross-phase obligations have a named owning phase: the process-level halt and the `Household`/`Firm` balance-field obligation against Phase 3, and the flip of the shipped liveness value against Phase 6"
    requirement: "LEDG-10"
    verification:
      - kind: other
        ref: "sed -n '/### Phase 3:/,/### Phase 4:/p' .planning/ROADMAP.md | grep -cE '^  [67]\\.' == 2, grep -c liveness_enabled == 1, grep -c set_cash == 1"
        status: pass
      - kind: other
        ref: "sed -n '/### Phase 6:/,/### Phase 7:/p' .planning/ROADMAP.md | grep -cE '^  7\\.' == 1, grep -c liveness_enabled == 1"
        status: pass
      - kind: other
        ref: "grep -c '^### Phase ' .planning/ROADMAP.md == 11; sed -n '/### Phase 2:/,/### Phase 3:/p' | grep -cE '^  [1-4]\\.' == 4"
        status: pass
    human_judgment: false

duration: ~14 min
completed: 2026-08-31
status: complete
---

# Phase 2 Plan 01: The Liveness Gate, Rendered Addresses and the Source Amendments Summary

**A required `invariants.liveness_enabled` config key landed across its four-file agreement in one commit, `Display` for all five ledger address types so plan 02-02's `Violation` variants can name an agent inline, and the ROADMAP criterion plus LEDG-09 corrected from a search over halves to a linear scan — with the three cross-phase obligations that previously fell between phases each given a named owner.**

## Performance

- **Duration:** ~14 min (commit span 09:02:14Z -> 09:05:43Z; context load preceded it)
- **Completed:** 2026-08-31
- **Tasks:** 3 of 3
- **Files modified:** 6

## Accomplishments

- **The liveness gate exists as configuration, not as a constant.** `config/baseline.toml` ships `[invariants] liveness_enabled = false` behind its two-line `# GRADE:` block; `src/config.rs` exposes `Params::invariants` as a bare required `bool` with no serde default and no optional wrapper; `config/PROVENANCE.md` carries the matching row. The four-part agreement (TOML key, schema field, provenance row, schema-leaf test) landed in one commit because three existing tests fail on any subset.
- **Every ledger address renders.** `household:12`, `firm-slot:3`, `good:0`, `firm:3:0`, with `Account` delegating to the inner identity. Two unit tests assert all six strings by full equality and prove two generations of one slot render differently. Both pass in debug and release.
- **The two authoritative sources now describe what this phase builds.** ROADMAP Phase 2 criterion 2 and REQUIREMENTS LEDG-09 both say localisation is a linear scan of the per-tick journal for the first non-conserving posting. Without this, Phase 2 would have closed against a criterion describing the exact algorithm plans 02-02/02-03/02-04 turn into a hard grep failure.
- **Three cross-phase obligations acquired owners.** Phase 3 criterion 6 (the process-level halt, naming the overridden key and reconciling explicitly with criterion 1), Phase 3 criterion 7 (the `Household`/`Firm` balance-field obligation, previously recorded only inside a lint failure string), Phase 6 criterion 7 (the flip of the shipped gate value, with its provenance row).

## Task Commits

1. **Task 1: The `[invariants]` liveness gate across all four config files at once** — `664788f` (feat)
2. **Task 2: Give every ledger address a stable rendered form** — `2b17413` (feat)
3. **Task 3: Make the two authoritative sources describe what this phase delivers** — `b28630d` (docs)

Plus one deviation fix, see below:

- **Deviation fix (belongs to Task 1)** — `365861b` (fix)

## Files Created/Modified

- `config/baseline.toml` — new `[invariants]` table appended after `[ownership]`, one key `liveness_enabled = false` preceded by exactly two comment lines. The config hash changes; no golden log or snapshot exists yet to invalidate.
- `src/config.rs` — `pub struct Invariants` with the same derive set and `deny_unknown_fields` as `Ownership`; `Params::invariants`; the embedded `FULL` test document extended to match. `Params::validate` untouched — a boolean has no out-of-domain value.
- `config/PROVENANCE.md` — section-2 row for `invariants.liveness_enabled`; prose counts corrected 41 -> 42 keys and 15 -> 16 grade-PROJECT rows.
- `src/ids.rs` — five `Display` impls with a block comment recording why the generation is in the rendered form and why no path/host/clock/PID may be; two new unit tests.
- `.planning/ROADMAP.md` — Phase 2 criterion 2 localisation clause plus a trailing amendment note; Phase 3 criteria 6 and 7; Phase 6 criterion 7.
- `.planning/REQUIREMENTS.md` — LEDG-09's own line rewritten, plus the fifth `*Rationale (amended …)*` block in the file.

## Decisions Made

- **The shipped value is `false` and its flip has a phase, not a date.** T-02-02 in the plan's threat register names the silent-off-switch risk; three separate records now close it — Phase 3 criterion 6 pins the *on* behaviour to an executed binary-level test, Phase 6 criterion 7 owns the flip of the *shipped* value, and Phase 2 criterion 3 remains the origin both cross-reference.
- **The firm rendered form carries the generation.** `firm:3:0` and `firm:3:1` are different strings because they are different firms. A halt naming only the slot would reintroduce, at the point a human reads the message, exactly the aliasing the generation was put in the identity to prevent.
- **The superseded verb is scrubbed from the ROADMAP but kept in REQUIREMENTS.** The ROADMAP is what the phase gate reads, so there is nowhere in it a superseded spelling can sit harmlessly. In REQUIREMENTS it survives on exactly one line — inside the rationale that explains its supersession — mirroring how CORE-03's rationale names the generator it bans.
- **`config/PROVENANCE.md`'s prose counts were corrected even though no test reads them.** Section 3 is a procedure a human runs from this document; a row count that disagrees with the table is a defect in the instrument, not cosmetic drift.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The embedded `FULL` test document in `src/config.rs` needed the new table**

- **Found during:** Task 1 (surfaced at the plan-level `cargo test --locked --all-targets` after Task 3)
- **Issue:** The unit tests in `src/config.rs` parse a hand-written `FULL` TOML constant rather than the shipped file. Adding `[invariants]` to `Params` left twelve of them failing with `missing field \`invariants\``. The plan's `<read_first>` for Task 1 pointed at lines 100-145 and 405-415 of `src/config.rs`; the fixture sits around line 555 and was not named, so the four-file change was in fact a five-part agreement.
- **Fix:** Appended `[invariants]\nliveness_enabled = false` to the `FULL` constant. The fixture is a second copy of the schema and moves with it.
- **Files modified:** `src/config.rs`
- **Verification:** `cargo test --locked --all-targets` — 89 lib tests pass (was 77 passed / 12 failed); release profile also green.
- **Committed in:** `365861b`

**2. [Rule 2 - Missing critical] `config/PROVENANCE.md` prose counts were left stale by the new row**

- **Found during:** Task 1
- **Issue:** The document states "41 keys, 41 rows — the count is set by the schema in `src/config.rs`" and "**Counts.** 41 rows: 2 grade A, 23 grade B, 1 grade C, 15 grade PROJECT". Adding a key makes both wrong. No test reads them, so this would have drifted silently, in the one document whose section 3 is a verification procedure a person executes by hand.
- **Fix:** Updated to 42 keys / 42 rows and 16 grade PROJECT. The A/B/C counts are unchanged — the new row is grade PROJECT.
- **Files modified:** `config/PROVENANCE.md`
- **Verification:** `grep -n '42 keys\|42 rows' config/PROVENANCE.md`; `cargo test --locked --test provenance` green.
- **Committed in:** `664788f` (part of the Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 x Rule 3 blocking, 1 x Rule 2 missing critical).
**Impact on plan:** Both were necessary for correctness and neither widened scope. Deviation 1 is a genuine gap in the plan's own "four-file change" framing — it is a five-part agreement, and a future config-leaf addition should treat `src/config.rs`'s `FULL` constant as the fifth part. No architectural change; Rule 4 was never reached.

## Issues Encountered

None. No authentication gates, no package-manager installs (the plan and phase add zero dependencies; `Cargo.lock` is unchanged, and `cargo test --locked` would fail on any change to it).

## Verification Results

Every plan-level `<verification>` command was run at HEAD:

| Command | Result |
|---|---|
| `cargo test --locked --all-targets` | pass — 89 + 14 + 14 + 4 + 4 + 5 + 6 + 8 tests, 0 failed |
| `cargo test --locked --release --all-targets` | pass — 87 + 14 + 14 + 4 + 4 + 5 + 6 + 8 tests, 0 failed |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| `cargo fmt --check` | pass |
| `bash tests/lints.sh` | pass — all 60 resolvable method bans fire, no alias/exemption/non-portable generator escapes |
| `bash tests/toolchain.sh` | pass |

The two-sided documentation checks required by the plan both hold, including the deliberate subtlety: `grep -ci 'bisect' .planning/REQUIREMENTS.md` is `1` (the mandated rationale block), while the LEDG-09 requirement line itself is `0` for `bisect` and `1` for `linear scan`.

## Known Stubs

None. `liveness_enabled = false` is not a stub — it is the value ROADMAP Phase 2 criterion 3 requires for Phase 3, and ROADMAP Phase 6 criterion 7 (added by this plan) owns changing it.

## Threat Flags

None. The plan's `<threat_model>` anticipated every surface this plan touches: T-02-01 and T-02-02 (the config gate), T-02-03 (the rendered forms), T-02-04 (provenance drift) and T-02-SC (package installs, of which there were none). No new network endpoint, auth path, file access pattern or trust-boundary schema change beyond the one config key the register already covers.

## Next Phase Readiness

Plan 02-02 is unblocked on both counts it depended on this plan for:

- `Params::invariants.liveness_enabled` exists, so the tracer's check set compiles and can be constructed off a config value rather than a constant.
- All five address types render, so every `Violation` variant's `{account}` and `{good}` interpolation produces `household:0` / `firm:0:0` / `good:0` rather than a debug dump — matching the verified halt line in `02-RESEARCH.md` § Error Type Design exactly.

The phase gate now reads Phase 2 criterion 2 as a linear scan, which is the algorithm plans 02-02, 02-03 and 02-04 will implement and guard, so Phase 2 can close against a criterion it does not contradict.

One note for plan 02-06: guard 7f's `Household`/`Firm` obligation is now recorded as ROADMAP Phase 3 criterion 7, so the failure string in that guard can cross-reference the criterion rather than being the only place the obligation lives.

## Self-Check: PASSED

- All six modified files exist on disk and carry the expected content (`grep` assertions in the Verification Results table above).
- All four commits exist in `git log`: `664788f`, `2b17413`, `b28630d`, `365861b`.
- Every task `<acceptance_criteria>` re-run at HEAD and passing.
- Every plan-level `<verification>` command re-run at HEAD and passing.
- `.planning/STATE.md` untouched, as required by this plan's shared-artifact rule.

---
*Phase: 02-books-journal-and-invariants*
*Completed: 2026-08-31*
