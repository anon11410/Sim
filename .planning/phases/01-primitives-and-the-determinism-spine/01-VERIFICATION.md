---
phase: 01-primitives-and-the-determinism-spine
verified: 2026-08-31T01:12:23Z
status: human_needed
score: 18/20 must-haves verified
behavior_unverified: 1
overrides_applied: 0
deferred:
  - truth: "CORE-11 clause (b) — the Lengnick Table 1 values are checked against the published paper by a person with journal access"
    addressed_in: "Phase 6"
    evidence: "ROADMAP.md Phase 6 Success Criterion 6: 'CORE-11 clause (b), gated here per D-19: before these values are consumed, the Lengnick Table 1 rows marked UNVERIFIED in config/PROVENANCE.md have been checked against the published paper by a person with journal access, following the procedure shipped in Phase 1'. Matching deferral recorded in REQUIREMENTS.md CORE-11 rationale and ROADMAP.md Phase 1 Success Criterion 5."
behavior_unverified_items:
  - truth: "Continuous integration runs the lint gate with the flags that actually lint every target, plus the test suite in both build profiles, so the determinism bans fail the build rather than documenting a preference."
    test: "Push the phase branch and open the Actions run for .github/workflows/ci.yml. Confirm the job reaches every one of the seven steps and exits green, paying attention to step 1 (`rustup show active-toolchain`) on a fresh runner image."
    expected: "All seven steps pass. Specifically, `rustup show active-toolchain` must not fail on a runner whose rustup has not yet installed 1.94.1 — rustup 1.28+ no longer auto-installs on `rustup show`, and the following `rustup component add clippy rustfmt` would then error on an uninstalled toolchain, failing the job before any real check runs."
    why_human: "The workflow has never executed on GitHub Actions. All seven steps were re-run locally by the verifier and every one passes, but runner-image rustup behaviour and whether Actions is enabled on the repository cannot be observed from this session — the GitHub API returned 'GitHub access is not enabled for this session'."
  - truth: "`overflow-checks = true` in `[profile.release]` applies uniformly to every integer arithmetic site compiled under that profile, so no ordering-dependent or inlined site escapes the check. (declared `verification: backstop` in 01-01-PLAN.md)"
    test: "Decide whether a held-out check is wanted — e.g. a release-profile test that overflows a raw i64 across an inlined call boundary and inside a generic, not only at a single black_boxed site."
    expected: "A universal claim about the profile flag either gains a held-out test that could falsify it, or is downgraded in the plan to the single-site claim the existing evidence supports."
    why_human: "Non-inferable (`verification: backstop`) — abstained for insufficient_spec. The manifest fact IS asserted (tests/toolchain.sh check 4b, stateful awk scan of the [profile.release] section), and the verifier observed a release-profile raw-i64 overflow panic directly. Neither is evidence for the universal quantifier 'every arithmetic site'. Presence + wiring never qualifies for a backstop truth."
coincidental_reliance_items: []
human_verification:
  - test: "Adjudicate the CORE-06 spelling divergence: the generational field is `generation`, not `gen`. Confirmed by the verifier that `gen` is a reserved keyword in Rust edition 2024. Decide: amend the CORE-06 requirement text (and 01-RESEARCH Pattern 5 / D-03) to say `generation`, or accept the divergence with a standing note."
    expected: "A recorded decision. The spelling propagates into the Phase 3 log schema as the identity pair `(slot, generation)`, so the divergence should be closed before Phase 3 writes the schema rather than after."
    why_human: "STATE.md line 94 explicitly defers this to verify-phase ('flagged for human confirmation at verify-phase'). Forced by the language, not by choice, and the type shape and derived total order are unchanged — but it is a requirement-text change, which only a human should authorise."
  - test: "Adjudicate PROVENANCE.md open item V-3a: `bankruptcy.entrant_size_ratio_ppm = 800000` (0.8) while its SOURCE field cites `BAM size-replacing-firms = 0.2`. Decide the tracking route so it cannot reach Phase 10 unsettled."
    expected: "V-3a escalated out of config/PROVENANCE.md into a place that gates consumption — STATE.md's open-items list (where V-4 already sits) and/or a ROADMAP Phase 10 criterion. Phase 6 criterion 6 covers only 'Lengnick Table 1 rows', and a BAM row is not in Table 1, so nothing currently forces the check before BANK-04 consumes the value."
    why_human: "Correctly NOT resolved by the agent: D-20 forbids settling a parameter's meaning from model memory, and the honest recording is what CORE-11(a) requires. What needs a human is the routing decision, not the value."
  - test: "Confirm the CI workflow executes green on GitHub Actions (see behavior_unverified_items entry 1)."
    expected: "A green run of .github/workflows/ci.yml on the phase branch."
    why_human: "Cannot be observed from this session; GitHub API access is disabled."
  - test: "Decide whether the `overflow-checks` universality backstop needs a held-out test (see behavior_unverified_items entry 2)."
    expected: "Either a held-out test or a downgraded claim."
    why_human: "Backstop abstention — insufficient_spec."
---

# Phase 1: Primitives and the Determinism Spine — Verification Report

**Phase Goal:** The project's vocabulary — money, identity, configuration and randomness — exists with the correctness properties that every later phase depends on and that no later phase can add cheaply.
**Verified:** 2026-08-31T01:12:23Z
**Status:** human_needed
**Re-verification:** No — initial verification.

**Verification stance.** Every gate below was re-run by the verifier in its own process. Where a
guard's whole value is that it *fires*, the verifier corrupted the tree itself and observed the
failure, rather than accepting the SUMMARY's or `tests/lints.sh`'s word for it. Where a claim was
about provenance (the generated ban list), the verifier independently re-derived the artefact from
its stated source and diffed it. Every corruption was reverted; the working tree is clean.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `Money` arithmetic panics on overflow in **both** debug and release profiles | ✓ VERIFIED | Operators route through `i64::checked_*` + `.expect` in `src/money.rs`, so the panic is in the code and not in the profile. `cargo test` 139 passed / `cargo test --release` 137 passed, both re-run by the verifier. Release-only pair `raw_i64_overflow_panics_when_overflow_checks_are_on` + `raw_i64_at_the_maximum_does_not_panic` confirmed present in the `--release` pass. |
| 2 | `Money::split(n)` sums exactly to the original, property-tested over amounts that do **not** divide evenly | ✓ VERIFIED | `tests/money_props.rs` `split_parts_sum_to_the_whole_when_not_evenly_divisible` uses `prop_assume!(amount % n != 0)`; 512 cases; strategy weights `i64::MAX`, `i64::MIN`, `0` and `n == 1` explicitly. `.proptest-regressions/money_props.txt` carries a committed real counterexample (`amount = 9223372036854775807, n = 1` — the CR-01 defect). This is the strongest single piece of evidence in the phase: the property is proven to have caught a real bug and is replayed forever. |
| 3 | `Money` implements no float conversion and no decimal `Display`, so no call site can cross the float boundary through the money type | ✓ VERIFIED | `src/money.rs` names no floating-point type at all (confirmed by `tests/numeric_det.rs::confinement_of_the_float_domain`, which reads every file under `src/` and asserts both the type name and the untyped-literal form). No `From<f64>`, `Into<f64>`, `Mul<f64>` or `Display` impl exists. |
| 4 | Same master seed ⇒ identical `u64` streams; different master seed ⇒ different stream; verified in-process **and** cross-process | ✓ VERIFIED | In-process: `tests/determinism_rng.rs::same_master_seed_identical_streams` (64 sub-streams) and `different_master_seed_differs`. Cross-process: `tests/tracer_end_to_end.rs::same_seed_is_reproducible` / `different_seed_changes_the_draw` spawn the built binary via `CARGO_BIN_EXE_sim`. All executed and green. |
| 5 | Sub-streams keyed on different `(tick, agent, purpose)` are isolated — an added draw in one market provably cannot perturb another | ✓ VERIFIED (behavioral) | This is an ordering/state invariant, so presence was not accepted. `extra_draws_in_one_purpose_cannot_perturb_another` takes 7 extra labour draws and asserts the goods sub-stream is bit-identical. Mechanism confirmed in `src/rng.rs`: each `stream()` re-seeds `ChaCha8Rng::from_seed(master)` then `set_stream(key)`, which resets `word_pos`. The second arm (`…_when_a_pool_is_involved`) covers the *other* coupling channel — `sample_k` permutes the caller's pool in place — and asserts both halves: fresh pools stay isolated, and a **shared** pool is asserted to *differ*, so the documented hazard is observed rather than promised. |
| 6 | Every sampler is fixed-draw with an exact stated count; no rejection sampling, no unbounded loop on the behaviour path | ✓ VERIFIED | `below` = 1 draw (multiply-high, no rejection loop); `coin_ppm` = 1; `sample_k` = exactly `k` (partial Fisher-Yates); `shuffle_in_place` = exactly `len-1`. Each asserted by a named test against `Stream::draws()`. `rand`'s own range/uniform/index samplers are not called and their identifiers are deliberately kept out of the file so a grep cannot false-positive. |
| 7 | CORE-03 (a): the standard-RNG and system-entropy generators are absent — referencing either does not compile | ✓ VERIFIED (verifier-executed) | The verifier wrote a throwaway integration test referencing each. `rand::rngs::StdRng` → `error[E0433]: failed to resolve: could not find StdRng in rngs`. `rand::rngs::SysRng` → `error[E0425]: cannot find value SysRng in module rand::rngs`. Probe removed; tree clean. |
| 8 | CORE-03 (b): `SmallRng` and the Xoshiro generators are never *used* — banned by lint plus source assertion | ✓ VERIFIED (verifier-executed) | The verifier confirmed `SmallRng` **does** compile under this feature set (so the 01-02 amendment rests on a true fact, and the CLAUDE.md correction is accurate), and that `cargo clippy --all-targets -- -D warnings` then emits `error: use of a disallowed type rand::rngs::SmallRng`. `tests/lints.sh` check 4d additionally greps all three names out of `src/`. |
| 9 | Generational `FirmId`: an identity held across a respawn is a typed miss; respawn is in place; identity carries a total order | ✓ VERIFIED (behavioral) | A state transition, so a passing test was required, not presence. `tests/ids_generational.rs::stale_identity_after_respawn_is_a_typed_miss` asserts `get`/`get_mut` both return `None` post-respawn and `Some` for the fresh id. `respawn_does_not_disturb_neighbouring_slots` pins in-place semantics; the arena exposes no removal operation and `swap_remove` appears nowhere. `FirmArena::with_occupants` now bounds length at `u16::MAX` with a real `assert!` (CR-03 fix) and `live_ids` uses `u16::try_from`, not `as`. |
| 10 | The float domain is one module wide, `pow_frac` is bit-identical across invocations, and `demand_to_units` is the single crossing | ✓ VERIFIED | `confinement_of_the_float_domain` walks every `.rs` under `src/`, matching on the path relative to `src/` (not the basename), and rejects both a float **type name** and an untyped float **literal** outside `numeric.rs`; `config.rs` is narrowed line-by-line to lines containing `expected_demand`. `pow_frac_is_bit_identical_across_many_invocations` compares `to_bits()` over 100 000 calls. `pow_frac(x,0.5) == x.sqrt()` and `pow_frac(x,0.25) == x.sqrt().sqrt()` exactly. `bit_count_is_load_bearing` proves 20 vs 40 bits actually differ, so the constant is not decorative. Domain guards (`pow_frac` base/exponent, `demand_to_units` finiteness) are unconditional `assert!`, not `debug_assert!` (WR-05/WR-06 fixes). |
| 11 | Config strictness: unknown key, missing key and removed value each fail with a **named** error; no serde defaults; no optional fields | ✓ VERIFIED | `every_key_is_required` is exhaustive, not a spot check: it enumerates every leaf of the shipped config (41 keys), deletes each in turn, and asserts the error names that leaf. `unknown_key_inside_a_table_is_rejected` deliberately targets a nested table so root-level `deny_unknown_fields` is not what catches it. `no_serde_defaults_anywhere_in_src` is line-based and attribute-order agnostic (WR-02 fix — the old whole-file `contains("serde(default")` was defeated by `#[serde(rename="x", default)]`). `no_optional_fields_in_the_config_schema` closes the un-greppable form. `the_schema_and_the_shipped_config_name_the_same_leaves` closes it from the other direction: a `Params` field with no config key fails. |
| 12 | The config hash is over raw file bytes, stable on repeat, and changes when comments change | ✓ VERIFIED | `config_hash` reads the file once and hashes those same bytes; `load` returns `(Params, hash)` from one read, so they cannot describe different byte sequences. `key_order_does_not_change_params_but_does_change_the_hash` asserts both halves. `hash_is_stable_across_repeated_computation` asserts a 64-char lowercase hex digest. |
| 13 | CORE-07: `cargo clippy --all-targets --all-features -- -D warnings` **fails the build** when a hashed collection or a banned float method is introduced | ✓ VERIFIED (verifier-executed injection) | Not accepted on `tests/lints.sh`'s word. The verifier appended a function using `std::collections::HashMap` and `f64::powf` to **`src/ids.rs`** (lints.sh only injects into `tests/`) and ran the gate: `error: use of a disallowed type std::collections::HashMap`, `error: use of a disallowed method f64::powf`, exit **101**. File restored from backup; `git status` clean; gate re-run green. `tests/lints.sh` separately reports all 60 resolvable method bans firing, one per marked probe call site. |
| 14 | CORE-09: `Cargo.lock` and `rust-toolchain.toml` committed; no data-parallelism crate; no `-C target-cpu=native`; no OS-entropy crate on the behaviour path | ✓ VERIFIED | `git ls-files` confirms both committed. `.cargo/` does not exist; no `target-cpu` string anywhere outside `target/`. `tests/toolchain.sh` (exit 0, re-run) searches the dependency **graph** via `cargo tree --edges normal`, not the manifest (WR-08 fix). Verifier independently confirmed `cargo tree -e normal` has no `getrandom` and no `rayon`, and that both `getrandom` instances reach the graph **only** through `proptest`, a dev-dependency (`cargo tree -i getrandom@0.3.4` → `rand_core 0.9.5 → rand 0.9.5 → proptest [dev-dependencies]`). |
| 15 | CORE-08: `lib.rs` plus a thin `main.rs`; exactly one lib and one bin target; integration tests reach the whole surface via `use sim::…` | ✓ VERIFIED | `cargo metadata --no-deps` shows two `sim` targets (lib + bin) and seven integration-test targets; no `[lib]`/`[[bin]]` tables, so both are implicit. `main.rs` is 61 lines and holds no simulation logic. The verifier wrote and ran a throwaway test whose only body is `use sim::{money::Money, rng::Rngs, config::Params};` — it compiled and passed; probe removed. `tracer_end_to_end.rs` recomputes every field the binary printed through the library surface and compares, which is the *behavioural* form of "no logic lives only in main.rs". |
| 16 | CORE-10 carve-out: `POW_FRAC_BITS`, `PPM_SCALE`, `MILLI_SCALE` are `const` in `src/numeric.rs` with a `GRADE: PROJECT` entry in `config/PROVENANCE.md` | ✓ VERIFIED | `config/PROVENANCE.md` §4 ("The project-grade code constants") is that record, with one table row per constant, each graded `GRADE: PROJECT`, each stating why it is code and not configuration. The section opens by quoting the amended CORE-10 clause it answers. It also carries the honest caveat that `POW_FRAC_BITS` nonetheless changes every trajectory. See W3 — the section is documented but not test-guarded. |
| 17 | CORE-11 (a): every config value carries a source-grade annotation, asserted by a test that names any unannotated key | ✓ VERIFIED | 41 leaf keys, 41 `# GRADE:` annotation blocks — 1:1, counted by the verifier. Six tests in `tests/provenance.rs` all green: `every_key_has_a_source_grade`, `every_grade_letter_is_in_the_vocabulary`, `every_annotation_has_a_source_and_a_cadence`, `no_annotation_is_orphaned`, `every_config_key_has_a_provenance_row`, `attributed_rows_are_still_marked_unverified`. The last is the anti-circularity guard: it *fails* the moment a row is upgraded, forcing the upgrade and the evidence into one commit. |
| 18 | CORE-11 (b) is **deferred, not dropped** — recorded in the requirement AND in the Phase 6 criteria | ✓ VERIFIED | Four independent recordings, all read by the verifier: (i) `REQUIREMENTS.md` CORE-11 clause (b) marked "**a blocking gate on Phase 6**" with an inline rationale citing D-19/D-20; (ii) `ROADMAP.md` Phase 1 Success Criterion 5 carries the same split and names D-19; (iii) `ROADMAP.md` **Phase 6 Success Criterion 6** is the gate itself, spelled out with the per-row outcome vocabulary (agrees / differs with the paper's value / not in Table 1); (iv) `STATE.md` line 83. Commit `c196193` changed REQUIREMENTS.md and ROADMAP.md in **one diff**, which is what the 01-02 prohibition demanded. |
| 19 | Continuous integration runs the lint gate with the flags that lint every target, plus both test profiles, so the bans fail the build | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | `.github/workflows/ci.yml` is committed, triggers on `push` and `pull_request`, and contains all seven steps with the correct flags (`cargo clippy --all-targets --all-features -- -D warnings`, `--locked` on build and both test runs, `cargo fmt --check`, `bash tests/toolchain.sh`, `bash tests/lints.sh`). The verifier ran **all seven steps locally, in order** — every one passes. What is unverified is the *runner*: the workflow has never executed on GitHub Actions, and `rustup show active-toolchain` on a fresh image is the specific unproven step (rustup 1.28+ no longer auto-installs on `rustup show`, which would then make `rustup component add` error on an uninstalled toolchain). GitHub API access is disabled in this session, so no run history could be read. Routed to human verification. |
| 20 | (backstop) `overflow-checks = true` applies uniformly to every integer arithmetic site compiled under the release profile | ⚠️ insufficient_spec (abstained) | The *manifest fact* is asserted where it belongs — `tests/toolchain.sh` check 4b uses a stateful `awk` scan over the `[profile.release]` section so it cannot be satisfied by an `overflow-checks` line under another profile (WR-12 fix), and `tracer_end_to_end.rs` names in its own comment what the test does and does not prove. The verifier directly observed a release-profile raw-`i64` overflow panic. None of that is evidence for the universal quantifier "**every** arithmetic site, including inlined and ordering-dependent ones". Per the backstop protocol, presence + wiring never qualifies; abstained rather than marked VERIFIED. |

**Score:** 18/20 truths verified (1 present, behavior-unverified; 1 abstained, insufficient_spec)

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | CORE-11 clause (b) — Lengnick Table 1 values checked against the published paper by a person with journal access | Phase 6 | ROADMAP.md Phase 6 Success Criterion 6, quoted in full in the frontmatter. Deferral authority D-19; standing prohibition D-20. Not a gap. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | edition 2024, pinned deps, `[profile.release] overflow-checks`, `[lints.clippy]` levels | ✓ VERIFIED | All present. `rand` is `default-features = false, features = ["std","chacha"]` — the feature set that makes `StdRng`/`SysRng` not resolve. |
| `rust-toolchain.toml` | pinned 1.94.1 + clippy + rustfmt | ✓ VERIFIED | Committed, `channel = "1.94.1"`. Note: `rust-src` is **not** a listed component (see W2). |
| `Cargo.lock` | committed | ✓ VERIFIED | Git-tracked; CI uses `--locked` so a drifted lockfile is a failure, not a silent regeneration. |
| `src/lib.rs` | module root, `forbid(unsafe_code)` | ✓ VERIFIED | 13 lines, five modules, `#![forbid(unsafe_code)]`. |
| `src/main.rs` | clap CLI, three flags, no simulation logic | ✓ VERIFIED | 61 lines. Effective seed = `cli.seed.unwrap_or(params.sim.seed)` and that is what is printed. |
| `src/money.rs` | Money newtype, panicking operators, Result API, exact split | ✓ VERIFIED | 480 lines, private `i64` field, split API, `MoneyOverflow` carrying operands and operator. |
| `src/rng.rs` | Rngs facade, bit-packed key, Purpose, fixed-draw samplers, re-entry guard | ✓ VERIFIED | 553 lines. `ALL_PURPOSES` completeness is now a **compile error** via an exhaustive `const fn` match plus a `const _` assertion loop (WR-03 fix) — not a comment. |
| `src/ids.rs` | generational FirmId, arena with Option accessors, in-place respawn | ✓ VERIFIED | 345 lines. Field spelled `generation` (see human item 1). |
| `src/numeric.rs` | confined float domain, `pow_frac_det`, the three constants, `demand_to_units` | ✓ VERIFIED | 295 lines, the only module permitted a float type. |
| `src/config.rs` | full Params tree, `deny_unknown_fields` on **every** struct, `load`, `config_hash`, `Params::validate` | ✓ VERIFIED | 811 lines. Seven structs, seven `deny_unknown_fields`. `validate()` added by the CR-02 fix — every bound names the consumer that imposes it. |
| `config/baseline.toml` | every simulation and economic parameter, per-key GRADE annotation | ✓ VERIFIED | 41 keys, 41 annotations, six tables. |
| `config/PROVENANCE.md` | per-key rows, UNVERIFIED markings, verification procedure, GRADE PROJECT carve-out | ✓ VERIFIED | 44 table rows, 31 UNVERIFIED mentions, a four-step procedure a non-domain-expert can execute, §4 carve-out record, five open items (V-1…V-5) plus V-3a. |
| `clippy.toml` | generated disallowed-types and disallowed-methods lists | ✓ VERIFIED (independently re-derived) | 5 types + 68 methods (2 clock + 66 float). See Data-Flow Trace below — the verifier re-derived the 66 from std source and got an exact match. |
| `tests/lints.sh` | the negative test: clean passes, injected hazard fails, probe count matches | ✓ VERIFIED | Exit 0, re-run. `trap cleanup EXIT INT TERM` means the injected files cannot survive a crash — which moots review finding IN-04. |
| `tests/toolchain.sh` | CORE-09 guard | ✓ VERIFIED | Exit 0, re-run. |
| `.github/workflows/ci.yml` | seven-step determinism gate | ⚠️ PRESENT, RUNNER-UNVERIFIED | Content correct and locally reproduced end to end; never executed on Actions. |
| `tests/lint-probes/*.rs.txt` | non-compiled probe + hazard sources | ✓ VERIFIED | Both git-tracked. |
| `.proptest-regressions/money_props.txt` | committed counterexamples | ✓ VERIFIED | Contains the real CR-01 counterexample. |
| clippy.toml generator script | derivation is reproducible | ⚠️ NOT COMMITTED | See W2. The *result* was verified correct by independent re-derivation; the *process* is not committed. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/config.rs` | `sim::config::load` then hash over the same bytes | ✓ WIRED | One read, one hash, one parse — `load` returns both, so they cannot disagree. `tracer_end_to_end` asserts binary hash == library hash. |
| `src/main.rs` | `src/rng.rs` | `Rngs::new(effective_seed)`, one `Stream` | ✓ WIRED | Test recomputes the draw through the library and compares against the binary's printed field. |
| `tests/tracer_end_to_end.rs` | built binary | `env!("CARGO_BIN_EXE_sim")` | ✓ WIRED | Real artefact spawned, not an in-process call. |
| `src/config.rs` | `src/money.rs` | config money crosses via `checked_add`, never the panicking operator | ✓ WIRED | `load` does `stock.checked_add(stock)` and maps failure to `ConfigError::MoneyRange`. |
| `src/rng.rs` | `ChaCha8Rng` | `ChaCha8Rng::from_seed` — the only generator construction in the crate | ✓ WIRED | Sole construction site; every other module receives a `Stream`. Verified by grep and by the module's own structure. |
| `tests/lints.sh` | `clippy.toml` | probe diagnostic count vs probe call-site count | ✓ WIRED | Reports "all 60 resolvable method bans fired, one per marked call site" — this is what makes clippy's *silent* ignoring of an unresolvable path detectable. |
| `.github/workflows/ci.yml` | `tests/lints.sh` | CI invokes the negative test, not only the plain lint run | ✓ WIRED (content) | Step present with an in-file comment explaining why it is not redundant with the clippy step. Execution unverified (T19). |
| `config/baseline.toml` | `config/PROVENANCE.md` | every annotated key has a matching row keyed by table path | ✓ WIRED | `every_config_key_has_a_provenance_row` passes. |
| `config/PROVENANCE.md` | `.planning/research/SUMMARY.md` | grade vocabulary transcribed, not re-invented | ✓ WIRED | PROVENANCE quotes SUMMARY.md:169 **verbatim** in a blockquote; the verifier read SUMMARY.md:169 and confirmed the quote is exact. This closes the 01-08 backstop truth with explicit evidence. |
| `src/numeric.rs` | `clippy.toml` | the banned power function has a replacement, so the ban needs no exemption | ✓ WIRED | `f64::powf`/`powi` are banned on both widths; `pow_frac` is the replacement; `sqrt` is deliberately not banned, which is what makes the ban survivable. `tests/lints.sh` check 4b confirms **no file** carries `allow`/`expect` for `disallowed_types`, `disallowed_methods`, `clippy::all` or `warnings` (WR-04 fix). |

### Data-Flow Trace (Level 4) — provenance of the generated ban list

The one claim in this phase whose value is entirely in its *provenance* is "the banned-float-method
list is generated from the pinned toolchain's own standard-library source rather than typed by
hand". The generator script is not committed, so the verifier re-derived it independently.

| Artefact | Value traced | Source | Produces real data | Status |
|----------|-------------|--------|--------------------|--------|
| `clippy.toml` `disallowed-methods` (float entries) | the set of `f32`/`f64` methods carrying std's unspecified-precision marker | `$(rustc --print sysroot)/lib/rustlib/src/rust/library/{std,core}/src/num/{f64,f32}.rs` on toolchain **1.94.1** | ✓ YES | ✓ FLOWING — **exact match** |

Method: scanned all four std/core source files for the marker sentence *"The precision of this
function is non-deterministic."* and the `pub fn` it precedes; produced **66** distinct
`width::method` entries. Extracted the `f32`/`f64` entries from `clippy.toml`: **66**. `comm` in
both directions returned **empty** — no method carries the marker and escapes the ban, and no
banned path is invented. The header's stated counts (33 per width; 4 unstable per width documented
as clippy-unresolvable; 60 resolvable and fired) are consistent with this.

**Conclusion:** the list is correct and complete against the pinned toolchain *today*, whether it
was generated or typed. What is not guarded is drift — see W2.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full debug suite | `cargo test` | 87 lib + 14 + 14 + 4 + 4 + 5 + 6 + 5 = **139 passed, 0 failed** | ✓ PASS |
| Full release suite | `cargo test --release` | **137 passed, 0 failed** (2 fewer = the two `#[cfg(debug_assertions)]` re-entry-guard tests) | ✓ PASS |
| Lint gate, clean tree | `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 | ✓ PASS |
| Formatting | `cargo fmt --check` | exit 0 | ✓ PASS |
| **Lint gate blocks a hazard in `src/`** | verifier appended `HashMap` + `f64::powf` to `src/ids.rs`, ran clippy | 3 errors, **exit 101**; restored, tree clean, gate green again | ✓ PASS |
| `StdRng` absent | verifier compiled a probe referencing `rand::rngs::StdRng` | `error[E0433]: could not find StdRng in rngs` | ✓ PASS |
| `SysRng` absent | verifier compiled a probe referencing `rand::rngs::SysRng` | `error[E0425]: cannot find value SysRng` | ✓ PASS |
| `SmallRng` compiles but is lint-banned | verifier compiled + linted a `SmallRng` probe | test passed; clippy: `error: use of a disallowed type rand::rngs::SmallRng` | ✓ PASS |
| Library surface reachable from `tests/` | verifier ran a test whose body is only `use sim::{money::Money, rng::Rngs, config::Params};` | 1 passed | ✓ PASS |
| Release-profile raw-i64 overflow | `cargo test --release --test tracer_end_to_end` | `raw_i64_overflow_panics_when_overflow_checks_are_on - should panic ... ok` | ✓ PASS |
| Exactly one lib + one bin | `cargo metadata --no-deps` | two `sim` targets, no `[lib]`/`[[bin]]` tables | ✓ PASS |
| No OS-entropy on the behaviour path | `cargo tree -e normal`; `cargo tree -i getrandom@{0.3.4,0.4.3}` | both getrandom instances reach the graph only via `proptest` **[dev-dependencies]** | ✓ PASS |
| Ban list completeness | re-derived from 1.94.1 std/core source, diffed against `clippy.toml` | 66 vs 66, `comm` empty both ways | ✓ PASS |
| CI workflow on the runner | GitHub Actions API | `GitHub access is not enabled for this session` | ? SKIP → human |

### Probe Execution

| Probe | Command | Result | Status |
|-------|---------|--------|--------|
| `tests/toolchain.sh` | `bash tests/toolchain.sh` | exit 0 — "lockfile and toolchain tracked, no data-parallelism crate in the graph, no codegen override, no OS-entropy crate on the behaviour path, release profile checks overflow" | PASS |
| `tests/lints.sh` | `bash tests/lints.sh` | exit 0 — checks 1–4; "all 60 resolvable method bans (floats + the clock) fired, one per marked call site"; "no alias, exemption or non-portable generator escapes it" | PASS |

Both re-executed by the verifier in its own process. Neither result was taken from SUMMARY.md.

### Test Quality Audit

| Test file | Linked req | Active | Skipped | Circular | Assertion level | Verdict |
|-----------|-----------|--------|---------|----------|-----------------|---------|
| `src/money.rs` (unit + `split_tests`) | CORE-01 | 30 | 0 | no | Value + panic | ✓ Sufficient |
| `tests/money_props.rs` | CORE-01 | 4 props × 512 cases | 0 | no | Value (property) | ✓ Sufficient — with a committed real regression |
| `src/rng.rs` (unit) | CORE-03/04/05 | 17 | 0 | no | Value + panic | ✓ Sufficient |
| `tests/determinism_rng.rs` | CORE-03/04/05 | 14 | 0 | no | **Behavioral** (isolation across a code-change simulation) | ✓ Sufficient |
| `src/ids.rs` + `tests/ids_generational.rs` | CORE-06 | 13 | 0 | no | **Behavioral** (state transition across respawn) | ✓ Sufficient |
| `src/numeric.rs` + `tests/numeric_det.rs` | CORE-10 carve-out, D-11 | 20 | 0 | no | Value (bit-level) + source scan | ✓ Sufficient |
| `src/config.rs` + `tests/config_strict.rs` | CORE-10 | 25 | 0 | no | Value + named-error content | ✓ Sufficient — exhaustive, not spot-check |
| `tests/provenance.rs` | CORE-11 (a) | 6 | 0 | no | Value + coverage + no-silent-upgrade | ✓ Sufficient |
| `tests/tracer_end_to_end.rs` | CORE-02/08/09 | 5 | 0 | no | **Behavioral** (real binary, cross-process) | ✓ Sufficient |
| `tests/lints.sh` | CORE-07, CORE-03(b) | 4 checks | 0 | no | **Behavioral** (injects a hazard, observes the block) | ✓ Sufficient |
| `tests/toolchain.sh` | CORE-09, CORE-02 | 5 checks | 0 | no | Fact assertions over graph + manifest | ✓ Sufficient |

**Disabled tests on requirements:** 0 — `grep -rn "#\[ignore\]"` over `src/` and `tests/` returns nothing.
**Circular patterns detected:** 0. The provenance expected-values question was checked specifically:
expected values come from an in-repo **graded research table** (`.planning/research/SUMMARY.md`)
transcribed verbatim, never from the system under test, and the rows are honestly marked
`UNVERIFIED` with a test (`attributed_rows_are_still_marked_unverified`) that fails on any upgrade.
That is the opposite of a circular test — it is a deliberate refusal to manufacture an oracle.
**Insufficient assertions:** 0.
**Notable strength:** every guard whose value is that it *fires* has a negative test — the lint
wall, the `should_panic` families, and the no-silent-upgrade provenance test. This is exactly the
property the phase's core value demands, and it is present.

### Requirements Coverage

| Requirement | Source plan(s) | Description | Status | Evidence |
|-------------|---------------|-------------|--------|----------|
| CORE-01 | 01-03 | `Money` newtype over `i64` cents, checked arithmetic panicking in every profile | ✓ SATISFIED | T1, T2, T3 |
| CORE-02 | 01-01 | `[profile.release] overflow-checks = true` | ✓ SATISFIED | T1; `toolchain.sh` 4b; release test observed. Universality clause abstained — T20 |
| CORE-03 | 01-02, 01-04, 01-07 | One master seed via `ChaCha8Rng`; (a) StdRng/SysRng absent, (b) SmallRng/Xoshiro never used | ✓ SATISFIED | T7, T8 — both clauses verified by verifier-run compilation |
| CORE-04 | 01-04 | RNG namespaced into `(seed, tick, agent, purpose)` sub-streams | ✓ SATISFIED | T5 — behavioral isolation test, both channels |
| CORE-05 | 01-04 | Fixed-draw sampling (partial Fisher-Yates), never rejection sampling | ✓ SATISFIED | T6 |
| CORE-06 | 01-05 | Generational `FirmId`, accessors return `Option` | ✓ SATISFIED (spelling divergence → human item 1) | T9 |
| CORE-07 | 01-07 | `clippy.toml` bans hashed collections and the non-deterministic `f64` methods, enforced in CI | ✓ SATISFIED (CI execution → T19) | T13; independent ban-list re-derivation |
| CORE-08 | 01-01 | `lib.rs` + thin `main.rs` so integration tests reach all code | ✓ SATISFIED | T15 |
| CORE-09 | 01-01 | `Cargo.lock` + `rust-toolchain.toml` committed; no rayon; no `target-cpu=native` | ✓ SATISFIED | T14 |
| CORE-10 | 01-02, 01-05, 01-06 | Every sim/economic parameter from TOML with `deny_unknown_fields`, no serde defaults; named carve-out recorded | ✓ SATISFIED | T11, T12, T16 |
| CORE-11 | 01-02, 01-08 | (a) source-grade annotation enforced by test — **delivered**; (b) paper verification — **gated on Phase 6** | ✓ SATISFIED (a) / DEFERRED (b) | T17, T18 |

**Orphaned requirements: none.** `REQUIREMENTS.md` maps exactly CORE-01…CORE-11 to Phase 1, and all
eleven appear across the plan frontmatter (`01-01` → 02/08/09, `01-02` → 03/10/11, `01-03` → 01,
`01-04` → 03/04/05, `01-05` → 06/10, `01-06` → 10, `01-07` → 07/03, `01-08` → 11). Union = 11/11.
The traceability table still shows all three amended requirements mapped to Phase 1 and the v1 count
unchanged at 87, which is what the 01-02 prohibition required.

### Prohibitions

| Prohibition (plan) | Tier | Status | Evidence |
|--------------------|------|--------|----------|
| Must not misstate the run's inputs — the seed printed and persisted is the **effective** seed (01-01) | test | ✓ HONOURED | `main.rs` computes `cli.seed.unwrap_or(params.sim.seed)` and prints exactly that. `runs_end_to_end` asserts `effective_seed=7` while the shipped config's own seed is `42` — so the test would fail if the config value were printed. |
| Must not amend a CORE requirement so a failing gate reads as passing without committing the rationale in the same diff (01-02) | judgment | ✓ HONOURED | All three amendment commits (`7508d1c`, `2d6dc06`, `c196193`) each add exactly one `*Rationale (amended …)*` block alongside the amended text; `c196193` touches REQUIREMENTS.md **and** ROADMAP.md in one commit. The verifier further confirmed each amendment rests on a **true, independently checked fact**: `SmallRng` really does compile under this feature set (CORE-03), the three carve-out constants really are non-economic and their record really exists (CORE-10), and clause (b) really is recorded as a Phase 6 gate rather than dropped (CORE-11). No gate was quietly moved. |
| Must not weaken, exempt, alias around or narrow a determinism ban to make a build pass (01-07) | test | ✓ HONOURED | `tests/lints.sh` check 4a/4b/4c: no type alias to a hashed collection (regex widened to `pub(crate)` etc. by the WR-04 fix), **no file** carries `allow`/`expect` for `disallowed_types`, `disallowed_methods`, `clippy::all` or `warnings`, and `src/lookup.rs` (the declined escape hatch) does not exist. The `sqrt`-based `pow_frac` is the phase acting on the prohibition's own instruction: add a primitive rather than exempt the ban. |
| Must not write, correct or upgrade an attributed value from model memory; never mark a row verified without a primary-source read (01-08) | judgment | ✓ HONOURED — demonstrably | The live test case is V-3a: a real numeric mismatch was found during code review, and it was recorded as an open item **and deliberately left unresolved**, with the three possible readings enumerated and none chosen. That is the prohibition being obeyed under pressure, which is far stronger evidence than its absence would have been. Backed by the `attributed_rows_are_still_marked_unverified` test. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | `TBD` / `FIXME` / `XXX` / `HACK` / `TODO` / `PLACEHOLDER` / "not yet implemented" | — | **None found** across `src/`, `tests/`, `config/`, `clippy.toml`, `Cargo.toml`, `.github/workflows/`. Debt-marker gate: clean. |
| — | — | `#[ignore]` / skipped tests | — | **None.** |
| — | — | Stub returns (`return null`, empty collections flowing to output) | — | N/A — no rendering path in this phase; the two "empty" returns (`sample_k` on `k == 0`, `shuffle_in_place` on an empty slice) are asserted-correct degenerate cases, not stubs. |

### Decision Coverage

`STATE.md` records D-01 … D-26 as cited across the phase artefacts, and notes (line 133) that the
coverage tool's `:`-splitting heuristic cannot read three CONTEXT.md bullets — D-09, D-23, D-26 —
so coverage was established by direct citation instead. The verifier spot-confirmed the three
unreadable ones are in fact honoured in code: D-09 (`Money::split` ascending-index remainder rule)
is implemented and pinned by two tests plus a proptest; D-26 (the effective seed is the value of
record) is implemented in `main.rs` and asserted in `runs_end_to_end`; D-23 (generated, not typed
ban list) is the subject of the Data-Flow Trace above. **Non-blocking, informational.**

### Human Verification Required

#### 1. CORE-06 spelling divergence — accept and amend, or flag

**Test:** Decide whether to amend the CORE-06 requirement text (and `01-RESEARCH.md` Pattern 5, and
D-03) from `FirmId { slot, gen }` to `FirmId { slot, generation }`, or to accept the divergence with
a standing note.
**Expected:** A recorded decision, ideally before Phase 3 writes the log schema — the pair
`(slot, generation)` becomes the firm's log identity there, and a rename after that point is a
schema migration rather than a search-and-replace.
**Why human:** `gen` is a reserved keyword in Rust edition 2024 (verified — it does not parse as an
identifier), so the divergence is forced by the language, not chosen. The type shape, the derived
total order and the log identity are all unchanged, so nothing is functionally wrong. But it is a
change to requirement text, which only a human should authorise, and `STATE.md` line 94 explicitly
routes it here: *"flagged for human confirmation at verify-phase"*.

#### 2. PROVENANCE item V-3a — decide the tracking route, not the value

**Test:** `bankruptcy.entrant_size_ratio_ppm = 800000` (0.8) while its SOURCE field cites
`BAM size-replacing-firms = 0.2`. Decide where this open item is tracked so it cannot reach Phase 10
unsettled.
**Expected:** V-3a escalated out of `config/PROVENANCE.md` alone — into `STATE.md`'s open-items list
(where its sibling V-4 already sits) and/or a `ROADMAP.md` Phase 10 criterion.
**Why human:** The value itself was **correctly** left unresolved — D-20 forbids settling a
parameter's meaning from model memory, and PROVENANCE enumerates all three possible readings without
choosing. That is the right call and is not the issue. The issue is routing: Phase 6 criterion 6
covers only *"the Lengnick Table 1 rows"*, and a BAM row is not in Lengnick Table 1, while
ROADMAP Phase 10 criterion 5 currently asserts *"sized at 0.8× a trimmed mean of incumbents"* as
settled fact. Nothing therefore forces the check before BANK-04 consumes it. Not a Phase 1 blocker —
Phase 1 consumes no economics — but it should not ride to Phase 10 untracked. See W1.

#### 3. CI workflow has never run on GitHub Actions

**Test:** Push the phase branch and open the Actions run for `.github/workflows/ci.yml`.
**Expected:** All seven steps green. Watch step 1 in particular.
**Why human:** The verifier ran all seven steps locally, in order, and every one passes — so the
*content* of the gate is proven. What is unproven is the runner: `rustup show active-toolchain` on a
fresh `ubuntu-latest` image, where rustup 1.28+ no longer auto-installs the toolchain named by
`rust-toolchain.toml`, which would then make the following `rustup component add clippy rustfmt`
error on an uninstalled toolchain and fail the job before any real check runs. GitHub API access is
disabled in this session (`GitHub access is not enabled for this session`), so no run history could
be read. **Verdict on the adjudication question: this is a human-verification item, not a phase
gap.** Every check the workflow performs has been independently observed to pass; only the
unattended execution is unwitnessed.

#### 4. Backstop abstention — the `overflow-checks` universality claim

**Test:** Decide whether to add a held-out release-profile test that overflows a raw `i64` across an
inlined call boundary and inside a generic, or to downgrade the plan's claim to the single-site
statement the existing evidence supports.
**Expected:** Either a held-out test, or a narrowed claim.
**Why human:** `verification: backstop` truths abstain absent explicit falsifiable evidence.
Presence + wiring never qualifies, and a single `black_box`ed site is not evidence for "every
arithmetic site". Low practical risk; recorded for honesty rather than because a defect is suspected.

---

## Warnings (non-blocking)

**W1 — V-3a is recorded in only one place, and that place does not gate its consumer.**
`config/PROVENANCE.md` open item V-3a was added late, in the fix pass (commit `cbfe606`). Unlike its
sibling V-4 it was **not** escalated to `STATE.md`, and it is not referenced in `ROADMAP.md`
Phase 10. Additionally, `deferred-items.md` closes with *"No deferred items remain open in this
phase"*, which was true when written but is no longer accurate. → Human item 2.

**W2 — the `clippy.toml` generator is not committed, and nothing re-derives the list.**
The file's header documents its sources and the marker sentence precisely enough that the verifier
reproduced the derivation in ten lines of `awk` and got an **exact 66/66 match** — so the "generated,
not typed" claim is *substantively* true today and the artefact is provably correct. What is missing
is a guard: on a future `rust-toolchain.toml` bump, if std adds or removes a method carrying the
unspecified-precision marker, nothing in the repo fails. Note also that `rust-src` is **not** in
`rust-toolchain.toml`'s `components`, so automating the re-derivation in CI would need
`rustup component add rust-src` first. **Assessment: not a gap** — the correctness property the
requirement cares about is verified, and the derivation is reproducible from the committed header.
Recommend committing the generator (or a `tests/` re-derivation check) in the phase that next
touches the pin.

**W3 — the CORE-10 carve-out record has no regression guard.**
`config/PROVENANCE.md` §4 is the artefact the amended CORE-10 points at, and it is present and
correct. But `tests/provenance.rs` deliberately expects **no** row for the three constants (they are
not config keys), so deleting §4 entirely would fail nothing. A three-line assertion that
`PROVENANCE.md` contains a `GRADE: PROJECT` line for each of `POW_FRAC_BITS`, `PPM_SCALE` and
`MILLI_SCALE` would close the requirement's only unguarded clause.

**W4 — the RNG re-entry guard (D-04) is debug-only, by design.**
`Rngs::issued` is `#[cfg(debug_assertions)]`, so a release run cannot detect a double-open of a
sub-stream key. This is a deliberate, documented trade (a decade-long run opens millions of keys and
the `BTreeSet` would grow unbounded), and the two guard tests are correspondingly
`#[cfg(debug_assertions)]` — which is the whole 139-vs-137 test-count delta. Carry it forward: from
Phase 3 the tick loop starts opening keys at volume, and a key collision would be silent in the
release binary that produces the acceptance run.

**W5 — the `cargo tree` must-have wording is narrower than its guard.**
01-01's truth says *"`cargo tree` shows no OS-entropy crate"*. Plain `cargo tree` **does** show
`getrandom` (twice). `tests/toolchain.sh` correctly scopes to `cargo tree --edges normal` and says so
in its own comment and its success message ("no OS-entropy crate **on the behaviour path**"), and the
verifier confirmed both `getrandom` instances reach the graph only through `proptest`, a
dev-dependency that cannot be linked into the sim binary. The guard is right; the plan's wording is
loose. Recorded so a future reader does not "fix" the guard to match the sentence.

---

## Gaps Summary

**No gaps.** Nothing FAILED. Every one of the eleven CORE requirements is satisfied by code that the
verifier executed, and the four properties whose value lies in *firing* — the money-overflow panic,
the lint wall, the stale-identity miss and the no-silent-upgrade provenance test — were each observed
to fire, two of them by corruptions the verifier introduced itself and reverted.

The phase's own standard was that a must-have asserted in prose but not enforced by a demonstrable
guard should be reported as unverified. Applying that standard, exactly two truths fall short and
both are honestly bounded rather than hidden:

- **T19 (CI on the runner)** — the gate's *content* is fully proven locally; only its unattended
  execution is unwitnessed, and the specific risk (`rustup show active-toolchain` on a fresh image)
  is named rather than hand-waved.
- **T20 (`overflow-checks` universality)** — a `verification: backstop` truth with real but
  single-site evidence; abstained rather than inflated.

Two further items need a human decision rather than more code: the forced `gen` → `generation`
spelling divergence, and where V-3a is tracked so it cannot reach Phase 10 unsettled.

**On the code review.** Sixteen defects (3 Critical, 13 Warning) were found *after* all eight plans
self-reported PASSED, which is exactly why this verification re-ran everything rather than reading
SUMMARY.md. All sixteen fixes hold: each is a separate commit, each repaired guard was proven to bite
by corrupting the tree, and the test count moved only upward (112 → 139 debug, 110 → 137 release).
The three Criticals were all real, and all three were the same species — *a guard that returned a
plausible wrong number instead of failing* (`split` aborting on a valid amount, `load` accepting a
NaN demand, `live_ids` truncating a slot index). That species is precisely what this project exists
to eliminate, and the phase now has unconditional `assert!`s at each of those boundaries rather than
`debug_assert!`s that vanish in the profile that produces the acceptance run.

**Recommendation: proceed to Phase 2** once the four human items are dispositioned. None of them
blocks Phase 2's work (the ledger consumes `Money`, `ids` and the RNG facade, all fully verified),
and item 1 should be settled before Phase 3 fixes the log schema.

---

_Verified: 2026-08-31T01:12:23Z_
_Verifier: Claude (gsd-verifier)_
