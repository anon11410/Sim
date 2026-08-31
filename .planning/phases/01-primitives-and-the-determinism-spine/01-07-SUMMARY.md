---
phase: 01-primitives-and-the-determinism-spine
plan: 07
subsystem: infra
tags: [clippy, lints, ci, github-actions, determinism, rustfmt, negative-testing]

# Dependency graph
requires:
  - phase: 01-01
    provides: "Cargo.toml [lints.clippy] levels at deny, rust-toolchain.toml pinning 1.94.1 with clippy+rustfmt, tests/toolchain.sh and its shell conventions"
  - phase: 01-03
    provides: "src/money.rs — one of the two files the deferred formatting item named"
  - phase: 01-04
    provides: "src/rng.rs — the ChaCha8Rng sub-stream machinery whose non-portable alternatives clause (b) bans by use"
  - phase: 01-05
    provides: "src/numeric.rs — the fractional power built from sqrt, which is why the powf ban needs no exemption in Phase 7; and the module-level half of the float ban in tests/numeric_det.rs"
  - phase: 01-06
    provides: "src/config.rs and config/baseline.toml — part of the clean tree the gate must pass over"
  - phase: 01-08
    provides: "config/PROVENANCE.md and tests/provenance.rs — likewise"
provides:
  - "clippy.toml: 5 disallowed-types and 68 disallowed-methods entries, generated from the pinned toolchain's own std source rather than typed, every entry carrying a reason"
  - "tests/lints.sh: the negative test — injects a known hazard, observes the gate block it, restores the tree"
  - "tests/lint-probes/float_ban_probe.rs.txt: 58 marked call sites, one per ban entry that resolves on stable, both float widths"
  - "tests/lint-probes/hazard.rs.txt: the known-bad subject, one hazard per lint"
  - ".github/workflows/ci.yml: the gate run unattended — both build profiles, clippy with the flags that lint every target, fmt, and both guard scripts"
  - "A rustfmt-clean tree, with cargo fmt --check in CI to keep it that way"
affects: [all later phases, Phase 7 MKT-01, Phase 2 invariants, any phase adding a dependency or a float call site]

actuals:
  tokens: 9160
  tasks: 3
  commits: 4

tech-stack:
  added: [GitHub Actions]
  patterns:
    - "Ban lists generated from the toolchain's own source, never typed from memory"
    - "A lint is proved by a negative test that watches it block, not by the configuration existing"
    - "Diagnostic count compared against probe call-site count, both computed, to convert clippy's silence about unresolvable paths into a failure"
    - "Escape hatches asserted absent by grep, because the lint cannot see through an alias behind an exemption"

key-files:
  created:
    - clippy.toml
    - tests/lints.sh
    - tests/lint-probes/float_ban_probe.rs.txt
    - tests/lint-probes/hazard.rs.txt
    - .github/workflows/ci.yml
  modified:
    - src/money.rs
    - tests/tracer_end_to_end.rs
    - .planning/phases/01-primitives-and-the-determinism-spine/deferred-items.md

key-decisions:
  - "The float ban list is derived by parsing the pinned toolchain's std/core f64.rs and f32.rs for the standard library's unspecified-precision doc marker and taking the next `pub fn`. Counts found locally — 31 in std + 2 in core = 33 distinct per width — were confirmed rather than assumed, and are recorded in the generated file's header."
  - "f32 declares cbrt in BOTH core (unstable) and std (stable); the stable declaration is the callable one, so the distinct f32 count is 33, matching f64, rather than the 34 a naive sum would give. The generator dedupes and records why."
  - "Check 2 asserts each ban list fired SEPARATELY rather than asserting only that the build failed. A bare failure assertion stays green after one list is deleted, because the other hazard in the same file still fails the build — the check would then be reporting the wrong lint's health. This is a strengthening over the plan's text, and it is what makes the plan's own disallowed-types-deletion acceptance criterion actually hold."
  - "The disallowed-types exemption assertion is scoped to tracked *.rs files, not the whole repository. The attribute has effect only in Rust source; CLAUDE.md and the planning documents quote it in prose and in code examples, and matching those would be matching a description of the hole rather than the hole."
  - "cargo fmt --check was added to CI beyond the plan's enumerated steps, so the phase's deferred formatting item closes permanently instead of being able to re-drift."

patterns-established:
  - "Generated-not-typed: any list long enough to hide a typo is derived from the authority and carries a header naming the authority, the marker and the count actually found."
  - "Negative testing of gates: a guard script injects the hazard, observes the block, and restores the tree under a trap that fires on every exit path."
  - "Grep-and-lint in pairs: where a lint has a verified blind spot (aliases, unresolvable paths, the unlinted test directory), a source assertion or a count comparison covers it, and each half is documented as insufficient alone."

requirements-completed: [CORE-07, CORE-03]

coverage:
  - id: D1
    description: "clippy.toml bans the hashed collections and the non-deterministic float methods, generated from the pinned toolchain's own standard-library source with the count found locally recorded in its header"
    requirement: CORE-07
    verification:
      - kind: integration
        ref: "cargo clippy --all-targets --all-features -- -D warnings (clean tree)"
        status: pass
      - kind: integration
        ref: "tests/lints.sh#check 2 — injected hazard produces both a disallowed-type and a disallowed-method diagnostic"
        status: pass
    human_judgment: false
  - id: D2
    description: "The lint gate is proved to BLOCK, not merely to exist: a known hazard is injected into tests/ — the directory plain cargo clippy is blind to — and observed to fail the build, then removed"
    requirement: CORE-07
    verification:
      - kind: integration
        ref: "bash tests/lints.sh (exits 0, final line starts OK:)"
        status: pass
      - kind: integration
        ref: "manual counter-experiment: with the hazard in tests/, `cargo clippy` exits 0 and `cargo clippy --all-targets --all-features` exits 101"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every configured float path is proved to fire: the probe's 58 marked call sites are compared against the diagnostic count, turning clippy's silence about an unresolvable path into a failure"
    requirement: CORE-07
    verification:
      - kind: integration
        ref: "tests/lints.sh#check 3 — 58 marked call sites, 58 diagnostics, both computed"
        status: pass
      - kind: integration
        ref: "manual counter-experiment: f64::log2 corrupted to f64::log_2 gives 57 vs 58 and a red script, with no clippy warning about the bad path"
        status: pass
    human_judgment: false
  - id: D4
    description: "The alias-shaped escape hatch, the disallowed-types exemption, the point-lookup wrapper (D-06) and the non-portable generator names are all asserted absent under src/"
    requirement: CORE-03
    verification:
      - kind: integration
        ref: "tests/lints.sh#check 4 — four assertions"
        status: pass
      - kind: integration
        ref: "manual counter-experiment: an alias behind #[allow(clippy::disallowed_types)] leaves clippy at exit 0 while check 4a names the file and line"
        status: pass
      - kind: integration
        ref: "manual counter-experiment: a SmallRng mention under src/ turns check 4d red"
        status: pass
    human_judgment: false
  - id: D5
    description: "CI runs the gate unattended with the flags that lint every target, both build profiles, formatting, and both guard scripts, naming no Rust version literal"
    requirement: CORE-07
    verification:
      - kind: other
        ref: "python3 -c \"import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))\" — parses"
        status: pass
      - kind: other
        ref: "all seven step commands run locally in the listed order, each exit 0"
        status: pass
      - kind: other
        ref: "grep -c '1\\.94' .github/workflows/ci.yml == 0 — the pin has exactly one home"
        status: pass
    human_judgment: true
    rationale: "The workflow's step commands were each verified locally and the file parses, but the workflow has never been executed by GitHub Actions in this environment — the runner image's rustup behaviour on first `rustup show active-toolchain` is the one link in the chain that local execution cannot prove. A human should confirm the first CI run goes green."

duration: 18 min
completed: 2026-08-31
status: complete
---

# Phase 01 Plan 07: The Determinism Lint Wall Summary

**The determinism bans stopped being prose: a `clippy.toml` generated from the pinned toolchain's own standard-library source, a negative test that injects a hazard and watches the gate block it, and a CI job that runs both — with all three of research's enforcement holes reproduced first-hand and each observed to be closed.**

## Performance

- **Duration:** 18 min
- **Started:** 2026-08-31T00:07:00Z (approx.)
- **Completed:** 2026-08-31T00:25:00Z
- **Tasks:** 3 of 3
- **Files modified:** 8 (5 created, 3 modified)

## Accomplishments

- **The ban lists are derived, not typed.** A generator reads the pinned toolchain's own `std/src/num/{f64,f32}.rs` and `core/src/num/{f64,f32}.rs`, finds every method carrying the standard library's "The precision of this function is non-deterministic" doc marker, and takes the next `pub fn`. It found **31 in std + 2 in core = 33 distinct methods per width**, confirming research's count from the local source rather than assuming it. The header of the generated `clippy.toml` records the counts actually found, the toolchain version, the source paths and the marker sentence.
- **The gate is observed to block, four ways.** `tests/lints.sh` passes the clean tree, injects a hazard into `tests/` and watches both ban lists fire on it, drives the 58-call-site probe and compares diagnostics to call sites, and asserts the four escape hatches absent — restoring the tree under a `trap` that fires on every exit path.
- **All three enforcement holes were reproduced first-hand this run, then closed.** Each was verified as a live failure before the closing check was accepted (see *Counter-experiments* below).
- **CI runs the gate unattended** with the flags that lint every target, in both build profiles, plus formatting and both guard scripts — and names no Rust version, so `rust-toolchain.toml` remains the single home of the pin.
- **The phase's one outstanding deferred item is closed.** `cargo fmt --check` now passes on the whole tree and is a CI step.

## Task Commits

1. **Task 1: Generate the ban lists from the pinned toolchain's own standard-library source** — `fbe4b11` (feat)
2. **Task 2: The negative test that proves the wall blocks** — `94542c6` (test)
3. **Deferred-item closure: bring the two remaining files to rustfmt-clean** — `bc6f16d` (style)
4. **Task 3: Run the gate in CI with the flags that lint every target** — `6d10d1a` (ci)

## Files Created/Modified

- `clippy.toml` — 5 `disallowed-types` (both hashed collections, `SmallRng`, both Xoshiro generators) and 68 `disallowed-methods` (33 float methods × 2 widths + `SystemTime::now` + `Instant::now`). Every entry carries a `path` and a `reason`. No entry for `sqrt`, `mul_add`, `abs`, `copysign`, `floor`, `ceil`, `round`, `trunc` or `rem_euclid` — IEEE-754 gives each a single correctly-rounded result.
- `tests/lint-probes/float_ban_probe.rs.txt` — 58 marked call sites (`// BANNEDCALL`, a token that appears nowhere else in the file so counting it is exact), one per entry that resolves on stable, on both widths. A `.txt` file, so cargo never compiles it during a normal build.
- `tests/lint-probes/hazard.rs.txt` — the known-bad subject: one hashed-collection return type, one `powf` call.
- `tests/lints.sh` — the negative test, in `tests/toolchain.sh`'s strict-mode style with its `fail`/`OK:` conventions and its grep exit-code discipline. Executable.
- `.github/workflows/ci.yml` — one job on push and pull request: checkout, install by honouring the toolchain pin, `cargo build --locked`, `cargo test --locked --all-targets`, `cargo test --locked --release --all-targets`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `bash tests/toolchain.sh`, `bash tests/lints.sh`.
- `src/money.rs`, `tests/tracer_end_to_end.rs` — rustfmt normalization only (whitespace and layout).
- `.planning/phases/.../deferred-items.md` — the formatting item marked closed with its commits.

## Counter-experiments (the holes, reproduced then closed)

Every check below was first observed to FAIL for the right reason before being accepted. A gate that has only ever been seen green has not been shown to work.

| # | Corruption | Observed | Closed by |
|---|---|---|---|
| A | Delete the `disallowed-types` block from `clippy.toml` | script red: "the injected hashed collection produced no disallowed-type diagnostic" | check 2, asserting each list separately |
| B | Corrupt `f64::log2` to the unresolvable `f64::log_2` | script red, printing `58` marked vs `57` fired — **and clippy emitted no warning whatsoever about the bad path** | check 3, the computed count comparison |
| C | Add `pub type LookupMap<K,V> = HashMap<K,V>` to `src/lib.rs` | script red at check 1 (a bare alias still trips the lint at its definition) | check 1 |
| C′ | The same alias **behind `#[allow(clippy::disallowed_types)]`** — the verified hole | **`cargo clippy` exits 0: the lint is completely blind.** Script red at check 4a, naming `src/lib.rs:16` | check 4a, a grep the lint cannot substitute for |
| D | A `SmallRng` mention under `src/` | script red at check 4d, naming the file and line | check 4d — CORE-03 clause (b) |
| E | `src/lookup.rs` created | script red at check 4c | check 4c — D-06 |
| F | The hazard sitting in `tests/` | **`cargo clippy` → exit 0 (the trap); `cargo clippy --all-targets --all-features` → exit 101** | the flags, which are never abbreviated in `tests/lints.sh` or in CI |

C′ and F are the two that matter most: in both, the naive invocation reports a perfectly healthy exit 0 over a tree that is broken.

## Decisions Made

1. **The f32 `cbrt` double declaration.** `cbrt` is declared in both `core/src/num/f32.rs` (unstable) and `std/src/num/f32.rs` (stable). A naive sum gives 34 methods for f32 against 33 for f64; deduping with the stable declaration winning gives 33, matching f64. The generator dedupes, asserts the two widths carry identical marker sets, and records the reason in the file header — so a future reader does not "fix" the count back to 34.
2. **Check 2 asserts each ban list separately.** The plan's text asked only that the injected hazard fail the build. But the hazard file carries one hazard per lint, so deleting the `disallowed-types` block would leave the `powf` hazard still failing the build and the check still green — the plan's own acceptance criterion ("deleting the `disallowed-types` block makes the script exit non-zero at check 2") would not have held. Check 2 now requires both a `use of a disallowed type` and a `use of a disallowed method` diagnostic. Experiment A above confirms it.
3. **The exemption assertion is scoped to tracked `*.rs` files.** A lint-exemption attribute has effect only in Rust source. `CLAUDE.md`, `01-RESEARCH.md` and `01-07-PLAN.md` all quote `allow(clippy::disallowed_types)` in prose or code examples; matching those would be matching a description of the hole, and would make the check unpassable for a reason unrelated to code health.
4. **`cargo fmt --check` joins CI.** Beyond the plan's enumerated steps, but the deferred-items ledger named this plan as the owner of the formatting item, and adding the check is the difference between closing the item and merely clearing it once.
5. **The generator script itself is not committed.** The plan's `files_modified` names five files and the generator is not among them. Its derivation is recorded instead in `clippy.toml`'s header — toolchain version, exact source paths, the marker sentence, and the counts found — which is what a future regeneration needs. Flagged below as a reviewable judgement call.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] The `rust-src` component was not installed, so the standard-library source the task must read did not exist**

- **Found during:** Task 1
- **Issue:** `$(rustc --print sysroot)/lib/rustlib/src/rust/library/std/src/num/f64.rs` did not exist. Research had read it after a `rustup component add rust-src`, but that component was not present in this environment. The task's central instruction — derive the list from the local source rather than typing it — was unexecutable.
- **Fix:** `rustup component add rust-src --toolchain 1.94.1-x86_64-unknown-linux-gnu`. This installs source for the already-pinned toolchain; it adds no dependency to the crate, changes no build input, and leaves `Cargo.lock` and `rust-toolchain.toml` untouched. It is not a package-manager install of a third-party package, so the Rule 3 package exclusion does not apply.
- **Files modified:** none (toolchain component only)
- **Verification:** the source appeared and the extraction produced exactly the counts research reported (31 + 2 = 33 per width, 4 unstable), which is itself strong evidence the right file was read.
- **Committed in:** n/a — no repository file changed.

**2. [Rule 2 - Missing critical] Check 2 could not distinguish which ban list was enforcing**

- **Found during:** Task 2
- **Issue:** As first written, check 2 asserted only a non-zero exit from the hazard run. Because the hazard file carries one hazard per lint, deleting either ban list left the other hazard failing the build and check 2 green — so the check could report a healthy gate over a half-dismantled one. The plan's own acceptance criterion for the `disallowed-types` deletion depended on this working.
- **Fix:** check 2 now captures the clippy output and requires both a `use of a disallowed type` and a `use of a disallowed method` diagnostic, failing with a specific message naming which list is not enforcing.
- **Files modified:** `tests/lints.sh`
- **Verification:** experiment A — deleting the `disallowed-types` block turns the script red with "the disallowed-types list is not enforcing".
- **Committed in:** `94542c6`

**3. [Rule 3 - Blocker] `set -e` aborted the script on the expected-failure runs**

- **Found during:** Task 2
- **Issue:** `HAZARD_OUT=$(cargo clippy ... )` under `set -euo pipefail` aborts on clippy's exit 101 — which is the outcome the check is trying to observe. The script exited 101 after check 1 with no diagnostic message.
- **Fix:** both places where a non-zero exit is the expected outcome now capture status explicitly with the `set +e` / status / `set -e` idiom `tests/toolchain.sh` already uses, and examine it. No `|| true` was used, in keeping with the no-error-suppressing-fallbacks rule both guard scripts follow.
- **Files modified:** `tests/lints.sh`
- **Verification:** the script runs to completion and all six counter-experiments produce the intended failure message.
- **Committed in:** `94542c6`

**4. [Rule 2 - Missing critical] The phase's outstanding deferred formatting item**

- **Found during:** Task 3 preparation
- **Issue:** `cargo fmt --check` failed on `src/money.rs` (10 diffs) and `tests/tracer_end_to_end.rs` (4 diffs). Plans 01-04, 01-05, 01-06 and 01-08 each left these alone under the scope boundary and logged this plan as the owner, because this is the plan that builds the CI wall.
- **Fix:** one repo-wide `cargo fmt` (whitespace and layout only — derive lists broken one-per-line, struct literals expanded; exactly the two files named, no others), plus a `cargo fmt --check` step in the CI workflow so the item cannot re-open.
- **Files modified:** `src/money.rs`, `tests/tracer_end_to_end.rs`, `.github/workflows/ci.yml`, `deferred-items.md`
- **Verification:** `cargo fmt --check` clean; 112 debug / 110 release tests still pass; clippy still clean.
- **Committed in:** `bc6f16d` (the formatting) and `6d10d1a` (the CI step)

---

**Total deviations:** 4 auto-fixed (2 × Rule 3 blocker, 2 × Rule 2 missing-critical).
**Impact on plan:** No scope creep. Deviations 1 and 3 were prerequisites for executing the plan as written; deviation 2 makes one of the plan's own acceptance criteria actually hold; deviation 4 closes an item the phase ledger explicitly assigned to this plan. No determinism ban was weakened, exempted, aliased around or narrowed — the plan's single prohibition is intact, and the one place the pressure could have arisen (Phase 7's `powf`) already has its replacement in `src/numeric.rs`.

## Issues Encountered

None unresolved. Two transient ones, both resolved and recorded above as deviations 1 and 3.

One item is flagged for review rather than resolved:

- **The generator script is not committed** (decision 5). The plan's `files_modified` does not include it, so committing it would have widened the file set; the derivation is recorded in `clippy.toml`'s header instead. The cost is that regenerating the list in a future phase means re-writing the extractor from that header rather than re-running a committed script. If a reviewer prefers the script committed, it is ~90 lines and reconstructible from the header. Worth a decision at verify-phase, since the "generated, not typed" property is only as durable as the ability to regenerate.

## Requirements

- **CORE-07** — complete. The lint configuration bans the hashed collections and the non-deterministic float methods, and CI fails the build when one is introduced. Proved by a negative test, not by the configuration's existence.
- **CORE-03 clause (b)** — complete. `SmallRng` and both Xoshiro generators carry `disallowed-types` entries, and `tests/lints.sh` check 4d asserts the names appear nowhere under `src/`. Usage-based enforcement is required because rand 0.10.2 makes the types unconditional and they cannot be removed from the dependency graph. Clause (a) was satisfied in plan 01-02.
- **CORE-09** — checked, not owned. The prior-wave note warned that `REQUIREMENTS.md` might still carry the unsatisfiable broad `cargo tree | grep getrandom` form. It does not: the current text reads "no `rayon` dependency and no `-C target-cpu=native`", with no getrandom clause, and it is already marked Complete. `tests/toolchain.sh` asserts the correct `cargo tree --edges normal` form. **Nothing open; no action needed.**

## Known Stubs

None. No hardcoded empty value, placeholder, TODO or FIXME was introduced.

One documented exclusion, which is not a stub: the four unstable float methods (`gamma`, `ln_gamma`, `erf`, `erfc`) keep their `clippy.toml` entries but are absent from the probe, because they cannot be called on a stable toolchain at all. This is the plan's own flagged assumption, recorded in `01-07-PLAN.md` under "Flagged assumptions". It is a hole only if a future toolchain stabilises one of them without the probe being regenerated — at which point check 3's count comparison would still pass (58 = 58) while a stabilised method went unexercised. Regenerating the list on any toolchain bump is therefore not optional, and the file header says so.

## Threat Flags

None. This plan adds no network endpoint, auth path, file access pattern or schema change. The five threats in its register (T-1-22, T-1-23, T-1-24, T-1-14, T-1-25) are each mitigated and each observed to be mitigated — see the counter-experiments table, which maps to them one-for-one.

## Next Phase Readiness

Phase 1 is complete: this was the last of eight plans. The full suite is green in both profiles (112 debug / 110 release), clippy is clean under the new lists, both guard scripts pass, the tree is rustfmt-clean, and no deferred items remain open in this phase.

Two things a later phase must not forget, both recorded in `clippy.toml`'s header:

1. **Regenerate the list on any toolchain bump.** A new compiler may add or stabilise a method carrying the marker. The count comparison in check 3 cannot detect a method that exists but is not configured.
2. **The ban is the gate.** If a later phase needs a banned construct, the correct move is a primitive that does not need it — as Phase 1 already did for the fractional power in `src/numeric.rs` — not an exemption. `tests/lints.sh` check 4b will fail the build if a disallowed-types exemption is added anywhere in the crate's Rust source.

One item for the human at verify-phase: the CI workflow has never actually run on GitHub Actions from this environment. Every step command was run locally in order and each exited 0, and the file parses as YAML, but the runner image's rustup behaviour on first `rustup show active-toolchain` is the one link local execution cannot prove. Flagged as `human_judgment: true` on coverage item D5.

---
*Phase: 01-primitives-and-the-determinism-spine*
*Completed: 2026-08-31*

## Self-Check: PASSED

All five created files exist on disk; all four task commits (`fbe4b11`, `94542c6`, `bc6f16d`, `6d10d1a`) are present in git; `tests/lints.sh` is tracked with mode 100755. Every task acceptance criterion and every plan-level verification command was re-run at close-out and passed: clippy clean on the committed tree, `bash tests/lints.sh` exits 0 with its `OK:` line and leaves the tree unmodified, all four deliberate corruptions turn it red, and all seven CI step commands exit 0 locally.
