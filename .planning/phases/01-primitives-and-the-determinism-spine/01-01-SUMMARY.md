---
phase: 01-primitives-and-the-determinism-spine
plan: 01
subsystem: infra
tags: [rust, cargo, chacha8rng, rand, serde, toml, sha2, clap, thiserror, anyhow, determinism, integer-money]

# Dependency graph
requires: []
provides:
  - "Single-crate skeleton: `src/lib.rs` library root + thin `src/main.rs` CLI (CORE-08)"
  - "`Money(i64)` newtype over a private cents field; `from_cents` the only constructor; `Add` panics in every profile"
  - "`config::load` — reads raw bytes once, SHA-256s those bytes, parses those same bytes; `deny_unknown_fields` on every struct"
  - "`config::config_hash(&[u8]) -> String` — lowercase hex, built byte-by-byte so it survives a future sha2 major bump"
  - "`rng::Rngs` facade over `ChaCha8Rng` with the `tick:24 | agent:24 | purpose:16` bit-packed sub-stream key"
  - "`rng::Stream::below` — exactly one 64-bit draw, multiply-high, no rejection loop"
  - "`rng::Purpose` — `#[repr(u16)]`, hand-assigned append-only discriminants (`TracerProbe = 1`)"
  - "Pinned toolchain 1.94.1 and committed `Cargo.lock` (CORE-09)"
  - "`[profile.release] overflow-checks = true`, proved load-bearing by a release test (CORE-02)"
  - "`tests/toolchain.sh` — the reproducibility contract as an executable assertion"
affects: [01-02, 01-03, 01-04, 01-05, 01-06, 01-07, 01-08, phase-02-ledger, phase-03-tick-pipeline]

actuals:
  tokens: 10206
  tasks: 3
  commits: 3

tech-stack:
  added:
    - "rand 0.10.2 (default-features = false, features = [\"std\", \"chacha\"])"
    - "serde 1.0.229 (derive)"
    - "toml 1.1.4"
    - "sha2 0.10.9"
    - "thiserror 2.0.20"
    - "clap 4.6.6 (derive)"
    - "anyhow 1.0.104"
    - "proptest 1.11.0 (dev)"
  patterns:
    - "Tracer-first: one thin production-quality path through every layer before any layer widens"
    - "Bit-packed RNG sub-stream keys — bijective, so distinct tuples give distinct nonces by arithmetic, not by collision resistance"
    - "Hash bytes, never a Rust value — the config digest is over the raw file bytes"
    - "Private-field newtype as the construction guard (Money cannot be conjured outside its module)"
    - "Split overflow API: operators panic in every profile; the named Result API is for config ingestion"
    - "Error plumbing split: thiserror in the library, anyhow confined to main.rs"
    - "Build-tooling invariants asserted by an executable script, not by convention"

key-files:
  created:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - src/lib.rs
    - src/main.rs
    - src/money.rs
    - src/config.rs
    - src/rng.rs
    - config/baseline.toml
    - tests/tracer_end_to_end.rs
    - tests/toolchain.sh
  modified:
    - .gitignore

key-decisions:
  - "The OS-entropy guard asserts over `cargo tree --edges normal`, not the full tree: getrandom is reachable only through the proptest dev-dependency, which the simulation cannot reach. Asserting over the full tree would assert something false."
  - "`ConfigError` carries a third variant, `Utf8`, alongside `Io` and `Parse` — reading bytes (required for the hash) makes non-UTF-8 input a real, distinct failure mode that must not be collapsed into a parse error."
  - "`config_hash` builds hex byte-by-byte with `{b:02x}` rather than relying on a `LowerHex` impl on the digest type, so the idiom survives a future sha2 major bump (D-25's hazard, closed structurally rather than by the version pin alone)."
  - "`tests/toolchain.sh` calls grep directly over a bash array instead of through xargs, so grep's exit 1 (no match) stays distinguishable from exit 2 (real error) — xargs reports 123 for both, which would let an unreadable file default to a passing comparison."
  - "The `target-cpu` search is scoped to build-configuration files (Cargo.toml, rust-toolchain.toml, .cargo/*, .github/workflows/*) rather than the whole repository, because only those can actually set the flag and the planning prose legitimately names it."
  - "`src/money.rs` names no floating-point type at all, including in its doc comments — the grep-able form of the float-never-touches-money rule."

patterns-established:
  - "Tracer slice: the end-to-end path is built and proved first; sibling plans widen layers without changing the shape."
  - "Adjacent-case testing: every negative assertion ships with its one-step-below positive, so 'the check works' cannot be confused with 'everything fails'."
  - "Load-bearing settings are proved load-bearing: the release overflow check was verified by removing it and observing the suite go red."
  - "Library reachability is asserted from tests/: the integration test recomputes the binary's output through `use sim::…`, which is what proves main.rs holds no logic."

requirements-completed: [CORE-02, CORE-08, CORE-09]

coverage:
  - id: D1
    description: "The seeded-run spine runs end to end: config bytes → SHA-256 → typed Params → effective seed → RNG sub-stream draw → Money, printed as one deterministic line."
    requirement: CORE-08
    verification:
      - kind: e2e
        ref: "tests/tracer_end_to_end.rs#runs_end_to_end"
        status: pass
      - kind: e2e
        ref: "cargo run -- --config config/baseline.toml --seed 7 --out $(mktemp -d)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A run is reproducible from its seed across the process boundary, and a different seed changes the draw (the counter-check a constant RNG cannot pass)."
    requirement: CORE-08
    verification:
      - kind: e2e
        ref: "tests/tracer_end_to_end.rs#same_seed_is_reproducible"
        status: pass
      - kind: e2e
        ref: "tests/tracer_end_to_end.rs#different_seed_changes_the_draw"
        status: pass
    human_judgment: false
  - id: D3
    description: "An integration test reaches every public item it needs through `use sim::…`; no simulation logic lives only in src/main.rs."
    requirement: CORE-08
    verification:
      - kind: integration
        ref: "tests/tracer_end_to_end.rs#runs_end_to_end (recomputes the binary's draw, hash and money via use sim::…)"
        status: pass
      - kind: other
        ref: "grep -c anyhow src/lib.rs src/money.rs src/config.rs src/rng.rs => 0 for every file"
        status: pass
    human_judgment: false
  - id: D4
    description: "The release profile cannot silently wrap: raw i64 overflow panics under --release, and the adjacent non-overflowing case returns normally."
    requirement: CORE-02
    verification:
      - kind: unit
        ref: "tests/tracer_end_to_end.rs#raw_i64_overflow_panics_in_release"
        status: pass
      - kind: unit
        ref: "tests/tracer_end_to_end.rs#raw_i64_at_the_maximum_does_not_panic"
        status: pass
      - kind: other
        ref: "negative test: overflow-checks removed => cargo test --release exits 101 'test did not panic as expected'"
        status: pass
    human_judgment: false
  - id: D5
    description: "The reproducibility contract is a checkable fact: lockfile and toolchain pin tracked, no data-parallelism dependency, no .cargo codegen override, no OS-entropy crate on the behaviour path, channel 1.94.1 pinned."
    requirement: CORE-09
    verification:
      - kind: other
        ref: "bash tests/toolchain.sh (exits 0, final line begins 'OK: ')"
        status: pass
      - kind: other
        ref: "negative tests: appended rayon dep / target-cpu line / .cargo/config.toml each make the script exit 1"
        status: pass
    human_judgment: false

# Metrics
duration: 11 min
completed: 2026-08-30
status: complete
---

# Phase 1 Plan 01: The Determinism Spine Summary

**A seeded `sim` binary that reads a TOML config, SHA-256s its raw bytes, resolves the effective seed, opens one bit-packed ChaCha8 sub-stream and constructs one `Money` value — printing a single line that is byte-identical across processes at the same seed and different at another.**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-08-30T22:57:00Z (approx.)
- **Completed:** 2026-08-30T23:08:00Z
- **Tasks:** 3 of 3
- **Files modified:** 12 (11 created, 1 modified), 1322 insertions

## Accomplishments

- **The whole Phase 1 spine composes.** The architecture CLAUDE.md and `01-RESEARCH.md` specify was proved by execution, not by review, before six sibling plans build on it: config → hash → effective seed → sub-stream → draw → money, one call site, no module off that path.
- **Reproducibility is demonstrated at the artefact level.** Two invocations of the built binary at seed 7 produce byte-identical stdout; seed 8 produces a different `draw`. The counter-check is what makes a constant RNG unable to pass.
- **CORE-02 is observable rather than declared.** Removing `overflow-checks = true` from `[profile.release]` was verified to turn the suite red (`test did not panic as expected`) and restoring it green — the setting is proved load-bearing, not merely present.
- **CORE-09 is a script, not a convention.** `tests/toolchain.sh` asserts all five facts and was verified to fail on each of three deliberate breakages (a rayon dependency, a `target-cpu` line, a `.cargo/config.toml`).
- **CORE-08 is proved from the test side.** `runs_end_to_end` recomputes the binary's draw, config hash and money through `use sim::…` and asserts they match — the strongest available statement that `src/main.rs` holds no simulation logic.

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end seeded run (tracer)** — `03b9e5c` (feat)
2. **Task 2: Commit the reproducibility contract and guard it** — `555d20d` (chore)
3. **Task 3: Prove the release profile cannot silently wrap** — `7626f95` (test)

## Files Created/Modified

- `Cargo.toml` — edition 2024, the eight pinned crates, proptest dev-dep, `[profile.release] overflow-checks`, `[lints.clippy]` deny levels
- `rust-toolchain.toml` — channel 1.94.1 + clippy/rustfmt, minimal profile
- `Cargo.lock` — committed; a minor bump of the RNG crate can legally change a sampling algorithm
- `src/lib.rs` — library root, `#![forbid(unsafe_code)]`, `pub mod config/money/rng`
- `src/money.rs` — `Money(i64)` with a private field; `ZERO`, `from_cents`, `cents`, panicking `Add`
- `src/config.rs` — `Params`/`Sim`/`MoneySection` with `deny_unknown_fields` on each; `load`, `config_hash`, `ConfigError`
- `src/rng.rs` — `Rngs`, `Stream`, `Purpose`; the 24/24/16 sub-stream key; `below` and `draws`
- `src/main.rs` — clap `Cli` with exactly `--config`/`--seed`/`--out`; anyhow confined here
- `config/baseline.toml` — `[sim]` (ticks 3650, seed 42, households 200, firms 20) and `[money]`
- `tests/tracer_end_to_end.rs` — five cases over the built binary and the raw-`i64` overflow pair
- `tests/toolchain.sh` — the CORE-09 guard, executable, `OK:` on success
- `.gitignore` — added `/target`; `Cargo.lock` deliberately not ignored

## Decisions Made

- **The OS-entropy assertion runs over the normal dependency graph.** `getrandom` is reachable only through `proptest` (which seeds its own generator, and whose `tempfile` names its own directories) — a dev edge the simulation cannot reach. `cargo tree --edges normal` reports zero. See deviation 1.
- **`ConfigError` gained a `Utf8` variant.** `load` must read bytes (the hash is over bytes), which makes non-UTF-8 input a distinct, real failure. Folding it into `Parse` would have reported a lie.
- **Hex is assembled byte-by-byte.** D-25 pins `sha2` to 0.10.x because 0.11's digest type has no `LowerHex`. Building hex with `{b:02x}` closes the hazard structurally, so a future bump is a version decision rather than a broken idiom.
- **The `target-cpu` search is scoped to build-configuration files.** Only `Cargo.toml`, `rust-toolchain.toml`, `.cargo/*` and CI workflows can set the flag; the planning prose legitimately names it, and a repo-wide grep would be a permanently red check. The globs already cover the CI file a later plan adds.
- **`src/money.rs` names no floating-point type anywhere, doc comments included** — the grep-able form of the rule that a float must never reach money.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The OS-entropy guard could not pass as literally specified**

- **Found during:** Task 2 (the CORE-09 guard script)
- **Issue:** The plan's acceptance criterion is `cargo tree 2>/dev/null | grep -c '^.*getrandom'` reports 0. It reports **2**. `01-RESEARCH.md` verified `getrandom`'s absence under the chosen *runtime* feature set, but the same plan's Task 1 adds `proptest 1.11.0` as a dev-dependency (deliberately, so `Cargo.toml` stays single-owner for plan 01-03). `proptest` pulls `rand 0.9.5` → `rand_core 0.9.5` → `getrandom 0.3.4`, and `tempfile 3.27.0` → `getrandom 0.4.3`. The criterion and the plan's own dependency choice are in direct conflict; satisfying it literally would mean dropping `proptest`, which plan 01-03 requires.
- **Fix:** The guard asserts over `cargo tree --edges normal` — the library-and-binary graph, which is exactly what ships and what the simulation runs on. That reports **0**. This is a sharpening, not a weakening: it states precisely the claim CORE-03/Pitfall 1 actually makes ("no OS entropy path exists" on the behaviour path), whereas the full-tree form asserts something false about a test-harness edge the simulation cannot reach.
- **Files modified:** `tests/toolchain.sh`
- **Verification:** `cargo tree -i getrandom@0.3.4` and `-i getrandom@0.4.3` both trace to `proptest [dev-dependencies]` and nowhere else; `cargo tree --edges normal | grep -c getrandom` = 0; `bash tests/toolchain.sh` exits 0.
- **Committed in:** `555d20d` (Task 2 commit)

**2. [Rule 1 - Bug] `src/money.rs` doc comment tripped its own acceptance criterion**

- **Found during:** Task 1 (verification gate)
- **Issue:** The criterion is that `src/money.rs` "does not contain `f64`". The module doc listed the deliberately-absent impls verbatim as `From<f64>`, `Into<f64>`, `Mul<f64>` — so the file documenting the float ban was the only file violating its grep.
- **Fix:** Reworded to name the prohibition without naming the type, and stated explicitly that the module names no floating-point type at all, which is the grep-able form of the rule.
- **Files modified:** `src/money.rs`
- **Verification:** `grep -c f64 src/money.rs` = 0; `grep -n 'pub const fn from_cents(' src/money.rs` matches; tests still pass.
- **Committed in:** `03b9e5c` (Task 1 commit)

**3. [Rule 2 - Missing Critical] `xargs` erased grep's error/no-match distinction**

- **Found during:** Task 2 (the guard script)
- **Issue:** The plan requires the script have "no error-suppressing fallbacks so a missing input surfaces rather than defaults to a passing comparison". The first draft piped a file list through `xargs grep -l`. `xargs` returns **123** for any non-zero child status, so grep's exit 1 (no match — a pass) and exit 2 (unreadable file — a real failure) became indistinguishable. An unreadable build-config file would have silently defaulted to a pass.
- **Fix:** Replaced `xargs` with `mapfile` into a bash array and one direct `grep -l -- "${FILES[@]}"`, then branched on the true status: `>1` fails loudly, `1` passes, `0` reports the offending files by name. Added an explicit failure when the file list is empty, so a broken `git ls-files` cannot look like a clean repo either.
- **Files modified:** `tests/toolchain.sh`
- **Verification:** clean tree exits 0 with `OK:`; a `target-cpu` line in `Cargo.toml` exits 1 naming the file; both re-checked after restore.
- **Committed in:** `555d20d` (Task 2 commit)

**4. [Rule 2 - Missing Critical] `ConfigError::Utf8` added**

- **Found during:** Task 1 (`src/config.rs`)
- **Issue:** The plan specifies `load` reads raw **bytes** (needed for the hash) and parses "the same bytes", and specifies `ConfigError` carry "at least" `Io` and `Parse`. Bytes → `toml::from_str` requires a UTF-8 conversion that can fail, and that failure is neither an I/O error nor a TOML parse error. Unwrapping would panic; folding it into `Parse` would misreport the cause.
- **Fix:** Added a third variant `Utf8 { path, source }`, keeping every failure mode named and typed.
- **Files modified:** `src/config.rs`
- **Verification:** `cargo build` and `cargo clippy --all-targets` clean; `load` returns `(Params, String)` as specified and the binary's hash matches the library's.
- **Committed in:** `03b9e5c` (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (2 bugs, 2 missing-critical)
**Impact on plan:** No scope creep and no shape change. Deviation 1 is the only one that alters a stated acceptance criterion; it is recorded here explicitly because the substituted check is *narrower and true* where the original was broad and false. Deviations 2–4 are corrections that make the plan's own criteria satisfiable and its own "no error-suppressing fallbacks" instruction actually hold.

## Issues Encountered

- **`cargo test --release` needed proof, not assumption.** Whether `[profile.release] overflow-checks` reaches the test profile under `--release` is an inheritance question, not something to take on faith. Resolved by removing the setting and observing `raw_i64_overflow_panics_in_release` fail with `test did not panic as expected` (exit 101), then restoring it and observing 5/5 green. The setting is confirmed load-bearing on the exact command the phase will run in CI.
- **The lockfile survived the negative tests intact.** The rayon negative test appends a dependency before running the guard, which resolves the graph. Verified `Cargo.lock` contains no `rayon` and `git diff --stat Cargo.lock` is empty — the guard's rayon check fires before its `cargo tree` call, so resolution never ran against the polluted manifest.

## Deliberately Narrow Surface (not stubs)

Each module ships exactly what the tracer path needs, per the plan's explicit instruction. These are complete, functioning implementations of a thin slice — not placeholders, and nothing returns fabricated or empty data. Named successors:

| Item | Current surface | Widened by |
|---|---|---|
| `Money` | `ZERO`, `from_cents`, `cents`, `Add` | 01-03 adds `MoneyOverflow`, the named `Result` API, `split`, `Sum`, remaining operators |
| `Purpose` | `TracerProbe = 1` | 01-04 appends the real draw sites (append-only, never renumbered) |
| `Stream` | `below`, `draws` | 01-04 adds `coin_ppm`, `sample_k`, `shuffle_in_place`, the issued-key guard |
| `Params` | `[sim]`, `[money]` | 01-06 widens to the full parameter set; 01-08 adds `# GRADE:` annotations |
| `src/lib.rs` | `config`, `money`, `rng` | 01-05 adds `ids`, `numeric` with their files |
| `clippy.toml` | absent (an empty list, not an error) | 01-07 supplies the disallowed-types/methods lists |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready.** The spine is proved and every sibling plan in this phase widens one layer of it without changing its shape:

- **01-02 … 01-08** all build on files this plan created. `Cargo.toml` is single-owner and already declares `proptest` for 01-03. `src/lib.rs` is owned by 01-05 next. `src/rng.rs` is owned by 01-04 next, and the key layout it must preserve is the layout written here.
- **Phase 2 (ledger)** consumes `Money` and its checked API; the operator half (panic in every profile) is in place, and `panic = "unwind"` was deliberately left at its default so `#[should_panic]` and `catch_unwind` machinery keeps working for Phase 2's negative invariant tests.
- **Phase 3 (tick pipeline / log seam)** consumes the `Rngs` facade, the `Purpose` enum, the config hash and the effective-seed value — all present and exercised.

**Concerns:** none blocking. One note for the phase verifier: the CORE-09 acceptance criterion as written in `01-01-PLAN.md` (`cargo tree | grep -c getrandom` = 0) will not pass on the full tree and should be read as the `--edges normal` form (deviation 1). The plan's own "Flagged assumptions" table already anticipates that CORE-09 is a set of repository facts rather than a data-shape behaviour.

---
*Phase: 01-primitives-and-the-determinism-spine*
*Completed: 2026-08-30*

## Self-Check: PASSED

All 11 created files verified present on disk; all 3 task commits verified in `git log`.
