---
phase: 03-world-tick-pipeline-and-log-seam
plan: 03
subsystem: infra
tags: [rust, serde_json, csv, build-script, jsonl, provenance, run-metadata, determinism, logging]

# Dependency graph
requires:
  - phase: 01-spine-config-rng-money
    provides: config::load returning (Params, sha256), config_hash, the float-confinement guard in tests/numeric_det.rs, the GRADE:PROJECT carve-out precedent in config/PROVENANCE.md § 4
  - phase: 02-ledger-and-invariants
    provides: Books::accounts/cash_of/stock_of/goods, the documented walk order, Books::new's step 5 (endowment postings cleared before tick 0), Account's Display form
  - phase: 03-01
    provides: serde_json in [dependencies], tempfile in [dev-dependencies]
  - phase: 03-02
    provides: Sink/RunWriter/TickRow, the eager-header + derived-header mechanism, phases::run, the real CLI in src/main.rs
provides:
  - "sim::log::Event — externally tagged, five flat variants (hire, fire, dividend, bankruptcy, endowment)"
  - "sim::log::endowment_events — one record per ledger account, read from the accessors in the documented walk order"
  - "sim::log::ProvenanceRow + Decision + Rule — seven flat non-optional columns, two closed vocabularies"
  - "sim::log::provenance_header, sim::log::header_of — one derivation for every table's column names"
  - "sim::log::SCHEMA_VERSION, EVENTS_FILE, PROVENANCE_FILE, RUN_META_FILE"
  - "Sink::event and Sink::provenance on all three sinks"
  - "build.rs — the compiler version string as a compile-time value (SIM_RUSTC_VERSION)"
  - "run_meta.json, written on both the clean and the halted path"
  - "A four-file run directory, three files diffed and none of them empty"
affects: [03-04, 03-05, 03-06, phase-04-analysis-harness, phase-06-labour, phase-08-accounting, phase-09-pricing, phase-10-bankruptcy]

actuals:
  tokens: 12154   # chars/4 over the added lines of the realized diff (48,615 chars), NOT a harness token count
  tasks: 3
  commits: 3

tech-stack:
  added: []   # no new dependency; plan 03-01 added serde_json and tempfile
  patterns:
    - "Externally tagged snake_case enum for a heterogeneous event stream; every variant flat, every field an integer or a rendered address"
    - "Closed vocabulary as a Rust enum rather than &'static str — 'never free text' becomes a property of the type, with no constructor that would accept one"
    - "One header derivation (header_of) shared by every comma-separated table in the module"
    - "build.rs for a build-environment fact, keeping a process spawn off the behaviour path"
    - "Origin-row content as the closure for the zero-byte-artifact hazard, rather than a test exemption"

key-files:
  created:
    - build.rs
  modified:
    - src/log.rs
    - src/phases.rs
    - src/main.rs
    - config/PROVENANCE.md

key-decisions:
  - "No nested record in the event stream (03-RESEARCH.md Open Question 4). A nested field serialises through the library's key-ordered value type while top-level fields keep declaration order, so one file would carry two orderings and the generated schema could record only one."
  - "No per-firm panel in Phase 3 (03-RESEARCH.md Open Question 2). Firm's whole behavioural state is one posted price; a panel with one meaningful column freezes a shape before there is anything to shape it around."
  - "SCHEMA_VERSION is a const in src/log.rs with a GRADE: PROJECT provenance row, not a config leaf (03-RESEARCH.md Open Question 1). The config leaf count stays at 41 and the five-part config-leaf agreement is untouched."
  - "The version is spelled `v1`, without a decimal point. The float-confinement guard reads whole lines and is deliberately string-blind; the Phase 1 precedent is to reword the source, never to widen the allowlist. Reproduced both directions."
  - "Every agent in the event stream is named by its RENDERED address (`household:12`, `firm:3:0`), not by a bare index — one spelling of an agent across events.jsonl, provenance.csv and a serialised posting. This departs from 03-RESEARCH.md Pattern 3, which spelled hire's household as a bare u32."
  - "Decision and Rule are Rust enums, not &'static str. A closed vocabulary the type system owns is what makes TICK-07's 'never free text' unwriteable rather than merely unwritten."
  - "run_meta.json carries no duration and no environment field. A duration differs between two identical runs and the natural repair is to widen a determinism test permanently."
  - "ticks_completed on the halt path is world.tick, because the invariant phase is position 7 and the log is position 8 — the failing tick did not complete. Measured: 0 on a tick-0 liveness halt."
  - "Only TICK-04 was marked Complete of the three in the frontmatter. TICK-05 is also claimed by plan 03-05 and TICK-07 by plan 03-04, so marking all three would repeat the failure WINDOWS entry 25 records."

patterns-established:
  - "Close a vacuous-artifact hazard by construction, not by exemption: the provenance file gets an eager header and the event stream gets genuine origin content, so no test needs to skip an empty file."
  - "Where the plan's own verification arithmetic is wrong, fix the measurement and report it — do not edit the artifact to satisfy a miscalibrated probe."
  - "Mutation-prove every new check on the real tree before trusting it to be silent: six mutations this plan, each reverted."

requirements-completed: [TICK-04]

coverage:
  - id: D1
    description: "events.jsonl carries genuine content at a phase with no economics: one opening-endowment record per account, read from the ledger accessors, summing in cents to the configured money stock"
    requirement: TICK-04
    verification:
      - kind: unit
        ref: "cargo test --locked --lib log::endowment (4 tests: one_record_per_account_in_the_ledgers_documented_walk_order, the_cash_fields_sum_to_the_configured_money_stock, the_endowment_is_read_from_the_accessors_not_from_the_journal, a_run_that_executed_no_tick_still_leaves_a_non_empty_event_file)"
        status: pass
      - kind: other
        ref: "sim --config config/baseline.toml --out target/verify-plan → events.jsonl 18,560 bytes, 220 records, cash fields summing to 2,000,000 cents (summed from the file with awk, not read back from the books)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The event stream already names every variant Phases 5-10 will emit — hire, fire, dividend, bankruptcy — so a later phase adds a call site, not a wire-shape decision"
    requirement: TICK-04
    verification:
      - kind: unit
        ref: "cargo test --locked --lib log::events (10 tests, one round-trip per variant plus the byte-shape, address-form, no-nesting and no-fraction claims)"
        status: pass
      - kind: other
        ref: "Mutation: reordering Hire's tick/firm/household fields fails the_tag_comes_first_then_the_declared_fields; nesting a map in Bankruptcy fails nothing_in_the_stream_nests. Both reverted."
        status: pass
    human_judgment: false
  - id: D3
    description: "provenance.csv exists after a run that recorded zero decisions, with its full column header and no data rows"
    requirement: TICK-07
    verification:
      - kind: unit
        ref: "cargo test --locked --lib log::provenance (8 tests, including a_writer_that_received_no_row_still_leaves_a_full_header and the_header_is_written_exactly_once)"
        status: pass
      - kind: other
        ref: "After a run: 49 bytes, wc -l = 1, head -1 = seven comma-separated names, grep -c ',,' = 0. pandas 3.x read_csv → shape (0, 7) with the correct column names."
        status: pass
      - kind: other
        ref: "Mutation: removing the eager header fails 3 tests (the file drops to 0 bytes); removing has_headers(false) fails 2 (the header is emitted twice). Both reverted."
        status: pass
    human_judgment: false
  - id: D4
    description: "Every provenance column is a fixed enumeration or an integer — the decision, the rule branch and the agent are named values, never free text"
    requirement: TICK-07
    verification:
      - kind: unit
        ref: "src/log.rs#provenance::the_decision_and_rule_columns_are_closed_vocabularies, #no_cell_is_ever_empty"
        status: pass
      - kind: other
        ref: "Decision and Rule are Rust enums with no From<String> and no free-text variant: there is no constructor that would accept a token outside the vocabulary"
        status: pass
    human_judgment: false
  - id: D5
    description: "run_meta.json records the effective seed, the configuration hash and the compiler that built the binary, and is the only file in the run directory that may carry a wall clock"
    requirement: TICK-05
    verification:
      - kind: other
        ref: "grep -cE '\"(seed|config_sha256|rustc)\"' <out>/run_meta.json → 3; the record is 223 bytes over 8 lines; no wall clock is present at all (the permission is unused)"
        status: pass
      - kind: other
        ref: "cargo build --locked exits 0 with build.rs present; bash tests/toolchain.sh green (five checks, none disturbed by a top-level build script)"
        status: pass
    human_judgment: false
  - id: D6
    description: "A halted run still produces a complete run_meta.json recording how far it got and that it ended in a violation"
    requirement: TICK-05
    verification:
      - kind: other
        ref: "sim --config <baseline with liveness_enabled=true> → exit 1, four files written, run_meta.json complete with \"ticks_completed\": 0 and \"exit\": \"violation\""
        status: pass
    human_judgment: false
  - id: D7
    description: "No diffed file gains a timestamp, host name, path or process identifier from any of this"
    requirement: TICK-06
    verification:
      - kind: other
        ref: "grep -cE '\"(duration_ms|elapsed|hostname|pid|cwd|path)\"' <out>/run_meta.json → 0; the three diffed files carry integers and rendered addresses only"
        status: pass
      - kind: other
        ref: "Two runs at seed 42: all three diffed files identical, and none hashes to the empty-string digest e3b0c442… (the vacuous-pass condition)"
        status: pass
    human_judgment: false

# Metrics
duration: ~25min
completed: 2026-08-31
status: complete
---

# Phase 3 Plan 03: The Event Stream, the Provenance Table and the Run Record Summary

**A run directory of exactly four files — three of them diffed and none of them empty — where the naive build wrote two of them at zero bytes and a cross-process hash comparison over them certified nothing.**

## Performance

- **Duration:** ~25 min (commit span 14:56:25 → 15:02:51 UTC)
- **Completed:** 2026-08-31
- **Tasks:** 3 of 3
- **Files created/modified:** 5 (`1110 insertions(+), 43 deletions(-)`)
- **Test count:** 263 → **285** in debug (+22, all library: `log::events` 10, `log::provenance` 8, `log::endowment` 4), 283 in release — the two-test gap is the pre-existing `#[cfg(debug_assertions)]` sub-stream re-entry pair in `src/rng.rs`, unchanged by this plan

## The measured run directory

Every byte count below is from a run of the shipped configuration into an empty directory, and every one matches `03-RESEARCH.md` Pattern 5's measurement exactly.

| File | Bytes | Lines | Diffed? |
|---|---:|---:|---|
| `ticks.csv` | 202,974 | 3,651 (header + 3,650) | yes |
| `events.jsonl` | 18,560 | 220 | yes |
| `provenance.csv` | 49 | 1 (header, zero rows) | yes |
| `run_meta.json` | 223 | 8 | **no — the single quarantined file** |

Two runs at seed 42 produce **identical** digests for all three diffed files, and **none of the three hashes to `e3b0c442…`**, the digest of the empty string. That last clause is the whole point of the plan: before it, `events.jsonl` and `provenance.csv` were both zero bytes, and comparing two of them compared the empty string with itself and passed.

### The endowment, exactly

- **220 records** — 200 households + 20 firm slots, one per account the ledger enumerates.
- **Cash fields summing to exactly 2,000,000 cents.** Summed out of the emitted file with `awk`, and compared against `money.total_money_cents` read from the configuration — never against `books.total_money()`, which would compare the ledger with itself.
- Units fields sum to 3,300 (20 firms × 165 units of opening inventory).
- Walk order is the ledger's own: `household:0` … `household:199`, then `firm:0:0` … `firm:19:0`.

First and last lines as written:

```
{"event":"endowment","tick":0,"account":"household:0","cash_cents":5000,"units":0}
{"event":"endowment","tick":0,"account":"firm:19:0","cash_cents":50000,"units":165}
```

### The run record, exactly

```json
{
  "schema_version": "v1",
  "seed": 42,
  "config_sha256": "b3530ae4d072b7ad4c4070e3f200cceb5e3fab9c6ab616fe51eab36f49dbd4b8",
  "rustc": "rustc 1.94.1 (e408947bf 2026-03-25)",
  "ticks_completed": 3650,
  "exit": "ok"
}
```

A liveness-halted run (baseline with `liveness_enabled = true`) leaves the same four files and a complete record reading `"ticks_completed": 0, "exit": "violation"`.

**No wall clock is present at all.** `03-CONTEXT.md` says this is the only file that *may* carry one; it does not say it must, and nothing here needs one. **No duration**, for the reason the plan gives and worth repeating: a duration differs between two otherwise identical runs, and the natural repair for the resulting red test is to widen the comparison — a widening that would be permanent while the reason for it would be forgotten.

## Task Commits

1. **Task 1: the event stream, opened by the endowment the ledger does not journal** — `a6d60a7` (feat) — `src/log.rs`, `src/phases.rs`
2. **Task 2: the decision-provenance table, present after a run that decided nothing** — `b87a2a3` (feat) — `src/log.rs`
3. **Task 3: the run's own record — seed, config hash and compiler, quarantined from the diff** — `0928ac1` (feat) — `build.rs`, `src/log.rs`, `src/main.rs`, `config/PROVENANCE.md`

## Column and field shapes frozen by this plan

**`events.jsonl`** — externally tagged on `"event"`, snake-cased, tag emitted first then declared fields:

| Variant | Fields, in emitted order |
|---|---|
| `hire` | `tick`, `firm`, `household`, `wage_cents` |
| `fire` | `tick`, `firm`, `household` |
| `dividend` | `tick`, `firm`, `household`, `amount_cents` |
| `bankruptcy` | `tick`, `firm`, `residual_cents` |
| `endowment` | `tick`, `account`, `cash_cents`, `units` |

**`provenance.csv`** — `tick`, `agent`, `decision`, `input_a`, `input_b`, `outcome`, `rule`.
`decision` ∈ {`price`, `wage`, `hire`}; `rule` ∈ {`raised`, `lowered`, `held`, `bounded`}.

**`run_meta.json`** — `schema_version`, `seed`, `config_sha256`, `rustc`, `ticks_completed`, `exit`.

## The consequence handed forward to Phase 4

**A header-only comma-separated file reads back with every column typed as an object, not as an integer.** Measured on this repository's own `provenance.csv`:

```
shape  (0, 7)
cols   ['tick', 'agent', 'decision', 'input_a', 'input_b', 'outcome', 'rule']
dtypes {'tick': 'object', 'agent': 'object', ..., 'rule': 'object'}
```

So HARN-02's dtype assertion **must be conditional on a non-empty frame, or must read the dtype from the generated schema** — which is one of the reasons the schema carries dtypes at all. A harness that asserted `int64` unconditionally would be red against a correct zero-decision run, and the tempting repair (drop the dtype assertion) would remove the one check that keeps `ticks.csv`'s conservation audit an exact integer comparison rather than a tolerance one.

The same file is also the reason the header exists at all: at zero bytes, `pandas.read_csv` **raises** rather than returning an empty frame.

## Open questions resolved

### 03-RESEARCH.md Open Question 4 — is a nested `Posting` wanted in the event stream? **No. Nothing nests.**

A nested field is serialised through the library's own value type, whose backing map is ordered by key, so the nested fields come out **alphabetically** while every top-level field comes out in **declaration order**. One file would then carry two different orderings, plan 03-04's schema emitter could record only one of them, and `pd.read_json(lines=True)` would hand the Python side a dictionary-valued column. Freezing that into the committed schema is exactly the "costly to change from Phase 3 onward" decision `03-CONTEXT.md` warns about.

So there is **no posting record in the event stream at this phase**. The journal is cleared each tick and a full journal dump is an opt-in flag a later phase may add; when it does, it flattens. Enforced by `log::events::nothing_in_the_stream_nests`, which was mutation-proved by nesting a map into `Bankruptcy` and watching it fail.

### 03-RESEARCH.md Open Question 2 — does the per-firm panel carry books-derived columns redundantly? **Not in Phase 3, because there is no panel in Phase 3.**

`Firm`'s whole behavioural state is one posted price. A panel with one meaningful column freezes a shape before there is anything to shape it around, and the schema is the thing that must be right. Phase 9 adds the panel, when `expected_demand`, `price`, `wage_offer` and `last_sales` all exist — and logs the ledger-derived columns (`cash_cents`, `stock_units`) **redundantly** alongside them at that point, because the alternative is a join in every Python query and the redundancy is roughly 4 MB against a 20 MB always-on budget.

### 03-RESEARCH.md Open Question 1 — config leaf or `const`? **`const`, with a `GRADE: PROJECT` row.**

`SCHEMA_VERSION` lives in `src/log.rs` and has a fourth row in `config/PROVENANCE.md` § 4. A schema version is neither an economic parameter nor a numerical-method constant — it is a wire-format label nothing in the model reads. The config leaf count stays at **41** and the five-part config-leaf agreement is untouched.

## The correction to this project's own documentation

`CLAUDE.md` and `research/STACK.md` both state that the serialisation library sorts map keys, giving byte-identical output. **That is true only of its own value type**, whose backing map is ordered. A **hashed-map field** on a serialised struct goes through the map-serialisation path and keeps the map's own iteration order — measured in research as **five different orderings in five consecutive runs of one binary**:

```
{"zeta":0,"alpha":1,"beta":3,"mid":2,"kilo":7,"delta":5,"omega":6,"yankee":4}
{"alpha":1,"zeta":0,"yankee":4,"omega":6,"mid":2,"beta":3,"kilo":7,"delta":5}
{"beta":3,"yankee":4,"delta":5,"zeta":0,"kilo":7,"mid":2,"alpha":1,"omega":6}
{"alpha":1,"delta":5,"beta":3,"yankee":4,"omega":6,"mid":2,"kilo":7,"zeta":0}
{"kilo":7,"omega":6,"beta":3,"alpha":1,"zeta":0,"mid":2,"yankee":4,"delta":5}
```

Nothing in this crate uses one, and the `clippy.toml` type ban plus `tests/lints.sh` check 4a (which catches a type *alias* the lint cannot see) is what keeps it that way. The correction now lives in `src/log.rs`'s module documentation, where a reader deciding whether a map field is safe will actually meet it. **The claim in `CLAUDE.md` and `research/STACK.md` is still uncorrected at source** — see *Deferred Issues*.

## Mutations proved, then reverted

Six, each breaking the thing a check exists to catch:

| Mutation | Caught by | Result |
|---|---|---|
| `endowment_events` returns an empty vector | `log::endowment` | all 4 fail |
| Nest a map into `Event::Bankruptcy` | `log::events::nothing_in_the_stream_nests` | fails |
| Swap `Hire`'s `tick`/`household`/`firm` order | `log::events::the_tag_comes_first_then_the_declared_fields` | fails |
| Drop the eager provenance header | `log::provenance` | 3 fail; the file goes to 0 bytes |
| Leave automatic headers on | `log::provenance` | 2 fail; the header appears twice |
| Spell `SCHEMA_VERSION` as `"1.0.0"` | `numeric_det::confinement_of_the_float_domain` | fails, naming `src/log.rs:125` |

The last reproduces `03-RESEARCH.md` Pitfall 10 exactly, including the diagnostic text.

## Deviations from Plan

### 1. [Rule 1 — miscalibrated verification] `grep -c 'GRADE: PROJECT' config/PROVENANCE.md` prints **5**, not the 4 the plan's check expects

- **Found during:** Task 3.
- **The plan's check:** *"`grep -c 'GRADE: PROJECT' config/PROVENANCE.md` — Prints anything other than `4` — the schema version has no project-grade row (a count of `3`, the pre-existing rows), or a row was duplicated."*
- **What is actually there:** `grep -c` counts matching **lines**, and the phrase occurs on **four** lines before this plan, not three — the three table rows *plus* line 223, which quotes CORE-10's own wording: *"recorded with a `GRADE: PROJECT` entry in `config/PROVENANCE.md` stating why they are not configuration"*. The plan's arithmetic counted only the table.
- **Consequence:** after correctly adding the fourth row, the check prints `5` and fires for a reason it does **not** name — neither "no row" nor "a row was duplicated". This is the same defect shape the phase has now seen eight times: an assertion whose stated claim is not what it measures.
- **What was done:** the artifact was **not** edited to satisfy the probe. Rewording line 223 to dodge a grep would mean mutating a quotation of the requirement to fit a miscalibrated measurement — the wrong direction entirely. Instead the substantive claim was measured directly:

  ```
  grep -cE '^\| `[A-Z_]+` \|.*GRADE: PROJECT' config/PROVENANCE.md   → 4
  ```

  Four project-grade rows in the section-4 table, which is what CORE-10's carve-out is conditional on. **Plan 03-04/03-05 should use the anchored form if this count is ever checked again.**
- **Files:** `config/PROVENANCE.md`. **Commit:** `0928ac1`.

### 2. [Rule 2 — consistency of the wire shape] Every agent in the event stream is a **rendered address**, where research spelled `hire`'s household as a bare `u32`

- **Found during:** Task 1.
- **Research's Pattern 3** declared `Hire { tick, firm: String, household: u32, … }` — the firm as `firm:3:0` and the household as a bare `12`.
- **Why changed:** the endowment variant's `account` field is necessarily a rendered address, so the research shape puts **two spellings of the same thing in one file**: `"account":"household:12"` on one line and `"household":12` on another. A Python-side join or a `grep` for one household would then need to know which variant it was looking at. `src/books.rs`'s own serialiser doc gives the rule — *"so an event stream stays greppable by agent and the ledger, not `src/ids.rs`, owns the wire shape of an address"* — and a bare index does not follow it.
- **Cost of the change:** none measurable. Nothing has a call site yet, no schema is committed until plan 03-04, and no golden run exists until 03-06. `events.jsonl`'s byte count is unaffected (only endowment records are written).
- **Files:** `src/log.rs`. **Commit:** `a6d60a7`.

### 3. [Rule 2 — "never free text" made unwriteable] `Decision` and `Rule` are Rust enums, not `&'static str`

- **Found during:** Task 2.
- **Research's Pattern 3** typed both as `&'static str` with a comment listing the permitted values. The plan's own wording is *"fixed enumerations of static strings"*.
- **Why changed:** a `&'static str` field is only a fixed enumeration by convention — `ProvenanceRow { decision: "whatever a caller felt like", … }` compiles. A Rust enum has no constructor that would accept a token outside the vocabulary, so TICK-07's *"never free text"* becomes a property of the type rather than of a reviewer's discipline, which is precisely what the plan asks the design to achieve. `csv` writes a unit variant as its snake-cased name, so the wire form is unchanged.
- **Also changed:** `agent` is a `String`, not `&'static str`. Research's own comment on that field said *"rendered address: `firm:3:0`"*, which a `&'static str` cannot hold. `ProvenanceRow` therefore is not `Copy`, and the header exemplar is a function rather than a `const`.
- **`Rule`'s variants — `raised` / `lowered` / `held` / `bounded` — are invented here.** Research named none. They cover the branch shape the Phase 6 wage rule, the Phase 6 labour-demand rule and the Phase 9 price rule all share (up, down, no change, bound overrode the arithmetic). Variants are appended, never renamed, so a phase that needs a fifth branch adds one.
- **Files:** `src/log.rs`. **Commit:** `b87a2a3`.

## Deferred Issues

- **`CLAUDE.md` and `.planning/research/STACK.md` still state that the serialisation library sorts map keys**, without the "only for its own value type" qualification. The correction is recorded in `src/log.rs`'s module doc and in this summary, but the two source documents are outside this plan's `files_modified` and neither is a behaviour-path artifact. **Recommended:** correct both in the phase's documentation pass, or in `/gsd-docs-update`. Logged to `WINDOWS.md`.
- **`endowment_events` sums a single account's holdings across `books.goods()` into one `units` field.** There is exactly one good, so the sum is that good's holding today. **Phase 5, which adds a second good, must revisit this field** — one record per account with a summed holding stops meaning anything the moment two goods exist. The doc comment on the function says so; there is no test that would fail on the day it happens, because there is no second good to write one against.
- **`TICK-04` marked Complete with four of five variants having no call site.** `hire`, `fire`, `dividend` and `bankruptcy` are declared and round-trip-tested but nothing emits them until Phases 6, 8 and 10. What Phase 3 owns and has delivered is the **wire shape** and the **origin content**; the clause *"sufficient to reconstruct any agent's history"* becomes testable only once agents have histories. Logged to `WINDOWS.md` so the traceability table's "Complete" is not read as more than it is.
- **`TICK-05` and `TICK-07` left Pending.** Plan 03-05 owns TICK-05's remaining half (the enforced exclusion from the diff, and the halted-run assertion) and plan 03-04 owns TICK-07's (the schema validation). Marking them here would repeat the failure `WINDOWS.md` entry 25 records.

## Known Stubs

None. Four of the five event variants have no call site, but they are a declared wire shape rather than a stub: nothing renders them, nothing reads them, and no code path returns a placeholder in their stead. Zero provenance rows is the correct output of a phase with no decisions, not an unimplemented path — the file, its header and its schema are all real.

## Verification Results

| Check | Result |
|---|---|
| `cargo build --locked` | 0 (build script compiles; `SIM_RUSTC_VERSION` read under the name it emits) |
| `cargo test --locked --lib log::events` | ok, **10 passed** |
| `cargo test --locked --lib log::endowment` | ok, **4 passed** |
| `cargo test --locked --lib log::provenance` | ok, **8 passed** |
| `cargo test --locked --all-targets` | ok, **285 passed**, 0 failed |
| `cargo test --locked --release --all-targets` | ok, **283 passed**, 0 failed |
| `cargo test --locked --test numeric_det confinement_of_the_float_domain` | ok |
| `cargo test --locked --test provenance` | ok, 6 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 |
| `cargo fmt --check` | 0 |
| `bash tests/lints.sh` | 0 — eleven source guards silent, all 60 method bans fire |
| `bash tests/toolchain.sh` | 0 — five checks, undisturbed by a top-level build script |
| `wc -c < <out>/events.jsonl` | 18560 (not 0) |
| `grep -c '"event":"endowment"' <out>/events.jsonl` | 220 |
| `grep -c '{"' <out>/events.jsonl` | 220 (= record count) |
| `grep -cE '(NaN\|Infinity\|null\|\{"[a-z_]+":\{)' <out>/events.jsonl` | 0 |
| `wc -l < <out>/provenance.csv` | 1 (not 0, not 2) |
| `head -1 <out>/provenance.csv` | `tick,agent,decision,input_a,input_b,outcome,rule` — 7 fields |
| `grep -c ',,' <out>/provenance.csv` | 0 |
| `grep -cE '"(seed\|config_sha256\|rustc)"' <out>/run_meta.json` | 3 |
| `grep -cE '"(duration_ms\|elapsed\|hostname\|pid\|cwd\|path)"' <out>/run_meta.json` | 0 |
| `grep -c '"ticks_completed": 3650' <out>/run_meta.json` | 1 |
| `ls <out>` | exactly `events.jsonl`, `provenance.csv`, `run_meta.json`, `ticks.csv` |
| `grep -vE '^[[:space:]]*//' src/log.rs \| grep -cE 'SCHEMA_VERSION…"[0-9]+\.[0-9]'` | 0 |
| `grep -c 'GRADE: PROJECT' config/PROVENANCE.md` | **5** — see *Deviations* #1; the anchored table-row form prints 4 |

## Next Steps

- **Plan 03-04** emits the schema from the bytes these three writers actually produce. The event stream's field order is declaration order and nothing nests, so a single ordering is recoverable from the file; the provenance header must come from `provenance_header()`, not from a second list.
- **Plan 03-05** owns the enforced exclusion of `run_meta.json` from the diff — including the clause that the excluded file must **exist**, since excluding a file that was never written enforces nothing — and the halted-run assertion that `ticks_completed` is 0 with `exit` `"violation"`. Its non-empty-before-comparing clause is the one that caught the original zero-byte artifacts; it should stay even though this plan removed the condition, because it is what stops the condition returning.
- **Phase 4** must make its dtype assertion conditional on a non-empty frame or read the dtype from the schema. See *The consequence handed forward to Phase 4*.

## Self-Check: PASSED

All five source artifacts exist on disk, all three task commits resolve in `git log`, and every
symbol this summary claims (`Event`, `endowment_events`, `ProvenanceRow`, `Decision`, `Rule`,
`provenance_header`, `header_of`, `SCHEMA_VERSION`, `RUN_META_FILE`, `Sink::event`,
`Sink::provenance`) is present in `src/log.rs`.
