---
schema_version: 1
open_count: 2
waived_count: 0
fixed_count: 0
total_count: 2
last_updated: 2026-08-31T00:14:39.114Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 01 | deviation | .planning/REQUIREMENTS.md |  | Tasks 1 and 4 assert grep -c '^- \\[ \\] \\*\\*' == 87 as a no-rows-lost tripwire; it reports 84 because plan 01-01 checked off CORE-02/08/09. Total rows (^- \\[[ x]\\] \\*\\*) is 87. Criterion baseline is stale, not a file defect. | open |  | 2026-08-30T23:17:58.131Z |  |
| 2 | 01 | deviation | config/baseline.toml |  | V-4: theta=0.75 sense contradicts the key name price_inaction_prob_ppm; flagged in config/PROVENANCE.md for the Phase 6 gate, not corrected from memory (D-20) | open |  | 2026-08-31T00:14:39.114Z |  |

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
  }
]
````
