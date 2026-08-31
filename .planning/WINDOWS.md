---
schema_version: 1
open_count: 10
waived_count: 0
fixed_count: 1
total_count: 11
last_updated: 2026-08-31T10:11:22.368Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | deviation | .planning/REQUIREMENTS.md |  | Tasks 1 and 4 assert grep -c '^- \\[ \\] \\*\\*' == 87 as a no-rows-lost tripwire; it reports 84 because plan 01-01 checked off CORE-02/08/09. Total rows (^- \\[[ x]\\] \\*\\*) is 87. Criterion baseline is stale, not a file defect. | open |  | 2026-08-30T23:17:58.131Z |  |
| 2 | 01 | deviation | config/baseline.toml |  | V-4: theta=0.75 sense contradicts the key name price_inaction_prob_ppm; flagged in config/PROVENANCE.md for the Phase 6 gate, not corrected from memory (D-20) | open |  | 2026-08-31T00:14:39.114Z |  |
| 3 | 02 | deviation | .planning/phases/02-books-journal-and-invariants/02-02-PLAN.md |  | Task 2 verify command 'cargo test --locked --lib books invariants' is not valid cargo syntax (one positional TESTNAME only); use 'cargo test --locked --lib -- books invariants'. Plans 02-03..02-07 must not copy the broken form. | open |  | 2026-08-31T09:22:48.072Z |  |
| 4 | 02 | stub | src/books.rs | 118 | Posting::units_out/units_in/goods_residual_units are zero on every posting this phase produces; plan 02-03 gives them values and 02-04 adds the goods-conservation check that reads them | fixed |  | 2026-08-31T09:22:48.317Z | 2026-08-31T09:38:20.748Z |
| 5 | 02 | deviation | .planning/phases/02-books-journal-and-invariants/02-03-PLAN.md |  | Task 1 acceptance criterion "grep -c 'pub fn produce' src/books.rs prints 1" is unsatisfiable: the pattern is a substring of 'pub fn produced', the accessor the same plan mandates in <artifacts_produced>. It prints 2. Anchored form 'pub fn produce\\(' prints 1. Same for consume/consumed. | open |  | 2026-08-31T09:38:20.990Z |  |
| 6 | 02 | deviation | src/invariants.rs |  | Violation carries Option<Box<Posting>> rather than Option<Posting>: a second posting-bearing variant pushed the enum past clippy::result_large_err's 128-byte threshold, which under -D warnings refuses to compile every Result in the crate that propagates a violation. Plans 02-04/02-05 construct Violations and must box the posting. | open |  | 2026-08-31T09:38:29.928Z |  |
| 7 | 02 | deviation | tests/invariant_halt.rs |  | Modified outside plan 02-03's files_modified: its active-check sequence assertion is [MoneyConservation, Liveness] and inserting goods conservation at position two made it fail. Plan 02-04 inserts two more checks and must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks). | open |  | 2026-08-31T09:38:30.201Z |  |
| 8 | 02 | deviation | src/books.rs |  | Plan 02-04 task 2 modified src/books.rs though its <files> lists only src/invariants.rs: check_non_negative cannot walk accounts without an enumerator (Books::accounts) and no accessor exposed the per-slot firm generations needed to build a firm Account. Also added PostError::EmptyExchange. | open |  | 2026-08-31T09:55:20.419Z |  |
| 9 | 02 | deviation | tests/invariant_halt.rs |  | Modified outside plan 02-04's files_modified, discharging ledger entry 7: the active-check sequence assertions now read five checks with the gate on and four with it off. Behavioural claims untouched. Plans 02-05..02-07 that add a check must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks). | open |  | 2026-08-31T09:55:20.666Z |  |
| 10 | 02 | deviation | src/invariants.rs |  | ZeroSumDetail ships 8 variants, not the 6 in plan 02-04's <artifacts_produced>: SplitParties (a one-party kind naming two accounts) and EmptyExchange (the action text's 'both legs non-zero' clause, which the six listed variants cannot express). Plan 02-05's message tests should expect eight. | open |  | 2026-08-31T09:55:20.916Z |  |
| 11 | 02 | deviation | .planning/ROADMAP.md |  | roadmap.update-plan-progress rewrote the Phase 2 plan checklist as a side effect; reverted per the wave shared-artifact rule (STATE.md and ROADMAP.md are owned by the orchestrator while sibling plans 02-06/02-07 are outstanding). Plans 02-06 and 02-07 will hit the same side effect and must revert it too. | open |  | 2026-08-31T10:11:22.368Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "01",
    "file": ".planning/REQUIREMENTS.md",
    "line": null,
    "description": "Tasks 1 and 4 assert grep -c '^- \\[ \\] \\*\\*' == 87 as a no-rows-lost tripwire; it reports 84 because plan 01-01 checked off CORE-02/08/09. Total rows (^- \\[[ x]\\] \\*\\*) is 87. Criterion baseline is stale, not a file defect.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-30T23:17:58.131Z",
    "resolved_at": null
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "01",
    "file": "config/baseline.toml",
    "line": null,
    "description": "V-4: theta=0.75 sense contradicts the key name price_inaction_prob_ppm; flagged in config/PROVENANCE.md for the Phase 6 gate, not corrected from memory (D-20)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T00:14:39.114Z",
    "resolved_at": null
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "02",
    "file": ".planning/phases/02-books-journal-and-invariants/02-02-PLAN.md",
    "line": null,
    "description": "Task 2 verify command 'cargo test --locked --lib books invariants' is not valid cargo syntax (one positional TESTNAME only); use 'cargo test --locked --lib -- books invariants'. Plans 02-03..02-07 must not copy the broken form.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:22:48.072Z",
    "resolved_at": null
  },
  {
    "id": 4,
    "kind": "stub",
    "phase": "02",
    "file": "src/books.rs",
    "line": 118,
    "description": "Posting::units_out/units_in/goods_residual_units are zero on every posting this phase produces; plan 02-03 gives them values and 02-04 adds the goods-conservation check that reads them",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-08-31T09:22:48.317Z",
    "resolved_at": "2026-08-31T09:38:20.748Z"
  },
  {
    "id": 5,
    "kind": "deviation",
    "phase": "02",
    "file": ".planning/phases/02-books-journal-and-invariants/02-03-PLAN.md",
    "line": null,
    "description": "Task 1 acceptance criterion \"grep -c 'pub fn produce' src/books.rs prints 1\" is unsatisfiable: the pattern is a substring of 'pub fn produced', the accessor the same plan mandates in <artifacts_produced>. It prints 2. Anchored form 'pub fn produce\\(' prints 1. Same for consume/consumed.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:38:20.990Z",
    "resolved_at": null
  },
  {
    "id": 6,
    "kind": "deviation",
    "phase": "02",
    "file": "src/invariants.rs",
    "line": null,
    "description": "Violation carries Option<Box<Posting>> rather than Option<Posting>: a second posting-bearing variant pushed the enum past clippy::result_large_err's 128-byte threshold, which under -D warnings refuses to compile every Result in the crate that propagates a violation. Plans 02-04/02-05 construct Violations and must box the posting.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:38:29.928Z",
    "resolved_at": null
  },
  {
    "id": 7,
    "kind": "deviation",
    "phase": "02",
    "file": "tests/invariant_halt.rs",
    "line": null,
    "description": "Modified outside plan 02-03's files_modified: its active-check sequence assertion is [MoneyConservation, Liveness] and inserting goods conservation at position two made it fail. Plan 02-04 inserts two more checks and must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks).",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:38:30.201Z",
    "resolved_at": null
  },
  {
    "id": 8,
    "kind": "deviation",
    "phase": "02",
    "file": "src/books.rs",
    "line": null,
    "description": "Plan 02-04 task 2 modified src/books.rs though its <files> lists only src/invariants.rs: check_non_negative cannot walk accounts without an enumerator (Books::accounts) and no accessor exposed the per-slot firm generations needed to build a firm Account. Also added PostError::EmptyExchange.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:55:20.419Z",
    "resolved_at": null
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "02",
    "file": "tests/invariant_halt.rs",
    "line": null,
    "description": "Modified outside plan 02-04's files_modified, discharging ledger entry 7: the active-check sequence assertions now read five checks with the gate on and four with it off. Behavioural claims untouched. Plans 02-05..02-07 that add a check must update the same two assertions (here and src/invariants.rs#the_gate_decides_the_exact_sequence_of_active_checks).",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:55:20.666Z",
    "resolved_at": null
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "02",
    "file": "src/invariants.rs",
    "line": null,
    "description": "ZeroSumDetail ships 8 variants, not the 6 in plan 02-04's <artifacts_produced>: SplitParties (a one-party kind naming two accounts) and EmptyExchange (the action text's 'both legs non-zero' clause, which the six listed variants cannot express). Plan 02-05's message tests should expect eight.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:55:20.916Z",
    "resolved_at": null
  },
  {
    "id": 11,
    "kind": "deviation",
    "phase": "02",
    "file": ".planning/ROADMAP.md",
    "line": null,
    "description": "roadmap.update-plan-progress rewrote the Phase 2 plan checklist as a side effect; reverted per the wave shared-artifact rule (STATE.md and ROADMAP.md are owned by the orchestrator while sibling plans 02-06/02-07 are outstanding). Plans 02-06 and 02-07 will hit the same side effect and must revert it too.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T10:11:22.368Z",
    "resolved_at": null
  }
]
````
