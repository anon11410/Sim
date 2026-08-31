---
phase: "2"
slug: "books-journal-and-invariants"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-08-31"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `02-RESEARCH.md` § Validation Architecture, which was compiled and
> executed on the pinned rustc 1.94.1 rather than reasoned about.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `libtest` (`cargo test`), rustc 1.94.1, plus `proptest` 1.11.0 for properties |
| **Config file** | `Cargo.toml` `[dev-dependencies]`; regressions in `.proptest-regressions/`, committed |
| **Quick run command** | `cargo test --locked --lib books invariants` |
| **Full suite command** | `cargo test --locked --all-targets && cargo test --locked --release --all-targets && bash tests/lints.sh && bash tests/toolchain.sh && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` |
| **Estimated runtime** | ~5 s warm |

**No new dependency is installed in this phase.** `proptest` 1.11.0 and `thiserror`
2.0.20 are already in the committed lockfile; CI runs `--locked`, so a `cargo add`
here would be a defect.

**The release profile is the primary one for this phase, not a duplicate run.**
LEDG-10's entire claim is about what the *release binary* contains. Verified this
research pass: `overflow-checks = true` does **not** enable `debug_assertions` —
they are independent Cargo flags, and a `debug_assert!` body was confirmed not
evaluated under this repo's exact release profile.

---

## Sampling Rate

- **After every task commit:** `cargo test --locked --lib books invariants` (sub-second)
- **After every plan wave:** `cargo test --locked --all-targets && cargo test --locked --release --all-targets`
- **Before `/gsd-verify-work`:** the full suite command above, all six steps green — matching CI exactly
- **Max feedback latency:** ~5 s

---

## Per-Task Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists | Status |
|---|---|---|---|---|---|
| LEDG-01 | Balances private to `books`; no `set_cash`, no `&mut Money` escapes | source guard + unit | `bash tests/lints.sh` | ❌ W0 | ⬜ pending |
| LEDG-02 (1) | A shared borrow held across a `transfer` is `E0502` | compile-fail probe | `bash tests/lints.sh` | ❌ W0 | ⬜ pending |
| LEDG-02 (2) | No `&mut Books` method takes a callback | source guard | `bash tests/lints.sh` | ❌ W0 | ⬜ pending |
| LEDG-02 (3) | `books.rs` names no interior-mutability type | source guard + clippy | `cargo clippy --all-targets --all-features -- -D warnings` | ❌ W0 | ⬜ pending |
| LEDG-02 (4) | A failing transfer leaves the books byte-identical | integration (`catch_unwind`) | `cargo test --release --test ledger_atomicity` | ❌ W0 | ⬜ pending |
| LEDG-03 | `transfer` returns the amount moved, equal to the books' delta | property | `cargo test --release --test ledger_props transfer_return_matches_delta` | ❌ W0 | ⬜ pending |
| LEDG-03 | `Money::split` parts sum exactly to the whole | property | `cargo test --lib money::split` | ✅ Phase 1 | ✅ green |
| LEDG-04 | Total money equals opening stock after every tick, any posting sequence | property | `cargo test --release --test ledger_props conservation_under_random_postings` | ❌ W0 | ⬜ pending |
| LEDG-04 | A seeded dropped cent yields exactly `Violation::MoneyConservation` | negative unit | `cargo test --release --lib invariants::negative` | ❌ W0 | ⬜ pending |
| LEDG-05 | `produced − consumed − Σ stock == 0` per good, in both consumption models | unit + property | `cargo test --release --test ledger_props goods_identity_holds` | ❌ W0 | ⬜ pending |
| LEDG-06 | No account holds negative cash, stock or headcount | unit + negative unit | `cargo test --release --lib invariants::negative` | ❌ W0 | ⬜ pending |
| LEDG-07 | Every units-bearing posting moves equal cash the other way | unit + negative unit | `cargo test --release --lib invariants::negative` | ❌ W0 | ⬜ pending |
| LEDG-08 | Zero-transaction tick fails gated on, passes gated off | unit (both directions) | `cargo test --release --lib invariants::liveness` | ❌ W0 | ⬜ pending |
| LEDG-08 | The config gate is read exactly once, at construction | source guard | `bash tests/lints.sh` | ❌ W0 | ⬜ pending |
| LEDG-09 | The reported posting is the **first** non-conserving one, including when a later posting heals the residual | unit (cancelling-residual case explicitly) | `cargo test --release --lib invariants::localise` | ❌ W0 | ⬜ pending |
| LEDG-09 | Every `Violation` `Display` names tick, agent and posting | unit over all variants | `cargo test --release --lib invariants::message` | ❌ W0 | ⬜ pending |
| LEDG-10 | No `debug_assert` / `cfg(debug_assertions)` in `books.rs` or `invariants.rs` | source guard | `bash tests/lints.sh` | ❌ W0 | ⬜ pending |
| LEDG-10 | The invariant phase returns `Result` and a tick loop **aborts** at the right tick | integration | `cargo test --release --test invariant_halt` | ❌ W0 | ⬜ pending |
| LEDG-10 | All negative tests pass **in the release profile** | profile coverage | `cargo test --locked --release --all-targets` | ✅ CI | ⬜ pending |
| order contract | `ALL_CHECKS` is in the documented `CheckId` order | unit | `cargo test --release --lib invariants::order` | ❌ W0 | ⬜ pending |
| config | The new key is required, annotated, and has a provenance row | existing tests, re-run | `cargo test --test config_strict --test provenance` | ✅ exists | ⚠️ will fail until all four config files are updated |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/books.rs` — the module, with its `#[cfg(test)]` corruption vocabulary (LEDG-01/02/03/07)
- [ ] `src/invariants.rs` — `Violation`, `CheckId`, `ALL_CHECKS`, `CheckSet`, localisation (LEDG-04…10)
- [ ] `src/lib.rs` — `pub mod books;` and `pub mod invariants;`, keeping flat alphabetical order
- [ ] `tests/ledger_props.rs` — proptest strategies for a valid `Books` and a random posting sequence
- [ ] `tests/ledger_atomicity.rs` — the `catch_unwind` LEDG-02 test
- [ ] `tests/invariant_halt.rs` — the library-level tick loop that must abort
- [ ] `tests/lint-probes/books_borrow_probe.rs.txt` — the `E0502` compile-fail probe
- [ ] `tests/lints.sh` — extended with the borrow probe and four grep guards, each asserting a non-empty search set
- [ ] `clippy.toml` — interior-mutability `disallowed-types` entries
- [ ] `config/baseline.toml` + `src/config.rs` + `config/PROVENANCE.md` — the `[invariants]` table (**all three**, or the schema-leaf agreement test fails)
- [ ] Framework install: **none needed**

---

## Manual-Only Verifications

All phase behaviors have automated verification. This phase introduces no
user-observable behavior — it is pure infrastructure with no UI, no network and
no external service.

---

## Two Constraints the Planner Must Budget For

1. **The liveness config key is a four-file change** — `config/baseline.toml` (with
   its two-line `# GRADE:` block), `src/config.rs`, `config/PROVENANCE.md`, plus the
   schema-leaf agreement — policed by three existing tests that will go red until
   all four are done together.
2. **`src/books.rs` and `src/invariants.rs` may not spell `f64` or `f32` anywhere,
   including doc comments.** `tests/numeric_det.rs` reads whole lines and its
   allowlist is exactly `["numeric.rs", "config.rs"]`.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
