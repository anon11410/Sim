---
phase: 03-world-tick-pipeline-and-log-seam
plan: 01
subsystem: infra
tags: [roadmap-amendment, cargo, csv, serde_json, assert_cmd, tempfile, determinism, dependency-audit]

requires:
  - phase: 01-primitives-and-the-determinism-spine
    provides: "`sha2` as an existing dependency (the `activation_digest` the amended criterion names is a sha256 of the tick permutation, so the correction adds no crate); `Purpose::ActivationOrderHouseholds`/`ActivationOrderFirms` already reserved in `src/rng.rs`"
  - phase: 02-books-journal-and-invariants
    provides: "the ROADMAP inline-rationale amendment shape, established by plan 02-01 on Phase 2 criterion 2's localisation clause — copied here rather than a second convention invented"
provides:
  - "ROADMAP Phase 3 criterion 3, amended to the counter-check that was measured to work: a seed-sensitive `activation_digest` column in `ticks.csv`, with the draw-count column retained explicitly as the divergence localiser"
  - "The name `activation_digest` reserved for plan 03-02's `TickRow` column, so no later plan invents a competing spelling"
  - "`csv` 1.4.0 and `serde_json` 1.0.151 under `[dependencies]`; `assert_cmd` 2.2.2 and `tempfile` 3.27.0 under `[dev-dependencies]`; `Cargo.lock` regenerated in the same commit"
  - "A recorded absence: no `schemars`, no `insta`, no direct `predicates`"
affects: [03-02, 03-04, 03-05, 03-06, "Phase 4 acceptance harness (the distinct-value standing check on the digest column)"]

actuals:
  tokens: 2900
  tasks: 2
  commits: 2

tech-stack:
  added: [csv 1.4.0, serde_json 1.0.151, assert_cmd 2.2.2 (dev), tempfile 3.27.0 (dev)]
  patterns:
    - "ROADMAP criterion amendments carry their measured counterexample inline, opened by a fixed greppable literal"
    - "Dependency-graph changes land on their own commit, ahead of the first commit that imports them, so a bisect can separate 'the graph changed' from 'the writer changed'"

key-files:
  created: []
  modified:
    - ".planning/ROADMAP.md — Phase 3 criterion 3, one line, replaced in place"
    - "Cargo.toml — four dependency entries plus their purpose comments"
    - "Cargo.lock — 15 packages added, purely additive (133 insertions, 0 deletions)"

key-decisions:
  - "Criterion 3's counter-check is scoped to `ticks.csv` specifically, not to 'the logs': at this phase `events.jsonl` carries only the seed-independent opening endowment, so the superseded promise that every log differs would be a red build against a correct simulation."
  - "The draw-count column is retained, not deleted — the correction narrows what it proves (a localiser, and a CORE-05 fixed-draw-sampling assertion) rather than removing a diagnostic. A positive check on `draw-count column` sits beside the negative check on the superseded phrase, because a negative-only check set cannot detect its own regression."
  - "REQUIREMENTS.md TICK-10 is deliberately NOT amended — see the dedicated section below."
  - "`serde_json` takes default features; the map-ordering feature is not enabled, and is not even NAMED in Cargo.toml, because the guard on it is a literal grep."
  - "`predicates` is not a direct dev-dependency: `assert_cmd` depends on it but does not re-export it, and `.assert().failure().code(1)` plus a plain `assert!` on stderr gives a better failure message."

patterns-established:
  - "Two-sided criterion checks: every negative assertion about a document's wording is paired with a positive assertion about what must survive, so 'the clause was corrected' is distinguishable from 'the clause was deleted'."
  - "A guard's literal is not written into the file the guard scans — a mention in a comment is indistinguishable from the thing itself."

# NOT marked complete in REQUIREMENTS.md — see Deviations #2. This plan works TOWARD these
# four but delivers none of them: it contains no production code. Each is marked by the plan
# that actually delivers it (TICK-02 -> 03-04, TICK-03 -> 03-02/03-04, TICK-04 -> 03-03,
# TICK-10 -> 03-02/03-05).
requirements-completed: []

coverage:
  - id: D1
    description: "ROADMAP Phase 3 criterion 3 states the counter-check as a seed-sensitive `activation_digest` in `ticks.csv`, retains the draw-count column as the localiser, scopes the claim away from `events.jsonl`, hands Phase 4 the distinct-value standing check, and carries its amendment parenthetical."
    requirement: "TICK-10"
    verification:
      - kind: other
        ref: "sed -n '/### Phase 3:/,/### Phase 4:/p' .planning/ROADMAP.md | grep -cE '^  3\\..*activation_digest'  => 1"
        status: pass
      - kind: other
        ref: "... | grep -cE '^  3\\..*seeds produce different logs'  => 0"
        status: pass
      - kind: other
        ref: "... | grep -cE '^  3\\..*draw-count column'  => 1 (the positive half of the pair)"
        status: pass
      - kind: other
        ref: "... | grep -cE '^  3\\..*ticks\\.csv'  => 1"
        status: pass
      - kind: other
        ref: "... | grep -c 'Mechanism clause amended 2026-08-31; authority:'  => 1"
        status: pass
      - kind: other
        ref: "grep -c '^### Phase ' .planning/ROADMAP.md => 11; Phase 3 criteria count => 7; git diff --numstat => 1 insertion, 1 deletion"
        status: pass
    human_judgment: false
  - id: D2
    description: "The log seam's four crates resolve, build under `--locked`, and leave every existing gate green, with `schemars` and `insta` absent from the resolved graph."
    requirement: "TICK-02"
    verification:
      - kind: other
        ref: "cargo build --locked => 0, and git diff --exit-code Cargo.lock after it => 0"
        status: pass
      - kind: other
        ref: "bash tests/toolchain.sh => 0 (prints its OK: line, incl. no getrandom on the behaviour path)"
        status: pass
      - kind: other
        ref: "bash tests/lints.sh => 0"
        status: pass
      - kind: other
        ref: "cargo clippy --all-targets --all-features -- -D warnings => 0"
        status: pass
      - kind: integration
        ref: "cargo test --locked --all-targets => 0 (242 tests)"
        status: pass
      - kind: other
        ref: "grep -cE 'name = \"(schemars|insta)\"' Cargo.lock => 0; grep -cE 'name = \"(csv|serde_json|assert_cmd)\"' Cargo.lock => 3"
        status: pass
    human_judgment: false

duration: 5min
completed: 2026-08-31
status: complete
---

# Phase 3 Plan 01: Criterion 3 Amendment and the Log Seam's Dependencies Summary

**The one ROADMAP criterion this phase could not have satisfied as written now describes the mechanism that was measured to work — a seed-sensitive `activation_digest` in `ticks.csv` rather than a constant draw count — and the four crates the rest of the phase compiles against are in the manifest and the lockfile with the two rejected ones provably out of the graph.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-08-31T14:14:00Z
- **Completed:** 2026-08-31T14:19:00Z
- **Tasks:** 2 of 2
- **Files modified:** 3

## Accomplishments

- Corrected the phase gate before the work it gates was built. ROADMAP criterion 3 prescribed a counter-check against vacuous reproducibility that `03-RESEARCH.md` built exactly as written and measured to be *itself* vacuous — byte-identical output at seeds 42 and 43 over 3,650 ticks. The criterion now names the column that actually carries the seed.
- Paired every negative assertion with a positive one. The check that the superseded phrasing is gone cannot, on its own, distinguish a correction from a deletion; a companion check requires the draw-count column to survive as the divergence localiser.
- Landed the dependency-graph change on its own commit, ahead of any code that imports it, so a future bisect can separate a graph change from a writer change. 15 packages added, purely additive, zero removals.
- Kept `schemars` and `insta` out of the resolved graph, asserted rather than assumed.

## Task Commits

1. **Task 1: Amend ROADMAP Phase 3 criterion 3** — `5b14b05` (docs)
2. **Task 2: The log seam's four crates in the manifest and the lockfile** — `6e2a664` (chore)

**Plan metadata:** see the final commit on this branch (docs: complete plan)

## Files Created/Modified

- `.planning/ROADMAP.md` — Phase 3 criterion 3, one line replaced in place. Diff is exactly 1 insertion and 1 deletion; all eleven phase headings and Phase 3's other six criteria are byte-identical to before.
- `Cargo.toml` — `csv` and `serde_json` added to `[dependencies]`; `assert_cmd` and `tempfile` added to `[dev-dependencies]`; each block carries a purpose comment in the manner of the existing `sha2` block. `[profile.release]` and `[lints.clippy]` untouched.
- `Cargo.lock` — regenerated by `cargo build` and committed in the same commit as the manifest.

## The criterion, before and after

**Before** (`.planning/ROADMAP.md` line 120 at `9140ac3`):

> 3. Counter-check against the vacuous-reproducibility pass: two runs at **different** seeds produce different logs, because the empty pipeline consumes at least one RNG draw per tick (activation-order shuffle plus a per-tick draw-count column). Reproducibility cannot pass by never consuming the RNG.

**After:**

> 3. Counter-check against the vacuous-reproducibility pass: two runs at **different** seeds produce a **different `ticks.csv`**, because every tick logs an `activation_digest` — a digest of that tick's activation permutation, and therefore a seed-sensitive *value* rather than a seed-independent *count*. The activation-order shuffle and the per-tick draw-count column are both **retained**, but as the divergence **localiser** rather than as the counter-check itself: the draw count is constant by construction, and that constancy is an assertion worth making in its own right, because a tick whose draw count moved is a fixed-draw-sampling violation (CORE-05). The claim is scoped to `ticks.csv` deliberately — at this phase `events.jsonl` carries only the opening endowment, which is seed-independent, so a criterion promising that *every* log differs would be a red build against a correct simulation. Standing check handed forward: any determinism column whose distinct-value count over a run is 1 is a constant column and proves nothing about the seed, so Phase 4's harness asserts the digest column has more than one distinct value. Reproducibility cannot pass by never consuming the RNG, and this counter-check cannot pass by observing only that the RNG was consumed. *(Mechanism clause amended 2026-08-31; authority: `03-RESEARCH.md` Pitfall 1, restated in `03-CONTEXT.md`. It previously read that a different seed produces “different logs, because the empty pipeline consumes at least one RNG draw per tick (activation-order shuffle plus a per-tick draw-count column)”; that design was built exactly as the criterion described and run at seeds 42 and 43 over 3,650 ticks, and `cmp` returned byte-identical — the draw count is 218 on every tick of every run at every seed, a constant column that proves draws occurred and says nothing about which — while adding the `activation_digest` column flipped the same comparison to differing at tick 0; both directions measured.)*

The amendment parenthetical follows the shape plan 02-01 established on Phase 2 criterion 2 (`*(Localisation clause amended 2026-08-31; authority: …. It previously read "…"; <measured counterexample>.)*`) — same document, same convention, distinguishable at a glance by its opening literal.

## Why REQUIREMENTS.md TICK-10 was NOT amended

TICK-10 reads *"A different seed produces different logs."* That is a statement about the **finished simulation**, and it becomes literally true of every log the moment economics exists in Phase 5 — wages, purchases and dividends all depend on sampled quantities, so `events.jsonl` will differ between seeds without any further machinery. Nothing about it is wrong; it simply is not a claim about a mechanism.

The **criterion** is different in kind: it names the mechanism, and it is the text a Phase 3 verification gate reads. Left as written it would have certified a counter-check that returns byte-identical at two different seeds — evidence that certifies nothing. So the criterion is what moved and the requirement stayed put.

**This is a deliberate asymmetry, not an oversight.** A later reader comparing the two documents will find the requirement's wording unchanged next to an amended criterion; that is the intended end state.

## Package legitimacy verdicts

From `03-RESEARCH.md` § Package Legitimacy Audit (`gsd-tools query package-legitimacy check --ecosystem crates`), carried forward here so the adoption decision is re-derivable without opening the research file:

| Package | First published | Downloads/wk | Source repo | Verdict | Disposition |
|---|---|---|---|---|---|
| `csv` 1.4.0 | 2014-11-21 | 3,490,786 | github.com/BurntSushi/rust-csv | OK | **Adopted** (normal) |
| `serde_json` 1.0.151 | 2015-08-07 | 22,522,340 | github.com/serde-rs/json | OK | **Adopted** (normal) |
| `assert_cmd` 2.2.2 | 2018-05-28 | 1,217,085 | github.com/assert-rs/assert_cmd | OK | **Adopted** (dev) |
| `tempfile` 3.27.0 | 2015-04-14 | 13,362,731 | github.com/Stebalien/tempfile | OK | **Adopted** (dev) |
| `predicates` 3.1.4 | 2017-06-02 | 3,007,756 | github.com/assert-rs/predicates-rs | OK | Legitimate, **not adopted** — unnecessary |
| `insta` 1.48.0 | 2019-01-13 | 1,972,642 | github.com/mitsuhiko/insta | OK | Legitimate, **not adopted** — dep weight |
| `schemars` 1.2.2 | 2019-08-08 | 11,815,846 | github.com/GREsau/schemars | OK | Legitimate, **not adopted** — wrong output |

No `[ASSUMED]` package, no `[SUS]` package, no `[SLOP]` removal — so **no install checkpoint was required**, and none was raised. Cargo has no `postinstall` equivalent; the pinned `Cargo.lock` plus `--locked` in CI is what fixes the resolved bytes from here on.

## The two rejections, recorded

- **`schemars` — not in the graph.** It cannot see `#[serde(serialize_with)]`, and this repository uses it on both of `Posting`'s address fields: the writer emits `"debit":"household:12"` where schemars declares a `$ref` to a `oneOf`. It also emits `properties` alphabetically, so it does not record CSV column order — which is precisely the contract `ticks.csv` must freeze. Re-argued at the point of use in plan 03-04.
- **`insta` — not in the graph.** `cargo add --dev insta --features json` pulls 17 packages including `fastrand`, which this project's own `CLAUDE.md` § What NOT to Use names by name. The committed-golden-file alternative is ~8 lines and 0 packages. Re-argued at the point of use in plan 03-06.

`grep -cE 'name = "(schemars|insta)"' Cargo.lock` prints `0`.

## `tempfile` added no package

`tempfile` was already in the resolved graph as a transitive dependency of `proptest`. Promoting it to a **direct** dev-dependency changed the manifest but added nothing to the graph: the lockfile's 15 new packages are `assert_cmd`, `bstr`, `csv`, `csv-core`, `difflib`, `itoa`, `memchr`, `predicates`, `predicates-core`, `predicates-tree`, `regex-automata`, `ryu`, `serde_json`, `termtree`, `zmij` — `tempfile` is not among them. This matches `03-RESEARCH.md`'s prediction of 7 normal-edge plus 8 dev-edge packages exactly.

Note that `predicates` **is** in the lockfile, as `assert_cmd`'s own dependency. It is not a direct dev-dependency and must not become one: `assert_cmd` does not re-export it, so `predicates::str::contains` would not compile from a test without the direct edge, and the phase's tests use `.assert().failure().code(1)` plus a plain `assert!` on captured stderr instead.

## Verify results

**Task 1** — all seven checks green:

| Check | Expected | Got |
|---|---|---|
| `grep -c '^### Phase ' .planning/ROADMAP.md` | 11 | 11 |
| Phase 3 criteria `^  [1-7]\.` | 7 | 7 |
| criterion 3 names `activation_digest` | 1 | 1 |
| criterion 3 contains `seeds produce different logs` | 0 | 0 |
| criterion 3 contains `draw-count column` (positive half) | 1 | 1 |
| `Mechanism clause amended 2026-08-31; authority:` | 1 | 1 |
| criterion 3 contains `ticks.csv` | 1 | 1 |
| `git diff --numstat .planning/ROADMAP.md` | scoped | `1 1 .planning/ROADMAP.md` |

**Task 2** — all ten checks green (one after the deviation below was resolved):

| Check | Expected | Got |
|---|---|---|
| `grep -cE '^csv[[:space:]]*=' Cargo.toml` | 1 | 1 |
| `grep -cE '^serde_json[[:space:]]*=' Cargo.toml` | 1 | 1 |
| `grep -cE '^(assert_cmd\|tempfile)[[:space:]]*=' Cargo.toml` | 2 | 2 (both read as sitting **below** `[dev-dependencies]`) |
| `grep -c 'preserve_order' Cargo.toml` | 0 | 0 (initially 1 — see Deviations) |
| `grep -cE 'name = "(schemars\|insta)"' Cargo.lock` | 0 | 0 |
| `grep -cE 'name = "(csv\|serde_json\|assert_cmd)"' Cargo.lock` | 3 | 3 |
| `cargo build --locked` | exit 0 | exit 0; `git diff --exit-code Cargo.lock` after it also exit 0 |
| `bash tests/toolchain.sh` | exit 0 | exit 0, `OK:` line printed |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 | exit 0 |
| `cargo test --locked --all-targets` | exit 0 | exit 0 |

**Whole-plan verification** — additionally `bash tests/lints.sh` exits 0.

## Decisions Made

Recorded in the frontmatter `key-decisions` and argued in the sections above. The load-bearing one: the criterion moved and the requirement did not, because only one of them describes a mechanism.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The `preserve_order` check fired on my own comment, not on the feature**

- **Found during:** Task 2
- **Issue:** The plan's action text asks for a one-line purpose comment on each new entry, and separately forbids enabling `serde_json`'s map-ordering feature. My first comment explained the prohibition by naming the feature. The verify check is `grep -c 'preserve_order' Cargo.toml`, expecting `0`, and it printed `1`. Its `fails_when` reads *"`serde_json`'s `preserve_order` feature was enabled"* — which was **not** what happened. The manifest line was `serde_json = "1.0.151"` with default features throughout; only the prose contained the token.
- **Why this was fixed rather than escalated:** the condition the `fails_when` names did not occur, so this was not a real gate trip — it was a false positive produced by my own optional prose. The check is deliberately blunt (any occurrence in the file is a failure), and that bluntness is the safe direction: a mention in a comment is textually indistinguishable from an enablement. The correct resolution is therefore to change the comment, never to loosen the check. Loosening it would have been the working-around; this was not.
- **Fix:** reworded the comment to describe the feature by function ("serde_json's map-ordering feature") instead of by name, and added a sentence recording *why* the literal is absent, so the next author does not reintroduce it. Also corrected a first draft of that sentence which claimed the guard lives in `tests/toolchain.sh` — it does not; that would have been exactly the defect shape this project keeps finding, a stated claim that is not the thing that exists.
- **Files modified:** `Cargo.toml` (comment only; no manifest key changed)
- **Verification:** `grep -c 'preserve_order' Cargo.toml` → `0`; the `serde_json` entry is unchanged and takes default features.
- **Committed in:** `6e2a664` (part of the task commit)

**2. [Rule 1 - Bug] The state-update step marked four requirements complete that this plan did not deliver**

- **Found during:** post-task state updates
- **Issue:** The executor protocol mechanically feeds the plan's `requirements:` frontmatter (`[TICK-02, TICK-03, TICK-04, TICK-10]`) to `requirements mark-complete`. Doing so flipped four checkboxes and four traceability rows in `.planning/REQUIREMENTS.md` from Pending to Complete. **All four claims were false at that moment:** TICK-02 asserts `schema/schema.json` is generated and committed (no such file exists — plan 03-04 builds it), TICK-03 asserts `ticks.csv` is written (no writer exists — 03-02), TICK-04 asserts `events.jsonl` is written (no writer exists — 03-03), and TICK-10 asserts a different seed produces different logs (there are no logs — 03-05 writes that test). This plan contains **no production code at all**; it could not have satisfied any of them.
- **Why this is a bug and not a protocol quibble:** it is precisely the defect shape this plan was written to fix — a document asserting something that is not what was measured — reproduced in the requirement ledger while fixing it in the ROADMAP. A reader or a later gate consulting REQUIREMENTS.md would have been told the log seam was delivered.
- **Fix:** reverted `.planning/REQUIREMENTS.md` (`git checkout --`). All four IDs stay **Pending**. Nothing is orphaned by this: TICK-02 is claimed by 03-04, TICK-03 by 03-02 and 03-04, TICK-04 by 03-03, and TICK-10 by 03-02 and 03-05 — each will be marked by the plan that actually delivers it.
- **The frontmatter itself is correct and was not changed.** `requirements:` records which requirements a plan *works toward*, which is why this plan legitimately names all four; it is the mechanical equation of "named" with "delivered" that was wrong.
- **Files modified:** none (a revert of an unwanted automatic edit)
- **Verification:** `grep -E '^\| TICK-(02|03|04|10) ' .planning/REQUIREMENTS.md` → all four rows read `Pending`; `git diff --stat .planning/REQUIREMENTS.md` → empty.

---

**Total deviations:** 2 auto-fixed (1 × Rule 3, 1 × Rule 1).
**Impact on plan:** none on substance. No manifest key, version, feature or table placement differs from what the plan specified. No scope creep.

## Issues Encountered

**The plan's stated test count is stale.** Task 2's `fails_when` for the test suite refers to "the existing 244-test suite". The actual count is **242**, both with and without doc-tests. Only `Cargo.toml` and `Cargo.lock` changed in this plan — `git status --short` confirms no test file was touched — so 242 was also the count before this plan ran. This is a plan-text arithmetic slip, not a regression, and nothing was done to the tests. Recorded here so a later reader does not go looking for two deleted tests.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Plan 03-02 (the tracer) is unblocked on both counts:

- The name `activation_digest` is now reserved and, more importantly, is what the phase gate reads — so 03-02's `TickRow` column and the criterion agree before the column is written.
- `csv` is resolvable, so 03-02's `ticks.csv` writer compiles; `serde_json` is resolvable for 03-03's event stream and 03-04's schema emitter; `assert_cmd` and `tempfile` are resolvable, dev-only, for 03-05's cross-process determinism test and the process-level halt.

**One obligation handed forward, outside this phase:** Phase 4's acceptance harness must assert `df.activation_digest.nunique() > 1`. The criterion now says so in its own text, which is the point — a determinism column that never varies is the failure mode this whole plan exists to close, and it will recur unless something asserts against it.

**No blockers.**

---
*Phase: 03-world-tick-pipeline-and-log-seam*
*Completed: 2026-08-31*

## Self-Check: PASSED

All three modified files exist on disk; both task commits (`5b14b05`, `6e2a664`) are present in `git log`.
