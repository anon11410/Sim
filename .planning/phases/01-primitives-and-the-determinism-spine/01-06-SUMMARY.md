---
phase: 01-primitives-and-the-determinism-spine
plan: 06
subsystem: config
tags: [serde, toml, deny_unknown_fields, sha256, determinism, parts-per-million]

# Dependency graph
requires:
  - phase: 01-01
    provides: "The thin `src/config.rs` tracer — ConfigError, Params, Sim, MoneySection, load(), config_hash() — plus the CLI and the tracer end-to-end test"
  - phase: 01-04
    provides: "Money::from_cents / Money::checked_add / MoneyOverflow, the Result-returning money API the loader now routes the money stock through"
provides:
  - "The full six-table `Params` tree (sim, money, household, firm, bankruptcy, ownership) with `deny_unknown_fields` on every struct"
  - "41 typed config keys covering every simulation and economic parameter the model will read"
  - "`config/baseline.toml` carrying those 41 keys, economic values transcribed from the in-repo graded 37-row table"
  - "`ConfigError::MoneyRange` — an absurd money stock is a named configuration error, not a process abort"
  - "`tests/config_strict.rs` — the exhaustive per-key deletion proof plus 10 supporting strictness assertions"
affects: [01-08 provenance annotation, 02 tick loop, 04 acceptance harness, 06 goods market, 07 consumption, 09 price and wage rules, 11 calibration]

actuals:
  tokens: 6716
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Parts-per-million integers for every ratio and probability, keeping thresholds in the integer domain alongside the rng sampler API"
    - "`# CALIBRATED-IN: phase-11` trailing marker making a deferred calibration visible at the point of use, not only in a roadmap"
    - "Grep-shaped source assertions written as `#[test]`s inside the test binary rather than as shell greps beside it"

key-files:
  created:
    - tests/config_strict.rs
  modified:
    - src/config.rs
    - config/baseline.toml

key-decisions:
  - "The `MoneyRange` range check is `stock.checked_add(stock)` — the money stock must survive being added to itself, giving the conservation audit's intermediate sums a factor-of-two headroom, and reporting an absurd amount as a named error instead of aborting (T-1-03)"
  - "`ConfigError::Utf8` is retained alongside Io/Parse/MoneyRange: `load` reads raw bytes so the hash and the parse describe the same byte sequence, and that read needs a UTF-8 boundary of its own"
  - "The unit tests in `src/config.rs` pin the schema against an embedded full document rather than reading `config/baseline.toml`, so a later edit to the shipped parameter values cannot break the schema tests; the shipped file itself is exercised by `tests/config_strict.rs` and the tracer test"
  - "The table-reordering test builds its reordered copy textually, not by round-tripping through `toml::Value`, whose table map re-sorts keys and would silently undo the reordering"

patterns-established:
  - "Strictness is proved by deletion, not by grep: `every_key_is_required` removes each of the 41 leaf keys in turn and asserts each removal is rejected by name"
  - "Negative probes are run and reverted before commit — removing `deny_unknown_fields` from a nested struct, adding a serde default, and switching a field to an optional type were each verified to break the corresponding test"

requirements-completed: [CORE-10]

coverage:
  - id: D1
    description: "Every simulation and economic parameter loads from the TOML config; no key can default, proved exhaustively rather than by spot check"
    requirement: CORE-10
    verification:
      - kind: integration
        ref: "tests/config_strict.rs#every_key_is_required"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#no_serde_defaults_anywhere_in_src"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#no_optional_fields_in_the_config_schema"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#empty_config_is_rejected"
        status: pass
    human_judgment: false
  - id: D2
    description: "Unknown fields are denied on every struct, not only the root; a stray key inside a nested table and a stray table are each rejected by name"
    requirement: CORE-10
    verification:
      - kind: integration
        ref: "tests/config_strict.rs#unknown_key_inside_a_table_is_rejected"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#unknown_table_is_rejected"
        status: pass
      - kind: unit
        ref: "src/config.rs#config::tests::a_misspelled_key_inside_a_table_is_rejected"
        status: pass
    human_judgment: false
  - id: D3
    description: "A value of the wrong type is rejected rather than coerced — decimal into integer and quoted number into integer both fail by name"
    requirement: CORE-10
    verification:
      - kind: integration
        ref: "tests/config_strict.rs#float_where_int_is_not_coerced"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#string_where_int_is_not_coerced"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#removed_value_is_rejected"
        status: pass
    human_judgment: false
  - id: D4
    description: "The config hash is a lowercase hex digest over the file's raw bytes: stable across repeated computation, unchanged by nothing, changed by a table reorder or a single comment character"
    requirement: CORE-10
    verification:
      - kind: integration
        ref: "tests/config_strict.rs#hash_is_stable_across_repeated_computation"
        status: pass
      - kind: integration
        ref: "tests/config_strict.rs#key_order_does_not_change_params_but_does_change_the_hash"
        status: pass
      - kind: unit
        ref: "src/config.rs#config::tests::the_hash_is_stable_and_sensitive_to_one_comment_character"
        status: pass
    human_judgment: false
  - id: D5
    description: "The shipped config loads end to end through the built binary and the library, with 41 keys in six tables"
    requirement: CORE-10
    verification:
      - kind: e2e
        ref: "cargo run -- --config config/baseline.toml --seed 7 --out $(mktemp -d)"
        status: pass
      - kind: e2e
        ref: "tests/tracer_end_to_end.rs#runs_end_to_end"
        status: pass
    human_judgment: false
  - id: D6
    description: "The economic values in config/baseline.toml are the correct transcription of the graded 37-row parameter table, and the ten Phase 11 deferrals are the right set to defer"
    verification: []
    human_judgment: true
    rationale: "Transcription fidelity and the choice of which initial conditions to defer to Phase 11 are judgment calls no test can make. D-20 forbids an agent re-deriving these from memory, so the only check is a human reading .planning/research/SUMMARY.md lines 171-209 against the shipped file. Plan 01-08's provenance pass and REQUIREMENTS.md CORE-11 clause (b) are where that judgment is formally recorded."

# Metrics
duration: 4min
completed: 2026-08-31
status: complete
---

# Phase 01 Plan 06: Strict Typed Configuration Summary

**The config file is now the whole input: 41 typed keys in six tables, `deny_unknown_fields` on every struct, and an exhaustive per-key deletion loop that proves not one of them can quietly default.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-08-30T23:57:18Z
- **Completed:** 2026-08-31T00:01:27Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- **CORE-10 is closed by construction and by proof.** `every_key_is_required` reads the shipped config, enumerates all 41 leaf keys, and for each one deletes it, re-serialises and asserts the parse fails with ``missing field `<name>` ``. This is the check `01-RESEARCH.md` Pitfall 7 showed the ROADMAP's serde-default grep cannot make: an optional field type defaults to absent with no attribute to find.
- **The parameter schema widened from 2 tables to 6** — `sim`, `money`, `household`, `firm`, `bankruptcy`, `ownership` — each carrying `#[serde(deny_unknown_fields)]`. A negative probe confirmed the nested attributes are load-bearing: deleting it from `Sim` alone makes the misspelled-key test fail.
- **Every ratio and probability enters as a parts-per-million integer** (18 `_ppm` keys), so no threshold parameter needs the float domain. `initial_expected_demand` is the single floating-point field in the entire configuration, exactly as D-11 permits and D-13 requires, and it is still the only line in `src/config.rs` naming a float type — `tests/numeric_det.rs::confinement_of_the_float_domain` continues to pass unchanged.
- **The money stock now crosses into the money domain through the checked API.** `load` routes `total_money_cents` through `Money::checked_add` and surfaces failure as `ConfigError::MoneyRange`, closing threat T-1-03: an absurd supplied amount is a named configuration error rather than a process abort.
- **The two grep-shaped checks moved inside the test binary.** `no_serde_defaults_anywhere_in_src` and `no_optional_fields_in_the_config_schema` run under `cargo test`, so they cannot be skipped by running the suite without a lint script.

## Task Commits

Each task was committed atomically:

1. **Task 1: The full parameter schema, strict on every struct** (TDD)
   - `620ee80` (test) — RED: 9 unit tests added, 6 failing on the two-table schema
   - `fb81eae` (feat) — GREEN: six tables, `deny_unknown_fields` throughout, `ConfigError::MoneyRange`
2. **Task 2: Ship the parameter file the schema declares** — `d63e946` (feat)
3. **Task 3: The exhaustive proof that no key can default** — `2c069c6` (test)

**Plan metadata:** see the final `docs(01-06)` commit.

## Files Created/Modified

- `src/config.rs` (108 → 388 lines) — the full `Params` tree; `Sim`, `MoneySection`, `Household`, `Firm`, `Bankruptcy`, `Ownership`; `ConfigError::MoneyRange`; a 9-test `#[cfg(test)] mod tests` pinning the schema against an embedded full document.
- `config/baseline.toml` (14 → 69 lines) — 41 keys in six tables, economic values transcribed from `.planning/research/SUMMARY.md` lines 171-209, ten Phase 11 deferrals marked `# CALIBRATED-IN: phase-11`.
- `tests/config_strict.rs` (new, 292 lines) — 11 tests: the exhaustive deletion loop, unknown key / unknown table / empty file / removed value / two type-coercion rejections, the reorder-versus-hash pair, hash stability, and the two source assertions.

## Verification Results

| Gate | Result |
|---|---|
| `cargo test --lib config::` | 9 passed |
| `cargo test --test config_strict` | 11 passed (debug and `--release`) |
| `cargo test` (whole suite, debug) | 106 passed, 0 failed |
| `cargo test --release` | 104 passed, 0 failed |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo run -- --config config/baseline.toml --seed 7 --out …` | exit 0, prints the tracer line |
| `grep -rc 'serde(default' src/` | 0 for every file |
| `grep -cE 'Option[<]' src/config.rs` | 0 |
| `grep -c 'deny_unknown_fields' src/config.rs` | 9 (7 attributes + 2 doc mentions) |
| Float type names in `src/config.rs` | 1 line, `pub initial_expected_demand: f64` |
| `grep -c 'CALIBRATED-IN: phase-11' config/baseline.toml` | 11 (10 keys + 1 header mention) |
| `grep -c '_ppm = ' config/baseline.toml` | 18 |

Test count rose from 86 to 106 in debug (9 new unit tests, 11 new integration tests).

### Negative probes (each run, observed to fail, then reverted before commit)

| Probe | Expected | Observed |
|---|---|---|
| Remove `deny_unknown_fields` from `Sim` only | the nested unknown-key test fails | `a_misspelled_key_inside_a_table_is_rejected` FAILED |
| Add `#[serde(default)]` to `Ownership::firms_per_owner` | both the deletion loop and the attribute assertion fail | `every_key_is_required` and `no_serde_defaults_anywhere_in_src` both FAILED |
| Change `Ownership::firms_per_owner` to an optional type | both the deletion loop and the optional-type assertion fail | `every_key_is_required` and `no_optional_fields_in_the_config_schema` both FAILED |
| Delete `month_days` from the shipped config | the binary exits non-zero naming the key | exit 1, `TOML parse error` naming the missing field |

`git diff --stat src/config.rs` was empty after each revert.

## Decisions Made

- **`MoneyRange`'s check is `stock.checked_add(stock)`.** `Money::from_cents` is infallible, so "route the config amount through the `Result`-returning API" needs a predicate with teeth. Requiring the stock to survive doubling gives the conservation audit's intermediate sums a factor-of-two headroom and makes an absurd `total_money_cents` a named `ConfigError`, not a panic.
- **`ConfigError::Utf8` stays.** The plan's artifact list names `Io`, `Parse`, `MoneyRange`. `Utf8` is a fourth variant inherited from 01-01 and it is structurally necessary: `load` reads raw bytes so the hashed and parsed byte sequences are provably the same, and that read needs a UTF-8 boundary. Removing it would mean either hashing a `String` or an `unwrap`.
- **The `src/config.rs` unit tests pin the schema against an embedded document, not the shipped file.** Keeping them independent of `config/baseline.toml` means a future value edit cannot break a schema test, and it kept Task 1's commits self-contained. The shipped file is covered by `tests/config_strict.rs` (all 11 tests read it) and by the tracer end-to-end test.
- **The reorder test rebuilds the document textually.** Round-tripping through `toml::Value` re-sorts table keys through its `BTreeMap`, which would silently undo the reordering and make the test pass for the wrong reason.
- **Provisional initial conditions were chosen for internal consistency at 200 households and 20 firms:** an initial wage of 6300 cents/month against productivity 3 and a 21-day month gives a marginal cost of 100 cents/unit, and an initial price of 105 cents sits inside the `[1.025, 1.15] × mc` band the same file declares. Household liquidity 5000 × 200 plus firm liquidity 50000 × 20 sums exactly to the 2,000,000-cent stock. All ten are marked `# CALIBRATED-IN: phase-11`.

## Deviations from Plan

None — plan executed as written. Two clarifications on how the plan's text was read, neither changing scope:

1. **Task 1's TDD RED/GREEN split produced two commits** (`620ee80`, `fb81eae`), as the executor's TDD flow requires. The RED commit compiles: the tests were written to reach the loader through `toml::from_str::<Params>` and string assertions only, so no test referenced a type that did not yet exist. The one test needing `PartialEq` (`parsing_the_same_document_twice_is_equal`) used a `Debug`-string comparison in RED and was strengthened to a `PartialEq` comparison in GREEN.

2. **The plan's behavior line "parsing the shipped config twice yields equal `Params` values" is satisfied at the integration layer**, by `key_order_does_not_change_params_but_does_change_the_hash`, which parses the shipped file twice (straight and reordered) and asserts equality. The corresponding unit test uses the embedded document, per the decision above. Task 3's test count is exactly the 11 the plan specifies.

## Issues Encountered

- **Task 1 leaves `config/baseline.toml` transiently unparseable.** The plan widens the schema before the file, so between `fb81eae` and `d63e946` the tracer test is red. This is inherent to the plan's task split and lasted 40 seconds; the branch tip is green in both profiles. Noted rather than worked around, because reordering the tasks would have meant committing a config file no schema accepted — the same problem in the other direction.
- **The Task 3 commit message initially said 44 leaf keys; the true count is 41.** Amended on the tip commit before any further work.

## Known Stubs

None. Every declared key is present in the shipped file and every field is consumed by the loader. The ten `# CALIBRATED-IN: phase-11` values are provisional, not stubbed — they load, parse and produce a working run today, and REQUIREMENTS.md CAL-01 and CAL-02 own their calibration.

## Threat Flags

None. All five threats in the plan's register are addressed as planned: T-1-19 (nested `deny_unknown_fields` + the deletion loop + both source assertions), T-1-03 (`ConfigError::MoneyRange`), T-1-20 (both coercion rejections asserted by error substring), T-1-21 (hash over raw bytes, proved sensitive to a table reorder and to one comment character), T-1-04 (accepted; no config value contributes to a path — `--out` is joined only with fixed filenames).

No new security-relevant surface was introduced. The one new trust boundary crossing, config money → `Money`, is the mitigation for T-1-03 rather than a new exposure.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 01-08 is unblocked and its input is exactly as expected.** `config/baseline.toml` carries the 41 keys 01-08 will annotate with `# GRADE:` blocks, and deliberately carries no grade annotations yet. The file header already states that a comment edit changes the config hash, which is the property 01-08's provenance comments depend on.
- **Plan 01-07 is unaffected.** The two pre-existing `cargo fmt --check` failures in `src/money.rs` and `tests/tracer_end_to_end.rs` are untouched and still 01-07's to fix; `src/config.rs` and `tests/config_strict.rs` are both rustfmt-clean.
- **CORE-10 is complete.** Both it and CORE-11 clause (a) were shared with 01-05 and 01-08; CORE-11 remains open pending 01-08.
- **Concern for Phase 11 (CAL-01/CAL-02), recorded now while the arithmetic is fresh:** at the shipped provisional values, expected demand of 330 units/firm-month against a productivity of 3 units/worker-day over 21 days implies roughly 5 workers per firm, or about 105 of 200 households employed. That is far outside any plausible unemployment band. The values are internally consistent and the model runs, but they are not a plausible starting economy — which is precisely why they are marked deferred rather than presented as calibrated.
- **Open question for Phase 7, surfaced by writing the schema:** `initial_expected_demand` is a per-month quantity in these values, while `productivity_units_per_worker_day` is per day and `demand_smoothing_ppm` updates per period. The cadence of the demand expectation is not yet pinned anywhere, and `01-RESEARCH.md` warns that evaluating the inventory band against a different demand notion than the price rule uses is a subtle desynchronisation bug. Phase 7 should fix the cadence explicitly before consuming either key.

---
*Phase: 01-primitives-and-the-determinism-spine*
*Completed: 2026-08-31*

## Self-Check: PASSED

All four artefacts exist on disk (`src/config.rs`, `config/baseline.toml`, `tests/config_strict.rs`, this SUMMARY) and all four task commits (`620ee80`, `fb81eae`, `d63e946`, `2c069c6`) are present in the git history.
