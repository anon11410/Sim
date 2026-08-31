---
phase: "01"
slug: "primitives-and-the-determinism-spine"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-08-31"
---

# Phase 01 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: **authored at plan time** — all 8 plans carried a parseable
`<threat_model>` block, so this is mitigation *verification*, not a retroactive
STRIDE scan. ASVS level 1, block threshold `high`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| operator argv → process | `--config`, `--seed`, `--out` cross from the shell; `clap` is the only parser | CLI arguments (non-sensitive) |
| config file bytes → typed `Params` | Repository-controlled TOML enters typed values via `serde` + `toml` with `deny_unknown_fields` | Simulation parameters, money values |
| crates.io → build graph | Third-party crate source enters the build; `Cargo.lock` is the pin | Dependency source |
| process → run directory | The process creates a directory at the operator-supplied `--out` path | Log artifacts |
| pinned std source → `clippy.toml` | Ban lists generated from the toolchain's own source, never typed from memory | Method/type deny lists |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-1-01 | Tampering | Raw `i64` arithmetic on the behaviour path (`src/*`, release profile) | high | mitigate | `[profile.release] overflow-checks = true` (Task 1) plus the `#[should_panic]` release test and its adjacent non-panicking case (Task 3), so removing  | closed |
| T-1-07 | Repudiation | `.planning/REQUIREMENTS.md` requirement amendments | high | mitigate | Every amendment ships with an inline rationale in the same diff, verified by a grep for the rationale line adjacent to each amended bullet (Tasks 1, 2 | closed |
| T-1-10 | Tampering | `Money::split` remainder handling | high | mitigate | Property tests over amounts that do not divide evenly assert the parts sum exactly to the whole, plus a spread bound of one cent (Tasks 2 and 3). Sile | closed |
| T-1-12 | Tampering | Sub-stream key packing (`pack_stream_key`) | high | mitigate | A non-debug `assert!` on each field width with a message naming the field, plus a swept injectivity test over 40 x 40 x every purpose (Tasks 2 and 3). | closed |
| T-1-13 | Tampering | Sub-stream key re-entry | high | mitigate | Re-entering a key was verified to replay identical values. A `#[cfg(debug_assertions)]` ordered set of issued keys panics on a duplicate, naming the d | closed |
| T-1-14 | Spoofing | Ambient entropy reaching the behaviour path | high | mitigate | The crate's feature set (`default-features = false`, `features = ["std","chacha"]`) makes the system-entropy RNG and the process-local generator not r | closed |
| T-1-15 | Spoofing | A stale `FirmId` resolving to the new occupant of a reused slot | high | mitigate | The generation is part of the identity and both accessors compare it before returning; a stale identity resolves to `None`, asserted by `stale_identit | closed |
| T-1-16 | Tampering | Non-deterministic transcendental precision on the behaviour path | high | mitigate | Fractional powers are computed from the square root and multiplication only — both IEEE-754 correctly rounded — with bit-identity asserted over 100,00 | closed |
| T-1-19 | Tampering | A parameter defaulting silently instead of loading from the file | high | mitigate | `deny_unknown_fields` on every struct, no serde default attribute anywhere under `src/`, no optional field type in the schema, and an exhaustive delet | closed |
| T-1-22 | Tampering | A determinism hazard introduced in the test directory | high | mitigate | The lint invocation everywhere — locally, in the guard script and in CI — carries the flags that lint every target. Verified: the bare invocation exit | closed |
| T-1-23 | Tampering | A configured ban that silently does nothing | high | mitigate | The list is generated from the toolchain's own source rather than typed, and a probe exercises every resolvable entry with its diagnostic count compar | closed |
| T-1-24 | Tampering | An alias or exemption neutralising the type ban | high | mitigate | The guard script asserts no type alias to a hashed collection exists under `src/`, no file carries the disallowed-types exemption attribute, and no po | closed |
| T-1-25 | Repudiation | A gate weakened to let work through | high | mitigate | Recorded as the prohibition on this plan, and structurally mitigated by having provided a replacement rather than an exemption: `src/numeric.rs` gives | closed |
| T-1-26 | Spoofing | An attributed parameter value written from model memory | high | mitigate | Every value is transcribed in-session from the in-repo graded table, never recalled; every row attributed to the baseline-model paper carries the unve | closed |
| T-1-27 | Repudiation | A parameter whose origin cannot be established later | high | mitigate | One annotation block per leaf key carrying grade, source and cadence, enforced by a test that names the offending key on failure, plus a provenance ro | closed |
| T-1-29 | Repudiation | A deferred requirement clause recorded in only one of the two files th | high | mitigate | Task 4 amends the CORE-11 bullet and Phase 1 Success Criterion 5 in the same diff, each citing D-19, and adds the gate to the Phase 6 criteria so the  | closed |
| T-1-SC | Tampering | crates.io installs (`cargo add` / first `cargo build`) | high | mitigate | Package-legitimacy gate satisfied by `01-RESEARCH.md` → "Package Legitimacy Audit": 9 of 9 crates verdict OK (3.5 M–31 M weekly downloads, named first | closed |
| T-1-03 | Denial of service | An absurd config-supplied amount reaching a panicking operator | medium | mitigate | The named `Result`-returning API (`checked_add`, `checked_sub`, `try_scale`) exists precisely so `src/config.rs` reports an out-of-range supplied amou | closed |
| T-1-06 | Elevation of privilege | First-party source memory safety | medium | mitigate | `#![forbid(unsafe_code)]` in `src/lib.rs` (Task 1). Auditable by compile failure, not by review. | closed |
| T-1-08 | Tampering | `.claude/CLAUDE.md` factual claims | medium | mitigate | The corrected rows cite `01-RESEARCH.md` Pitfall 1, whose evidence is crate-source line numbers and two compiler errors reproduced on the pinned toolc | closed |
| T-1-09 | Tampering | Scope of the edits to shared planning artifacts | medium | mitigate | All three files are edited with scoped replacements on named rows, never a whole-file write; the diff-size acceptance criteria on Tasks 3 and 4, and t | closed |
| T-1-11 | Tampering | Float contamination of the money domain | medium | mitigate | No conversion to or from a floating-point type, no float multiplication and no decimal `Display` are implemented; asserted by a source grep in Task 1' | closed |
| T-1-17 | Tampering | Float leakage into modules outside the numeric domain | medium | mitigate | A test reads every source file under `src/` and asserts only `src/numeric.rs` names a floating-point type (Task 3); plan `01-07`'s lint wall enforces  | closed |
| T-1-18 | Tampering | An out-of-range float crossing into the integer domain | medium | mitigate | `demand_to_units` asserts finiteness in debug builds and its cast saturates rather than wrapping; both directions are asserted (Tasks 2 and 3). A wrap | closed |
| T-1-20 | Tampering | Type coercion silently changing a parameter's meaning | medium | mitigate | The parser refuses decimal-to-integer and string-to-integer coercion; both directions are asserted with the exact error substring (Tasks 1 and 3). Thi | closed |
| T-1-21 | Repudiation | A run whose identifying hash does not change when its inputs change | medium | mitigate | The hash is taken over the raw file bytes, not the parsed struct, so a comment change is a hash change — which is correct, because the comments carry  | closed |
| T-1-28 | Tampering | An annotation drifting away from the key it describes | medium | mitigate | `no_annotation_is_orphaned` asserts every annotation line is immediately followed by a key assignment (Task 3). A drifted annotation is worse than a m | closed |
| T-1-04 | Tampering | `--out` path handling in `src/main.rs` | low | accept | Operator-supplied, not attacker-supplied, in a local single-process simulation. Control kept as a construction rule: `--out` is a plain `PathBuf` join | closed |
| T-1-05 | Information disclosure | Tracer stdout line and future run metadata | low | accept | No secrets exist in this system; the seed is a deliberately public run parameter recorded so a run is reproducible from its own record. | closed |
*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `high` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**29 unique threats** across 8 plans — 17 high, 10 medium, 2 low; 27 `mitigate`, 2 `accept`. All closed.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01 | T-1-04 | `--out` is operator-supplied, not attacker-supplied, in a local single-process simulation. Control kept as a construction rule: `--out` is a plain `PathBuf` joined only with fixed filenames, never assembled from config content. | Phase 01 plan author | 2026-08-31 |
| AR-02 | T-1-05 | No secrets exist in this system. The seed is a deliberately public run parameter, recorded precisely so a run is reproducible from its own record. | Phase 01 plan author | 2026-08-31 |

---

## Verification Evidence

Mitigations were verified present **and enforced**, not merely asserted by the plan summaries:

| Control | Evidence |
|---------|----------|
| T-1-01 release overflow checks | `[profile.release] overflow-checks = true` present; three `#[should_panic]` release tests plus an adjacent non-panicking case, all green in the release profile |
| T-1-06 memory safety | `#![forbid(unsafe_code)]` at `src/lib.rs:7` — enforced by compile failure, not review |
| T-1-SC supply chain | `Cargo.lock` tracked in git; `tests/toolchain.sh` asserts no data-parallelism crate, no codegen override, and no OS-entropy crate on the behaviour path |
| Non-portable RNG ban | No `StdRng` / `SmallRng` / `Xoshiro` / `rand::rng()` in `src/`; the only occurrences are inside the guard that bans them |
| Float determinism ban | `clippy.toml` generated from the pinned toolchain's own std/core source (66 float entries across both widths); `tests/lints.sh` check 3 confirms all 60 resolvable bans actually fire |
| Guard adversariality | `tests/lints.sh` check 2 injects a hazard and confirms both lists block it; check 4 confirms no alias, exemption or lookup wrapper escapes the gate |
| Suite health | `cargo test --all-targets` 142 passed / 0 failed; `--release --all-targets` 140 passed / 0 failed |

Toolchain pinned at `rustc 1.94.1 (e408947bf 2026-03-25)` via `rust-toolchain.toml`.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-08-31 | 29 | 29 | 0 | /gsd-secure-phase 01 |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
