---
schema_version: 1
open_count: 4
waived_count: 0
fixed_count: 0
total_count: 4
last_updated: 2026-08-31T09:22:48.317Z
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
| 4 | 02 | stub | src/books.rs | 118 | Posting::units_out/units_in/goods_residual_units are zero on every posting this phase produces; plan 02-03 gives them values and 02-04 adds the goods-conservation check that reads them | open |  | 2026-08-31T09:22:48.317Z |  |

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
    "status": "open",
    "reason": "",
    "recorded_at": "2026-08-31T09:22:48.317Z",
    "resolved_at": null
  }
]
````
