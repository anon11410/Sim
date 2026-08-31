---
phase: "1"
slug: "primitives-and-the-determinism-spine"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-08-30"
validated: "2026-08-31"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (unit + integration), `proptest` 1.11.0 for properties |
| **Config file** | none required — `Cargo.toml` `[dev-dependencies]`; `.proptest-regressions` persists counterexamples |
| **Quick run command** | `cargo test --all-targets` |
| **Full suite command** | `cargo test --all-targets && cargo test --release --all-targets && bash tests/lints.sh && bash tests/toolchain.sh` |
| **Estimated runtime** | ~4 s warm (0.8 s debug · 0.5 s release · 2.5 s lints · 0.2 s toolchain) |

**Both profiles are mandatory, not redundant.** CORE-01 and CORE-02 are
profile-dependent: a default release build silently wraps `i64`, so a
debug-only run cannot observe the bug those requirements exist to prevent.
Debug reports 142 tests, release 140 — the two-test difference is the
debug-only pair that has no release counterpart, not a skipped assertion.

---

## Sampling Rate

- **After every task commit:** `cargo test --all-targets` (~0.8 s)
- **After every plan wave:** full suite command above (~4 s)
- **Before `/gsd-verify-work`:** full suite green in **both** profiles
- **Max feedback latency:** ~4 s

---

## Per-Task Verification Map

| Requirement | Secure Behavior | Test Type | Evidence File(s) | Automated Command | Status |
|---|---|---|---|---|---|
| CORE-01 | `Money` panics on overflow in every profile; `split` conserves every cent | unit + property | `src/money.rs` (26 tests), `tests/money_props.rs` (4 proptests) | `cargo test --all-targets` | ✅ green |
| CORE-02 | Release profile checks overflow; removing the setting turns the suite red | integration | `tests/tracer_end_to_end.rs`, `tests/toolchain.sh` | `cargo test --release --all-targets` | ✅ green |
| CORE-03 | Only `ChaCha8Rng`; non-portable generators do not resolve and are grep-banned | unit + lint | `src/rng.rs`, `tests/determinism_rng.rs`, `tests/lints.sh` | `cargo test --all-targets && bash tests/lints.sh` | ✅ green |
| CORE-04 | An added draw in one sub-stream provably cannot perturb another | unit | `src/rng.rs`, `tests/determinism_rng.rs` | `cargo test --all-targets` | ✅ green |
| CORE-05 | Fixed-draw sampling only — no rejection loop on the behaviour path | unit | `src/rng.rs`, `tests/determinism_rng.rs` | `cargo test --all-targets` | ✅ green |
| CORE-06 | Generational `FirmId`; a stale ID after respawn is a typed miss | unit | `src/ids.rs`, `tests/ids_generational.rs` | `cargo test --all-targets` | ✅ green |
| CORE-07 | Hashed collections and the 60 resolvable non-deterministic float/clock methods are blocked | lint | `src/rng.rs`, `tests/lints.sh` | `bash tests/lints.sh` | ✅ green |
| CORE-08 | `lib.rs` + thin `main.rs` — `tests/` can reach all code | integration | `src/lib.rs`, `src/main.rs`, 3 integration test files | `cargo test --all-targets` | ✅ green |
| CORE-09 | `Cargo.lock` + `rust-toolchain.toml` tracked; no rayon, no codegen override | integration | `tests/toolchain.sh` | `bash tests/toolchain.sh` | ✅ green |
| CORE-10 | `deny_unknown_fields`, no serde defaults, money parsed without float round-trip | unit | `src/config.rs`, `src/numeric.rs`, `tests/config_strict.rs` | `cargo test --all-targets` | ✅ green |
| CORE-11 | Every config value carries a source grade; provenance is machine-checked | integration | `tests/provenance.rs`, `tests/config_strict.rs` | `cargo test --all-targets` | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

**11 of 11 requirements have automated verification.** Coverage was confirmed by
requirement-ID grep across `src/` and `tests/`, not by reading plan summaries.

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. `cargo test` is built in;
the one dev-dependency (`proptest`) was added during the phase and is in the
committed lockfile.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Lengnick Table 1 parameter spot-check | CORE-11 | The source PDF is unreachable from this environment (egress-blocked). `tests/provenance.rs` mechanically enforces that every value *carries* a grade and that grades are internally consistent — it cannot confirm a grade-B value matches the published table. | Obtain `econstor.eu/bitstream/10419/45012/1/654079951.pdf`, then check α, γ, θ, υ, δ, φ, ϑ, χ, n, ζ, ψ, β, π against Table 1 and reconcile `config/PROVENANCE.md`. |
| `entrant_size_ratio_ppm` adjudication (open item **V-3a**) | CORE-11 | The shipped value `800000` (0.8×) contradicts its own cited source `size-replacing-firms = 0.2`. Three readings are possible and are not equivalent; D-20 forbids resolving it from model memory. | Decide which reading is correct, record it in `config/PROVENANCE.md`, and regrade the row if it turns out to be derived rather than read. **Blocking gate before Phase 10's BANK-04 consumes the value.** |

Both are traceability obligations on a *value's provenance*, not gaps in
behavioural coverage — the behaviour that reads these values is fully tested.
Neither blocks Phase 1; V-3a blocks Phase 10.

---

## Validation Audit 2026-08-31

| Metric | Count |
|--------|-------|
| Requirements in phase | 11 |
| Automated (COVERED) | 11 |
| Gaps found | 1 |
| Resolved | 1 |
| Escalated to manual-only | 2 |

**Gap found and closed:** CORE-01 was fully covered by 26 unit tests and 4
property tests but carried no requirement-ID tag, unlike CORE-02…CORE-11 — so
the coverage was real but not greppable, and a future audit would have recorded
it as MISSING. Closed by tagging `src/money.rs` and `tests/money_props.rs` to
match the convention already used by the other ten. Doc-comment only; the suite
was re-run in both profiles afterwards and is unchanged at 142 / 140.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references — none remained
- [x] No watch-mode flags
- [x] Feedback latency < 5 s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-31
