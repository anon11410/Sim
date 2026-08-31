---
phase: 01-primitives-and-the-determinism-spine
plan: 08
subsystem: configuration-provenance
tags: [provenance, config, CORE-11, CORE-10, testing]
status: complete

requires:
  - 01-06 (config/baseline.toml and the six-table Params schema)
  - 01-02 (the CORE-10 carve-out clause, the CORE-11 split, ROADMAP criterion 5 split)
  - 01-05 (src/numeric.rs and its three constants)
provides:
  - config/PROVENANCE.md — per-key grade/source/cadence/verification-state, the
    domain-knowledge-free Phase 6 verification procedure, and the GRADE PROJECT
    record for the src/numeric.rs constants that the amended CORE-10 points at
  - per-key "# GRADE: … | SOURCE: … | CADENCE: …" annotations in config/baseline.toml
  - tests/provenance.rs — the six-test automated gate replacing review
affects:
  - Phase 6 (carries the blocking paper-verification gate; amends test 6 when it runs)
  - Phase 11 (CAL-01/CAL-02 calibrate the 15 PROJECT-grade rows)

tech-stack:
  added: []
  patterns:
    - raw-text scanning of the config, because the TOML parser discards comments
    - a hand-rolled line scanner instead of a regex dependency
    - a failing test as the review gate on a provenance upgrade

key-files:
  created:
    - config/PROVENANCE.md
    - tests/provenance.rs
  modified:
    - config/baseline.toml

key-decisions:
  - "Grade-B rows are marked UNVERIFIED regardless of which paper attributes them: the test keys off grade B, not off the string 'Lengnick', because grade B means 'from an annotated replication, not read from the published paper' — which is exactly the unverified condition. This makes the two BAM rows honest too."
  - "The verification-state vocabulary has four values, not the three the plan named: grade C needed 'UNVERIFIED — derived from a grade-B source', because the arithmetic is checkable here but its input is not."
  - "21 config keys are attributed to the baseline-model paper, not the 18 D-19 states. The 18 counts graded-table rows; 21 counts config keys, and one graded row can expand into two keys."
  - "The sense of θ = 0.75 is flagged as an open verification item (V-4) rather than corrected: the graded table reads it as P(considers a change) while the key is named price_inaction_prob_ppm. Per D-20 an agent does not resolve this from memory."

requirements-completed: [CORE-11]

coverage:
  - deliverable: "Every leaf key in the shipped config carries exactly one adjacent grade/source/cadence annotation"
    verification:
      - kind: test
        ref: "tests/provenance.rs#every_key_has_a_source_grade"
        status: pass
      - kind: test
        ref: "tests/provenance.rs#no_annotation_is_orphaned"
        status: pass
      - kind: command
        ref: "grep -c '^# GRADE: ' config/baseline.toml → 41"
        status: pass
    human_judgment: false
  - deliverable: "An un-annotated key fails by name, and an out-of-vocabulary grade letter fails"
    verification:
      - kind: test
        ref: "tests/provenance.rs#every_grade_letter_is_in_the_vocabulary"
        status: pass
      - kind: command
        ref: "injected probe key → every_key_has_a_source_grade FAILED naming ownership.unannotated_probe_key; injected grade Z → FAILED naming \"Z\"; both reverted"
        status: pass
    human_judgment: false
  - deliverable: "config/PROVENANCE.md has one row per leaf key, grade-B rows marked UNVERIFIED, with a verification procedure and the GRADE PROJECT code-constants record"
    verification:
      - kind: test
        ref: "tests/provenance.rs#every_config_key_has_a_provenance_row"
        status: pass
      - kind: test
        ref: "tests/provenance.rs#attributed_rows_are_still_marked_unverified"
        status: pass
      - kind: command
        ref: "grep -c 'UNVERIFIED' config/PROVENANCE.md → 30; grep -c 'GRADE: PROJECT' → 4"
        status: pass
    human_judgment: false
  - deliverable: "The annotated config still parses and drives the binary end to end"
    verification:
      - kind: command
        ref: "cargo run -- --config config/baseline.toml --seed 7 --out $(mktemp -d) → exit 0, tracer effective_seed=7"
        status: pass
    human_judgment: false
  - deliverable: "The A/B/C/PROJECT vocabulary means the same thing in config/PROVENANCE.md as in .planning/research/SUMMARY.md"
    human_judgment: true
    rationale: "The plan authored this as a `verification: backstop` truth. The definitions are quoted verbatim from SUMMARY.md:169 with the file and line cited, and tests/provenance.rs pins the four letters — but whether two documents *use* a vocabulary identically is a reading judgement no test can assert. A verifier should read section 1 of config/PROVENANCE.md against SUMMARY.md:169 and 211."

metrics:
  duration: "26 min"
  completed: 2026-08-31
  tasks: 3
  files: 3
  commits: 3

actuals:
  tokens: 21000
  tasks: 3
  commits: 3
---

# Phase 01 Plan 08: Config Provenance and the Annotation Gate Summary

Every one of the 41 shipped config keys now states its source grade, source and cadence in a
comment block a test can check, backed by `config/PROVENANCE.md` where the 23 grade-B rows say
plainly that no agent has read them from a published paper, above a procedure a person with
journal access can execute without domain knowledge.

## Accomplishments

**Task 1 — `config/baseline.toml` annotated key by key.** Each of the 41 leaf keys the schema in
`src/config.rs` declares is now immediately preceded by a machine-checkable line of the shape
`# GRADE: <A|B|C|PROJECT> | SOURCE: … | CADENCE: <day|month|period|none>`, above a human
description. Every grade, source and cadence was transcribed **in session** by reading the graded
table at `.planning/research/SUMMARY.md:171-209`; none was recalled. Grade-B sources carry
`(UNVERIFIED)` inline, so a value's inherited-from-a-replication status is legible at the point of
use and not only in a separate document. **No value changed** — `git diff` on the file shows added
comment lines only, with no assignment line touched, verified mechanically.

Grade distribution over the 41 keys: **2 A, 23 B, 1 C, 15 PROJECT**.

**Task 2 — `config/PROVENANCE.md`.** Four sections, as specified:

1. The grade vocabulary quoted verbatim from `.planning/research/SUMMARY.md:169` with the file and
   line cited, plus the "Do NOT treat as sourced" paragraph from line 211, and a one-line statement
   that the vocabulary is reused rather than invented. Also defines the cadence and
   verification-state vocabularies this file introduces.
2. A 41-row table — one per leaf key, keyed by dotted table path — with value, grade, source,
   cadence and verification state.
3. The verification procedure: which document to open (the published JEBO article, grade A for this
   purpose), the warning that the open-access mirror URL came from a search-result title rather than
   a fetch and may not resolve, the three recordable outcomes `agrees` / `differs` (with the paper's
   value written down) / `not in Table 1`, the explicit no-silent-overwrite rule, the statement that
   this is a **blocking** gate on Phase 6 per D-19, and the instruction that no further automated
   fetch is to be scheduled.
4. The `GRADE: PROJECT` record for `POW_FRAC_BITS`, `PPM_SCALE` and `MILLI_SCALE`, with the
   rationale and the honest caveat that `POW_FRAC_BITS` is nonetheless a committed constant whose
   change alters every trajectory exactly as an economic parameter would.

The file opens with a one-line statement of its own status: no value in an `UNVERIFIED` row has
been read from a primary source by any agent, in this session or the previous research round.

**Task 3 — `tests/provenance.rs`.** Six named tests over the raw text of both files (the TOML
parser discards comments, so a parsed representation cannot see an annotation at all), implemented
with a hand-rolled line scanner and no regex dependency. Each was proven to bite by injecting the
defect it exists to catch, observing the named failure, and reverting:

| Test | Injected defect | Observed failure |
|---|---|---|
| `every_key_has_a_source_grade` | probe key appended to config + schema | `ownership.unannotated_probe_key (line 165)` |
| `every_grade_letter_is_in_the_vocabulary` | one grade letter changed to `Z` | `line 40: grade "Z" is not one of ["A","B","C","PROJECT"]` |
| `every_annotation_has_a_source_and_a_cadence` | — (positive path; malformed lines covered by the field parser shared with test 2) | — |
| `no_annotation_is_orphaned` | blank line inserted between an annotation and its key | `line 28: … is followed by "", not by a key assignment` |
| `every_config_key_has_a_provenance_row` | — (positive path; 41/41 keys matched) | — |
| `attributed_rows_are_still_marked_unverified` | one `UNVERIFIED` changed to `VERIFIED` | `sim.month_days: verification state is "VERIFIED"` |

A file-level comment states that test 6 is expected to be amended by Phase 6 and by no earlier
phase, so a future reader does not treat it as an obstacle to route around.

## The CORE-10 carve-out clause, closed

The prior-wave context flagged that plan `01-02` amended CORE-10 to keep `POW_FRAC_BITS`,
`PPM_SCALE` and `MILLI_SCALE` as consts in `src/numeric.rs` **on the condition** that each is
"recorded with a `GRADE: PROJECT` entry in `config/PROVENANCE.md`", and that the clause was not
satisfiable because the file did not exist. **It exists now and section 4 is that record.** All
three constants appear by name with a `GRADE: PROJECT` marking, their values and a stated
rationale; `src/numeric.rs`'s doc comment on `POW_FRAC_BITS` already pointed forward at this file,
and that forward reference now resolves. The phase verifier can confirm the clause is closed by
reading `config/PROVENANCE.md` section 4 against `.planning/REQUIREMENTS.md:22`.

## Counts the plan asked to be reported and reconciled

**Leaf-key count: 41, matching the plan's stated 41** (Sim 5, MoneySection 1, Household 13, Firm 18,
Bankruptcy 3, Ownership 1). No discrepancy. The count was taken from the schema in `src/config.rs`,
not from the graded table's 37 rows, and confirmed by walking the parsed TOML.

**Rows attributed to the baseline-model paper: 21 found, against the 18 stated in CONTEXT.md D-19.
Flagged, not forced.** The two numbers count different things and both are correct on their own
terms: D-19's 18 counts *rows of the graded table*, while this plan's table counts *config keys*,
and a single graded row can expand into two keys — `P(price search) / P(rationing search)` is one
graded row and two keys (`household.price_search_prob_ppm`, `household.rationing_search_prob_ppm`).
Several other graded rows describe rules rather than parameters (the reservation-wage ratchet, the
marginal-cost formula, the bankruptcy trigger) and have no config key at all. The larger number is
the one to work from at the Phase 6 gate, because it is the set of keys a person must actually
check; `config/PROVENANCE.md` section 2 records the reconciliation in place.

`grep -c 'UNVERIFIED' config/PROVENANCE.md` reports **30**, above the plan's ≥18 floor: 23 grade-B
rows, 1 grade-C row, plus the prose occurrences in the status statement and the procedure.

## Deviations from Plan

### Auto-fixed / judgement calls

**1. [Rule 2 - Missing critical] The verification-state vocabulary has four values, not three**
- **Found during:** Task 2
- **Issue:** The plan named three states — `UNVERIFIED` (paper-attributed), `N/A — project choice`
  (PROJECT) and `VERIFIED — authors' code` (A). Grade C fell through the gaps:
  `bankruptcy.incumbent_trim_per_tail` is derived arithmetic (5% of 20 firms = 1) whose *input* is
  a grade-B BAM row that has never been read either.
- **Fix:** Added a fourth state, `UNVERIFIED — derived from a grade-B source`. Marking it
  `VERIFIED` would have been false and marking it `N/A` would have hidden an unread dependency.
- **Files modified:** `config/PROVENANCE.md`
- **Commit:** f96fd7b

**2. [Judgement] Test 6 keys off grade B rather than off the string "Lengnick"**
- **Found during:** Task 3
- **Issue:** The plan describes `attributed_rows_are_still_marked_unverified` as covering rows
  "whose source attributes it to the baseline-model paper". Implemented literally, the two
  BAM-attributed rows — equally unread, equally grade B — would have been free to be upgraded
  silently.
- **Fix:** The test asserts every **grade-B** row carries `UNVERIFIED`. Grade B is defined as "an
  annotated replication citing the paper's table/equation numbers", which *is* the unverified
  condition; the check is a strict superset of the plan's and is simpler to read. It also asserts
  it found at least one grade-B row, so a drift in the table shape cannot make it pass on nothing.
- **Files modified:** `tests/provenance.rs`
- **Commit:** e726b32

**3. [Cosmetic] Four annotation blocks are three comment lines, not two**
- **Found during:** Task 1
- **Issue:** The plan specifies "two comment lines". Four keys
  (`household.consumption_exponent_ppm`, `firm.price_inaction_prob_ppm`,
  `firm.initial_expected_demand`, `bankruptcy.incumbent_trim_per_tail`) needed a wrapped
  two-line description to stay inside the line width.
- **Fix:** Kept the descriptions at two lines where needed. The load-bearing invariant is
  **adjacency** — the `# GRADE:` line is still the immediately-preceding non-blank line of its key
  in all 41 cases, which is what `no_annotation_is_orphaned` asserts and what makes the
  association positional.
- **Commit:** b746838

**Total deviations:** 3 (1 missing-critical auto-fix, 1 judgement strengthening a check, 1
cosmetic). **Impact:** none negative — each makes the gate stricter or more honest than the plan
specified. No plan requirement was weakened or skipped.

## Issues Encountered

**None blocking.** One item recorded for Phase 6 rather than resolved here, per the plan's own
prohibition:

**V-4 — the sense of θ.** The graded table at `.planning/research/SUMMARY.md` reads θ = 0.75 as
*"P(firm considers a price change)"*, while the config key shipped by plan `01-06` is named
`price_inaction_prob_ppm` — the complementary event. One of the two readings is wrong, and which
one changes how often prices move by a factor of three. Per D-20 this was **not** corrected from
model memory: the value is transcribed exactly as the graded table gives it, the mismatch is
flagged inline in the config annotation, and it is recorded as open item V-4 in
`config/PROVENANCE.md` section 3 for the Phase 6 gate to settle against the paper. This is
precisely the class of defect CORE-11's verification clause exists to catch, and it was surfaced by
the act of writing the provenance down.

**V-5 — the demand-expectation cadence** (carried in from the `01-06` summary, not owned here):
`firm.initial_expected_demand` is a per-month quantity while
`firm.productivity_units_per_worker_day` is per-day and λ smoothing is per *period*. Recorded as an
open item because it bears on how a reader interprets the `period` cadence in the provenance table.
Flagged as a Phase 5+ modelling question, not a provenance defect, and not resolved here.

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or FIXME was introduced by this plan, and
no test was skipped.

The `UNVERIFIED` markings are **not** stubs — they are the accurate, load-bearing output of this
plan. Marking them anything else without a primary-source read is the exact failure the plan exists
to prevent.

## Threat Flags

None. This plan adds no network endpoint, no auth path, no file-access pattern and no schema change
at a trust boundary. It writes documentation and comments, and adds a read-only test that opens two
files under `CARGO_MANIFEST_DIR`.

The threat register's four entries are each mitigated as planned: T-1-26 (a value written from
model memory) by in-session transcription plus the no-silent-upgrade test; T-1-27 (unestablishable
origin) by the 41 annotations and 41 provenance rows; T-1-28 (a drifted annotation) by
`no_annotation_is_orphaned`; T-1-21 (a run identified by less than its full input) by the config
hash covering the raw bytes, so the added comments are inside the run's identity — the shipped
config's hash is now `b7ea1d3b6e7fa51505852a6aba41a81a2bd103f86b6e2c92961b09074c2a55ba`.

## Verification Results

| Check | Result |
|---|---|
| `cargo run -- --config config/baseline.toml --seed 7 --out $(mktemp -d)` | exit 0, `tracer effective_seed=7 …` printed |
| `grep -c '^# GRADE: ' config/baseline.toml` | 41 |
| `grep -cE '^# GRADE: (A\|B\|C\|PROJECT) \| SOURCE: .+ \| CADENCE: .+$'` | 41 — identical, so every annotation is well-formed |
| `grep -cE '^# GRADE: PROJECT'` | 15 (floor was 9) |
| assignment lines modified by the annotation diff | 0 |
| `test -s config/PROVENANCE.md` | exit 0 |
| `grep -c 'UNVERIFIED' config/PROVENANCE.md` | 30 (floor was 18) |
| `grep -c 'GRADE: PROJECT' config/PROVENANCE.md` | 4 |
| provenance rows vs config leaf keys | 41 / 41, none missing |
| `cargo test --test provenance` | 6 passed, 0 failed |
| `cargo test --release --test provenance` | 6 passed, 0 failed |
| full suite, debug | 112 passed, 0 failed (was 106; +6 added here) |
| full suite, release | 110 passed, 0 failed (was 104; +6) |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `rustfmt --check tests/provenance.rs` | clean (the two pre-existing `cargo fmt` failures in `src/money.rs` and `tests/tracer_end_to_end.rs` are untouched — deferred item owned by plan `01-07`) |

## Success Criteria

- **CORE-11 (annotation clause):** met. Every config value carries a source-grade annotation,
  enforced by `tests/provenance.rs` rather than by review, and the test names any unannotated key.
- **CORE-11 (verification clause):** the machinery and the domain-knowledge-free procedure ship
  here; the verification itself is a blocking gate on Phase 6 per D-19, and the 21 affected rows say
  honestly that they are unverified until then.
- **Every leaf key annotated:** 41/41, count set by `src/config.rs`.
- **The amended CORE-10 carve-out has its recorded rationale:** `config/PROVENANCE.md` section 4.
- **No attributed number written from memory, no row silently upgraded:** every grade, source and
  cadence read in session from the in-repo graded table; the θ mismatch was flagged rather than
  fixed; no row is marked verified.

## Next Phase Readiness

Plan `01-08` is the last plan of Phase 1's wave 3. Two hand-offs:

- **Phase 6** owns CORE-11 clause (b). Its first action before consuming any Lengnick value is
  `config/PROVENANCE.md` section 3, including open items V-1 through V-4. Amending
  `tests/provenance.rs::attributed_rows_are_still_marked_unverified` is part of that commit, not a
  workaround for it.
- **Plan `01-07`** still owns the two pre-existing `cargo fmt` failures; nothing here touched them.

## Self-Check: PASSED

- `config/PROVENANCE.md` — FOUND
- `config/baseline.toml` — FOUND
- `tests/provenance.rs` — FOUND
- commit `b746838` — FOUND
- commit `f96fd7b` — FOUND
- commit `e726b32` — FOUND
- working tree clean after all four mutation checks were reverted
- no tracked file deleted by any commit in this plan
