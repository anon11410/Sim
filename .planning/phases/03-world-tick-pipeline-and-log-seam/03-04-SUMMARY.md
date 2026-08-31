---
phase: 03-world-tick-pipeline-and-log-seam
plan: 04
subsystem: infra
tags: [rust, serde_json, csv, schema, wire-format, drift-test, mutation-testing, clap, determinism]

# Dependency graph
requires:
  - phase: 01-spine-config-rng-money
    provides: the float-confinement guard in tests/numeric_det.rs, the generated-and-committed-artifact precedent (clippy.toml + tests/lints.sh)
  - phase: 02-ledger-and-invariants
    provides: Posting with its two #[serde(serialize_with)] address fields — the exact type the schema-derive approach was measured describing wrongly
  - phase: 03-02
    provides: TickRow, header_of, ticks_header, RunWriter, phases::run, the real CLI in src/main.rs
  - phase: 03-03
    provides: Event, ProvenanceRow, Decision, Rule, provenance_header, SCHEMA_VERSION, the eager-header mechanism
provides:
  - "sim::log::schema_json — the deterministic wire-format schema, read out of the writers themselves"
  - "sim::log::first_difference — the first differing line of two texts, as a line number and both lines"
  - "sim::log::SCHEMA_FILE, sim::log::SCHEMA_REGEN_COMMAND — the artifact path and the one command that regenerates it"
  - "schema/schema.json — 2,300 bytes, 73 lines, generated and committed; the contract Phase 4 reads across the disk boundary"
  - "--dump-schema — a mode flag; --config becomes required only in its absence"
  - "tests/log_schema.rs — the drift test, the tick-file shape test and the provenance-header test"
  - "tests/schema_drift_negative.sh — the mutation proof, wired into CI beside the lint gate"
affects: [03-05, 03-06, phase-04-analysis-harness, phase-05-second-good, phase-06-labour, phase-08-accounting, phase-09-pricing, phase-10-bankruptcy]

actuals:
  tokens: 11779   # chars/4 over the added lines of the realized diff (47,115 chars), NOT a harness token count
  tasks: 3
  commits: 3

tech-stack:
  added: []   # no new dependency — that is the point of this plan, see "Why no schema-derive crate"
  patterns:
    - "Generate a schema from the bytes the writers emit: names from the emitted text, types from parsing that same text, so there is no second description that could disagree"
    - "A generated-and-committed artifact whose drift test NEVER writes; regeneration is an operator command named in the failure message"
    - "Every drift test paired with a mutation script that has been watched failing, restoring under a trap and verifying the restore by digest"
    - "One type name per line in a hand-composed artifact, so a build's negative grep over it can stay a bare substring pattern"

key-files:
  created:
    - schema/schema.json
    - tests/log_schema.rs
    - tests/schema_drift_negative.sh
  modified:
    - src/log.rs
    - src/main.rs
    - .github/workflows/ci.yml

key-decisions:
  - "No schema-derive crate, on measured evidence rather than on taste. A derive is a second, independent description of the types that cannot see #[serde(serialize_with = …)] — and this project uses one on BOTH address fields of a serialised posting. It also emits properties alphabetically, so it does not record CSV column order at all, which is the tick file's whole contract."
  - "The schema's field names come from the emitted TEXT and its types from parsing that same text. Reading the order from the parsed value instead would report every record alphabetically, because serde_json's Value is backed by a key-ordered map. This is asserted by a unit test over a struct whose fields are declared in an order no sort produces."
  - "The classifier returns an explicit UNSUPPORTED marker rather than guessing, and the committed artifact is asserted to contain none. A silent fallback would let a shape the Python side cannot read pass as understood."
  - "The drift test never writes. A test that regenerated and then compared would compare the generator with itself and pass however far the wire format drifted — the same discipline clippy.toml already gets in this repository."
  - "The float-dtype check in the build is a bare `grep -cE 'float(64|32)'`, deliberately not the full `\"dtype\": \"float64\"` spelling. It is a NEGATIVE check over hand-composed text: a pattern that also pinned the key name and the spacing would pass vacuously the moment either drifted. The one-type-name-per-line spelling is pinned by a unit test instead, so the two checks do not depend on each other."
  - "Nothing in the schema module names a floating-point type. The type name for a non-integral number is the string literal \"float64\", which contains no `f64` at identifier boundaries and is therefore invisible to the confinement guard — the same measured escape the SCHEMA_VERSION spelling used, and it was checked rather than assumed."
  - "--config is made conditionally required through clap's own required_unless_present attribute rather than made optional and hand-checked. A hand-check would report a missing configuration as a panic instead of as a usage error."
  - "The perturbation in the mutation script is an adjacent column swap, not a corruption. A syntactically broken file would prove only that the test reads a file; a column reorder is the realistic defect, and the analysis side reads columns positionally."
  - "The mutation script asserts the drift test failed WITH ITS OWN DIAGNOSTIC, and that the restored run reports `1 passed`. A test run that failed to build also exits non-zero, and a name filter matching nothing exits 0 reporting `0 passed` — both would otherwise be read as proof."

patterns-established:
  - "Read a contract out of the artifact, never describe it twice: one derivation for names, one for types, both from the same serialiser."
  - "Pair every drift test with a mutation script in the mould of tests/lints.sh — perturb, observe the block, restore under a trap, verify the restore by digest, observe the pass."
  - "A guard's negative pattern and the spelling it searches are pinned by two different tests, so neither can go vacuous behind the other."

requirements-completed: [TICK-02, TICK-03, TICK-07]

coverage:
  - id: D1
    description: "The log schema is generated from the bytes the writers actually emit — the CSV header the writer itself produces and the field order the JSON writer itself writes — so it cannot describe a shape the files do not have"
    requirement: TICK-02
    verification:
      - kind: unit
        ref: "cargo test --locked --lib log::schema (11 tests, including the_key_order_comes_from_the_text_and_not_from_the_parsed_value and two_calls_in_one_process_return_identical_bytes)"
        status: pass
      - kind: other
        ref: "cargo run --locked --quiet -- --dump-schema | diff - schema/schema.json → exit 0, no output"
        status: pass
    human_judgment: false
  - id: D2
    description: "An address field that renders through a custom serialiser is reported as the string it is, not as the object a second derive would infer"
    requirement: TICK-02
    verification:
      - kind: unit
        ref: "src/log.rs#schema::a_custom_serialised_address_is_typed_as_the_string_it_becomes — builds a real Posting, asserts the writer emits \"debit\":\"household:12\", then asserts json_fields types both debit and credit as string"
        status: pass
    human_judgment: false
  - id: D3
    description: "schema/schema.json is generated, committed and drift-tested; regeneration is an operator action, never something a test performs"
    requirement: TICK-02
    verification:
      - kind: integration
        ref: "cargo test --locked --test log_schema schema_matches_the_committed_file"
        status: pass
      - kind: other
        ref: "git ls-files --error-unmatch schema/schema.json → exit 0; the test reads the file and never opens it for writing; the failure message names `cargo run --locked --quiet -- --dump-schema > schema/schema.json`"
        status: pass
    human_judgment: false
  - id: D4
    description: "The drift test has been observed to fail on a perturbed schema and on a renamed field — a check watched working rather than one that has only ever been green"
    requirement: TICK-02
    verification:
      - kind: integration
        ref: "bash tests/schema_drift_negative.sh — column swap, drift test exits 101 with `schema drift at line 5`, restore verified by sha256, test passes again reporting `1 passed`"
        status: pass
      - kind: other
        ref: "Mutation run by hand: rng_draws → rng_draws_per_tick in the Rust type only → `schema drift at line 10`, generated \"rng_draws_per_tick\" vs committed \"rng_draws\". Reverted; suite green."
        status: pass
      - kind: other
        ref: "grep -c 'schema_drift_negative.sh' .github/workflows/ci.yml → 1, beside the lint gate"
        status: pass
    human_judgment: false
  - id: D5
    description: "The schema records ticks.csv's columns in order and every one of them as an integer"
    requirement: TICK-03
    verification:
      - kind: integration
        ref: "cargo test --locked --test log_schema ticks_csv_is_flat_and_integer_only — header order, 3,650 rows × 9 cells each parsing as i64, no empty cell, no carriage return, money named with the _cents suffix"
        status: pass
      - kind: unit
        ref: "src/log.rs#schema::every_tick_column_is_an_integer_in_file_order"
        status: pass
      - kind: other
        ref: "head -1 of a real run's ticks.csv equals, field for field, the ordered column list the committed schema records"
        status: pass
    human_judgment: false
  - id: D6
    description: "The schema records provenance.csv's seven columns, which is what lets Phase 4 read a dtype for a table that is legitimately empty"
    requirement: TICK-07
    verification:
      - kind: integration
        ref: "cargo test --locked --test log_schema provenance_has_a_header_even_with_no_rows — the file exists, is non-empty, is exactly one line, and that line is the full seven-column header"
        status: pass
      - kind: unit
        ref: "src/log.rs#schema::the_provenance_table_carries_seven_columns_with_their_declared_types — agent, decision and rule typed string, the rest int64"
        status: pass
    human_judgment: false
  - id: D7
    description: "No schema entry carries a floating dtype or an unsupported marker"
    requirement: TICK-03
    verification:
      - kind: other
        ref: "grep -cE 'float(64|32)' schema/schema.json → 0; grep -c 'UNSUPPORTED' schema/schema.json → 0"
        status: pass
      - kind: unit
        ref: "src/log.rs#schema::nothing_is_unsupported_and_no_type_is_fractional, #exactly_one_type_name_per_line_in_the_pretty_printed_spelling"
        status: pass
    human_judgment: false

# Metrics
duration: ~20min
completed: 2026-08-31
status: complete
---

# Phase 3 Plan 04: The Committed Wire Format Summary

**A 2,300-byte schema read out of the writers' own bytes rather than described a second time — and a drift test watched failing on a column swap and on a field rename, because a generator and a generated file that are wrong in the same way agree with each other forever.**

## Performance

- **Duration:** ~20 min (commit span 15:16:10 → 15:20:38 UTC, plus the whole-plan verification block)
- **Completed:** 2026-08-31
- **Tasks:** 3 of 3
- **Files created/modified:** 6 (`1,201 insertions(+), 6 deletions(-)`)
- **Test count:** 285 → **299** in debug (+14: `log::schema` 11 library, `log_schema` 3 integration), 283 → **297** in release — the two-test gap is the pre-existing `#[cfg(debug_assertions)]` sub-stream re-entry pair in `src/rng.rs`, unchanged by this plan

## Task Commits

1. **Task 1: Generate the schema from the writers** — `55c31bf` (feat)
2. **Task 2: The schema dump mode, and the committed artifact** — `7ff6e31` (feat)
3. **Task 3: The shape tests, the mutation script and its build step** — `76dfa5b` (test)

## The committed artifact, exactly

| Fact | Value |
|---|---:|
| `schema/schema.json` | **2,300 bytes**, 73 lines |
| Regeneration command | `cargo run --locked --quiet -- --dump-schema > schema/schema.json` |
| `ticks.csv` columns recorded | 9, all `int64` |
| `provenance.csv` columns recorded | 7 — `agent`, `decision`, `rule` as `string`, the rest `int64` |
| Event variants recorded | 5 (hire, fire, dividend, bankruptcy, endowment), tag first in each |
| Fractional dtypes | 0 |
| `UNSUPPORTED` markers | 0 |

The command string lives in exactly one place in the source — `sim::log::SCHEMA_REGEN_COMMAND` — and the drift test's failure message prints it, so a reader who hits drift is handed something runnable rather than a description.

```json
{
  "schema_version": "v1",
  "ticks.csv": [
    { "name": "tick", "dtype": "int64" },
    { "name": "total_money_cents", "dtype": "int64" },
    …
  ],
  "provenance.csv": [
    { "name": "tick", "dtype": "int64" },
    { "name": "agent", "dtype": "string" },
    …
  ],
  "events.jsonl": [
    {
      "event": "endowment",
      "fields": [
        { "name": "event", "dtype": "string" },
        { "name": "tick", "dtype": "int64" },
        { "name": "account", "dtype": "string" },
        …
      ]
    }
  ]
}
```

The tick file a real run writes opens with

```
tick,total_money_cents,firm_cash_cents,stock_units,headcount,transactions,rng_draws,activation_digest,postings
```

which is, field for field and in order, the column list the committed schema records. That agreement is not asserted by comparing two hand-written lists: both come from `header_of`, the single header derivation `src/log.rs` has carried since plan 03-02.

## Why no schema-derive crate is in the manifest

This is the plan's central design claim, and it was **measured**, not reasoned about. It is recorded here so that a future reader reaching for the obvious library finds the reason instead of re-deriving it.

A derive macro is a **second, independent description of the same types**. It runs beside `serde`, not through it, and it therefore **cannot see `#[serde(serialize_with = …)]`**. Compiled against a replica of this repository's own `Posting`:

| | What it says about `debit: Account` |
|---|---|
| `serde_json` — the writer that actually produces the file | `"debit":"household:12"` — a short string |
| a schema derive — a second macro over the same type | `{"$ref": "#/$defs/Account"}` → `oneOf [ {Household: integer}, {Firm: object} ]` — a tagged object |

This project uses `#[serde(serialize_with = "serialize_account")]` on **both** address fields of `Posting`, and a rendered address is what CONTEXT.md calls the wire-shape stake this phase carries. So the generated schema would have contradicted the bytes for exactly the field that matters most — and, because the generated file and the generator would both have been wrong in the same way, `schema_matches_the_committed_file` would have compared two identical errors and passed forever. The drift test would have looked green while describing a file that does not exist.

The derive also emits `properties` **alphabetically**, so it does not record CSV column order at all. Column order *is* the contract for `ticks.csv`.

The replacement is ~250 lines of `src/log.rs` and **zero new dependencies**: names come from the header `csv::Writer` itself emits and from the key order of the text `serde_json` itself writes; types come from re-parsing that same text. There is no second description anywhere, so the schema cannot disagree with the file.

That property is now a test rather than a claim: `a_custom_serialised_address_is_typed_as_the_string_it_becomes` builds a real `Posting`, asserts the writer emits `"debit":"household:12"`, and then asserts the generator types both address fields `string`.

**The subtler half of the same trick.** The key order must come from the emitted *text*, never from the parsed value: `serde_json::Value` is backed by a key-ordered map, so a reader taking its order from there would report every record alphabetically while the file carries declaration order. `the_key_order_comes_from_the_text_and_not_from_the_parsed_value` pins this with a struct whose fields are declared `zebra, alpha, middle` — an order no sort produces.

## The mutation proofs

The project's recurring defect shape is an assertion whose stated claim is not what it measures, and the suite has been green through every instance of it. So every check this plan adds was watched failing on a real perturbation before it was trusted to be silent.

| # | Mutation | Expected | Observed | Reverted |
|---|---|---|---|---|
| 1 | Swap two adjacent column entries in `schema/schema.json` (a column reorder — still valid JSON) | `schema_matches_the_committed_file` **fails** | **FAILED**, exit 101, `schema drift at line 5`: generated `"total_money_cents"` vs committed `"firm_cash_cents"` | yes, digest-verified |
| 2 | Rename `rng_draws` → `rng_draws_per_tick` **in the Rust type only**, leaving every expectation string untouched | the same test **fails** | **FAILED**, `schema drift at line 10`: generated `"rng_draws_per_tick"` vs committed `"rng_draws"` | yes, `git diff` empty |
| 3 | Swap `Production` and `Wages` in the `PHASES` table | the phase-order tests **fail** | **FAILED** — `the_table_runs_the_documented_sequence` and `an_identifier_cannot_exist_without_a_table_entry`, the latter naming `Production is not at position 2 of the table` | yes, `git diff` empty |

Mutations 1 and 2 are the two the research ran; mutation 1 is now **automated** as `tests/schema_drift_negative.sh` and runs unattended in CI, while 2 and 3 are recorded here as measurements in the manner Phase 2 recorded its own.

Mutation 3 is deliberately a *second, separate* drift test: this plan's success criterion covers both the `PHASES` name sequence and the generated schema, and one drift test firing says nothing about the other.

Mutation 2's output is also the readable-difference helper doing its job. The raw equality assertion it replaces prints a 2.3 KB single-line escaped blob; what a reader now gets is:

```
schema drift at line 10
  generated: "    { \"name\": \"rng_draws_per_tick\", \"dtype\": \"int64\" },\n"
  committed: "    { \"name\": \"rng_draws\", \"dtype\": \"int64\" },\n"
The wire format and the committed contract have parted company. Regenerate
deliberately and review the diff:
    cargo run --locked --quiet -- --dump-schema > schema/schema.json
```

### What the mutation script refuses to accept as proof

Three ways a mutation script can certify nothing, each closed explicitly:

- **A failure that is not the failure.** A test run that fails to compile also exits non-zero. The script requires the captured output to contain `schema drift at line`, so a broken build cannot masquerade as a working guard.
- **A pass over nothing.** `cargo test --test log_schema <name>` exits **0** with `0 passed` when the filter matches nothing, so a renamed test would look exactly like a passing one. The restored run is required to report `1 passed`.
- **A restore that did not restore.** The file is copied back under a `trap` on `EXIT INT TERM`, and the restored bytes are compared against a `sha256` digest taken before the perturbation. `git status --porcelain` is clean after the run — verified.

## Decisions Made

Recorded in full in the frontmatter. The three that will matter to a later reader:

1. **The float-dtype check stays a bare `grep -cE 'float(64|32)'`.** It is a *negative* check over hand-composed text. Pinning the key name and the spacing in the same pattern — `"dtype": "float64"` — would make it pass vacuously the instant either drifted, which is precisely the vacuous-negative failure this phase exists to eliminate. The spelling is pinned separately, by `exactly_one_type_name_per_line_in_the_pretty_printed_spelling`, so the two checks cannot go vacuous behind one another.
2. **Nothing in the schema module names a floating-point type.** `tests/numeric_det.rs` reads whole lines including comments and string literals, and requires identifier boundaries around a float type name. The string literal `"float64"` contains no such occurrence — `f`,`l`,`o`,`a`,`t`,`6`,`4` — so the classifier's own vocabulary is invisible to the guard. This was checked by running the guard, not assumed; wave 3 reproduced the opposite outcome at a cost of one build when `SCHEMA_VERSION` was spelled `"1.0.0"`.
3. **`--config` is conditionally required through `clap`'s `required_unless_present`**, so `--dump-schema` alone is a legal invocation and a missing configuration on the run path is still a usage error rather than a panic.

## Deviations from Plan

Two, both procedural rather than substantive.

**1. Task 1 was committed as one commit rather than as a TDD test → implementation pair.**
- **Found during:** Task 1 (marked `tdd="true"` in the plan).
- **Issue:** The RED gate wants a failing test committed before the implementation. Every test in this task asserts on the *output of the generator*, so a RED commit would have been a tree that does not compile — `schema_json`, `json_fields` and `value_kind` do not exist to be called. A non-compiling commit is not a red test; it is a broken tree.
- **Resolution:** Tests and implementation were written and committed together, and each behaviour listed in the task's `<behavior>` block has a named test. The task's own `<verify>` block asks for `at least 4 tests passed` from `cargo test --locked --lib log::schema`; it reports **11**.
- **Also:** the user's execution instruction for this wave was explicitly "one atomic commit per task", which this follows.

**2. `bash tests/schema_drift_negative.sh` writes a backup under `target/`.**
- **Found during:** Task 3.
- **Issue:** The script needs somewhere to hold the pristine artifact while the perturbed one is in place. `mktemp` would put it outside the repository.
- **Resolution:** `target/schema-drift-negative.backup`, which is already gitignored (`/target`), created with `mkdir -p` and removed by the same `trap` that restores the schema. Nothing untracked survives the run — `git status --porcelain` is empty afterwards, and that is one of the task's own checks.

**Total deviations:** 2, both procedural. **Impact:** none on scope, artifacts or behaviour.

## Issues Encountered

Three, all resolved inside their own task.

1. **`ONLY_GOOD` is private to `src/books.rs`.** The custom-serialiser test needs a `GoodId` to build a `Posting`. Rather than widen `books.rs`'s visibility for a test — an unrelated change to a Phase 2 module — the test constructs `GoodId(0)` directly, which is public.
2. **`clippy::while_let_loop` on the key scanner.** The `loop { let … else { break } }` form was written to dodge `while_let_on_iterator`; clippy wanted the `while let` form instead. `while let` is correct here because the loop body calls `chars.next()` itself, so `while_let_on_iterator` does not apply. Converted; the gate is clean.
3. **`cargo fmt` reflowed three call sites** in `src/log.rs` and `tests/log_schema.rs`. Formatted and re-verified.

## Verification

Every whole-plan check from the plan, run on the committed tree:

| Check | Result |
|---|---|
| `cargo run --locked --quiet -- --dump-schema \| diff - schema/schema.json` | exit 0, no output |
| `cargo test --locked --test log_schema` | 3 passed |
| `bash tests/schema_drift_negative.sh` | exit 0, both OK lines printed, `git status --porcelain` empty |
| `grep -cE 'float(64\|32)' schema/schema.json` | `0` |
| `grep -c 'UNSUPPORTED' schema/schema.json` | `0` |
| `grep -c 'schema_drift_negative.sh' .github/workflows/ci.yml` | `1` |
| `cargo test --locked --all-targets` | 299 passed |
| `cargo test --locked --release --all-targets` | 297 passed |
| `bash tests/lints.sh` | exit 0 |
| `bash tests/toolchain.sh` | exit 0 |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |

## User Setup Required

None.

## Next Phase Readiness

**Ready.** The wire format is frozen and committed, and drift from it is a build failure that has been watched firing.

- **Plan 03-05** (cross-process determinism, TICK-06/08/09/10) inherits an unchanged run directory: this plan added a mode flag and a file under `schema/`, and touched no byte any run writes. The four measured artefact sizes from wave 3 are unchanged.
- **Plan 03-06** (golden run) can freeze its snapshot against a schema that a test now guarantees describes the same bytes.
- **Phase 4** reads `schema/schema.json` as the contract across the disk boundary. Note especially: a header-only CSV reads back in pandas with **every column typed `object`, not `int64`** — measured on this repository's own `provenance.csv`. The harness's dtype assertion must therefore be conditional on a non-empty frame, or read the dtype from this schema. Recording seven `provenance.csv` dtypes is exactly what makes the second option available, and is why TICK-07 is only now complete.
- **Phases 5-11** append to this shape. A new event variant is an appended entry; a renamed or reordered field is a change to two committed artifacts and, from Phase 4, to a reader in another language.

## Self-Check: PASSED

All four created files exist on disk (`schema/schema.json`, `tests/log_schema.rs`, `tests/schema_drift_negative.sh`, this summary); `schema/schema.json` is tracked by git; all three task commits (`55c31bf`, `7ff6e31`, `76dfa5b`) resolve in the repository.

---
*Phase: 03-world-tick-pipeline-and-log-seam*
*Completed: 2026-08-31*
