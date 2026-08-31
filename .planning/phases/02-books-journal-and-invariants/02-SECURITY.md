---
phase: "02"
slug: "books-journal-and-invariants"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity
threats_open: 0
asvs_level: 1
created: "2026-08-31"
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: **authored at plan time** — all 7 plans carried a parseable
`<threat_model>` block, so this is mitigation *verification*, not a retroactive
STRIDE scan. ASVS level 1, block threshold `high`.

**Domain note.** This phase is an in-process, single-threaded computation over a
local TOML file. There is no network, no principal, no session, no persistence and
no untrusted input beyond the config file Phase 1 already validates. The dominant
STRIDE category is therefore **Tampering** (28 of 41) — and here "tampering" means
value being created, destroyed or duplicated inside the ledger, which is the same
thing as the phase's correctness goal. The security register and the conservation
invariants are largely the same control surface.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| public ledger API → `Books` internal state | `transfer` / `produce` / `consume` / `exchange` are the only entries; balances are private with no `set_cash` | Money (integer cents), goods units |
| `#[cfg(test)]` corruption vocabulary → `Books` | Five `pub(crate) corrupt_*` methods write state the public API cannot reach | Deliberately invalid ledger state |
| `Books` → `Violation` → stderr | A halt message is rendered and read next to a diffed log | Tick, agent id, cents, units |
| config file bytes → `Params.invariants` | The new `liveness_enabled` leaf, `deny_unknown_fields`, no serde defaults | Boolean gate |
| journal buffer → invariant checks | Per-tick postings accumulated, checked, localised, cleared | Postings with two cash and two unit legs |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-02 | Tampering | the liveness gate as a silent off-switch | high | mitigate | The gate ships `false` only because Phase 3 needs it off, and three separate records stop that becoming permanent: the new Phase 3 criterion | closed |
| T-02-05 | Tampering | `Books::transfer` arithmetic | high | mitigate | Every money operation routes through the named result-returning half of the money API; the operators panic in every profile and the release  | closed |
| T-02-06 | Tampering | panic between the two balance writes | high | mitigate | Compute-then-commit: every fallible step completes before any write, and the commit step contains only assignments. Measured: the naive orde | closed |
| T-02-07 | Tampering | a mid-transaction observer through a callback | high | mitigate | No method that borrows the books mutably takes a callback of any kind, and `Books` holds no field of such a type. Reproduced: a hook of that | closed |
| T-02-08 | Tampering | value mutation from outside the ledger | high | mitigate | Every balance field is private, there is exactly one constructor, and no accessor returns a mutable reference to a balance or a balance vect | closed |
| T-02-11 | Spoofing | a stale firm identity resolving to a different firm | high | mitigate | Balances are keyed on the slot but a firm account resolves only when its generation matches the ledger's record for that slot, so an identit | closed |
| T-02-12 | Tampering | the conservation baseline derived from what it checks | high | mitigate | `opening_stock` is set once in `new` from the configured total money, never from a sum over the balances, and construction fails with a name | closed |
| T-02-13 | Tampering | units created or destroyed outside a posting | high | mitigate | Every unit movement is a posting, and the running produced and consumed totals are advanced from the operation's arguments while the residua | closed |
| T-02-14 | Tampering | a half-applied cash-for-units swap | high | mitigate | `exchange` is one posting with a compute step that validates cash and stock before any write, and a commit step containing only assignments. | closed |
| T-02-15 | Tampering | a vacuous goods check | high | mitigate | The identity is compared against a second, separately maintained source rather than derived from the journal at check time; a single-source  | closed |
| T-02-18 | Tampering | a vacuous non-negativity check | high | mitigate | The check walks cash and stock, both of which are signed and can genuinely go negative, and plan 02-05 drives it with a seeded corruption. H | closed |
| T-02-19 | Tampering | a non-zero-sum posting passing unnoticed | high | mitigate | A posting carries two cash amounts and two unit amounts, so an imbalance is expressible as data and is checked per posting for every kind, i | closed |
| T-02-21 | Tampering | a check silently dropped from the table | high | mitigate | `CheckId::ALL` is the single source of truth, the order test reads the table rather than a second list, and an exhaustive pattern match make | closed |
| T-02-23 | Elevation of Privilege | fault-injection surface reachable in a shipped build | high | mitigate | The vocabulary is gated on the crate's own test configuration, which was verified unreachable from an integration test and therefore from ev | closed |
| T-02-24 | Tampering | a negative test that passes for the wrong reason | high | mitigate | Every violation is asserted by whole-value equality including tick, numbers and posting; message substring matching is confined to the modul | closed |
| T-02-25 | Tampering | a fault that bypasses the production check path | high | mitigate | Every corruption that records a posting routes through the same private recorder as a real posting, so the residual arithmetic under test is | closed |
| T-02-26 | Repudiation | a violation named against the wrong posting | high | mitigate | The localisation test reproduces the measured cancelling-residual case and asserts both that the early posting is reported and that the late | closed |
| T-02-28 | Tampering | a public operation sequence that conjures or destroys money | high | mitigate | Conservation is asserted after every operation of every generated sequence, both directly from the balances and through the check set, with  | closed |
| T-02-29 | Tampering | a returned amount that disagrees with what moved | high | mitigate | The return-agreement property compares the returned value against the observed change in both balances on success, and asserts nothing moved | closed |
| T-02-30 | Tampering | the two conservation sources silently collapsing into one | high | mitigate | The agreement property asserts the posting-derived residual against the balance-derived quantity directly, so a change that derives one from | closed |
| T-02-32 | Tampering | a residual reset at the tick boundary | high | mitigate | The tick-boundary property asserts that ending a tick empties the journal and the transaction count while leaving both running residuals and | closed |
| T-02-33 | Tampering | a mid-transaction observer through a callback | high | mitigate | Guard 7a bans every closure, trait-object and function-pointer parameter type in the ledger, with the measured counterexample stated in the  | closed |
| T-02-34 | Tampering | a mid-transaction observer through shared mutability | high | mitigate | Guards 7b and 7c plus the clippy entries from task 2, with the one permitted site pinned to a single named file. The crate-wide prohibition  | closed |
| T-02-35 | Tampering | a panic between two balance writes | high | mitigate | Task 1's executable test, standing next to the naive mutant it discriminates against. Measured: minus 400 against an opening 100 for the nai | closed |
| T-02-36 | Elevation of Privilege | fault-injection surface reachable by a consumer | high | mitigate | Check 6 copies a probe that calls a corruption method from an integration test and asserts the build fails with the no-such-method diagnosti | closed |
| T-02-37 | Tampering | an invariant compiled out of the binary that produced a run | high | mitigate | Guard 7d bans the debug-only assertion vocabulary in both ledger modules, with a fixture proving the pattern catches the banned spelling and | closed |
| T-02-38 | Tampering | a balance written outside the ledger | high | mitigate | Guard 7f confines the balance identifiers to one file and guard 7g bans a mutable-reference return, which is the escape no search for a sett | closed |
| T-02-40 | Tampering | a guard that silently matches nothing | high | mitigate | Every guard's pattern is asserted to match a known hazard fixture before it is asserted absent from the tree, and every guard asserts its se | closed |
| T-02-SC | Tampering | npm/pip/cargo installs | high | mitigate | No package-manager install occurs in this plan or this phase. 02-RESEARCH.md § Package Legitimacy Audit records zero new packages, and CI's  | closed |
| T-02-01 | Tampering | `Params::invariants` deserialisation | medium | mitigate | `#[serde(deny_unknown_fields)]` on the new struct and no serde default, so a misspelt key is a hard error rather than a silently-disabled in | closed |
| T-02-03 | Information Disclosure | `Display` on the address types | medium | mitigate | The rendered forms carry only integer identifiers — no path, host name, wall-clock reading or process id. Pinned by string-equality unit tes | closed |
| T-02-09 | Denial of Service | unbounded journal growth over a decade | medium | mitigate | The journal is a per-tick buffer cleared by `end_of_tick` with the vector's clear operation, so capacity is reused and the allocation is bou | closed |
| T-02-10 | Information Disclosure | a halt message leaking environment | medium | mitigate | Every interpolated field of every `Violation` variant is a number, an identity or a posting. No path, host name, wall-clock reading or proce | closed |
| T-02-16 | Tampering | stock index confusion across a firm respawn | medium | mitigate | Stock is keyed on the firm slot, matching cash, and a firm account resolves only when its generation matches the ledger's record, so a stale | closed |
| T-02-17 | Tampering | integer overflow in unit arithmetic | medium | mitigate | Unit counts are signed 64-bit and the release profile enables overflow checks, so a wrap aborts rather than producing a plausible negative i | closed |
| T-02-20 | Repudiation | a nondeterministic answer to "which account is negative" | medium | mitigate | The walk order is fixed and documented — households ascending, then firm slots ascending, cash before stock — resting on the derived total o | closed |
| T-02-22 | Information Disclosure | the two new variants leaking environment into a halt message | medium | mitigate | Both detail enums carry only integers and identities; no owned string, path, host name, wall-clock reading or process id can enter them. Pla | closed |
| T-02-27 | Information Disclosure | a halt message carrying environment to stderr | medium | mitigate | The message module asserts no rendered violation contains a path separator; plan 02-06 adds the source-level guard over the whole module. | closed |
| T-02-31 | Repudiation | a rare counterexample discarded after one run | medium | mitigate | Failure persistence is configured to a file in the committed regression directory, and one verify command asserts the file is tracked, so a  | closed |
| T-02-39 | Information Disclosure | environment values in a halt message | medium | mitigate | Guard 7h bans path, clock and process types in the violation module; plan 02-05's message module asserts the same rule at runtime. Neither i | closed |
| T-02-04 | Repudiation | provenance drift between the config and its record | low | mitigate | `no_annotation_is_orphaned` and `every_config_key_has_a_provenance_row` fail if the three files disagree, so the new key cannot exist withou | closed |
*Status: open · closed · open — below high threshold (non-blocking)*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**41 unique threats** across 7 plans — 29 high, 11 medium, 1 low; **all 41 `mitigate`, all closed.**
No threat in this phase was dispositioned `accept` or `transfer`.

By category: Tampering 28 · Information Disclosure 5 · Repudiation 4 · Elevation of
Privilege 2 · Spoofing 1 · Denial of Service 1.

---

## Accepted Risks Log

No accepted risks. Every threat in this phase carries an implemented mitigation.

---

## Verification Evidence

Mitigations were verified present **and enforced**, not asserted from the plan summaries.

| Control | Evidence |
|---------|----------|
| Memory safety (Elevation of Privilege) | `#![forbid(unsafe_code)]` crate-wide — enforced by compile failure |
| No test-only surface in production | Corruption vocabulary is `#[cfg(test)] pub(crate)`; `E0599` compile-fail probe proves it unreachable from `tests/`; **no `[features]` table exists** (verified: `grep -c '^\[features\]' Cargo.toml` = 0) |
| Probe keeps pace with the vocabulary | Guard 7j pins the probe's call count to the declaration count — it **fired** during gap closure when a fifth corruption method was added with only four probe lines |
| Invariants present in the shipped binary | Zero `debug_assert` occurrences in `src/books.rs` and `src/invariants.rs`; full suite green under `--release`, the primary profile for this phase |
| Determinism (Tampering) | Zero `HashMap`/`HashSet` occurrences in the ledger; zero `f64`/`f32` occurrences including doc comments; single-threaded; no `rayon` |
| Information disclosure in halt messages | `Violation` renders integer identifiers only — no path, hostname, wall-clock or PID, which is simultaneously the TICK-06 determinism requirement |
| Overflow (Tampering) | `Money`'s checked operators panic in every profile; `overflow-checks = true` in release; `Books::new`'s money-side gate moved from `saturating_add` to checked arithmetic with a named error (review finding WR-01) |
| Value conservation (Tampering) | Five invariants, **each mutation-proven to fire** on a seeded fault. The last — goods conservation — was found never to have fired and was closed by `e0ee1b4`, re-verified independently by the orchestrator |
| Supply chain | `Cargo.lock` committed; `git diff --exit-code -- Cargo.toml Cargo.lock` clean across the whole phase — no dependency entered |

Suite at audit: **242 tests green in both profiles**, `tests/lints.sh`, `tests/toolchain.sh`,
`cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --check` all clean.

---

## A Note on What the Register Could Not Catch

Five defects of one shape were found in this phase — an assertion whose stated claim is not
what it actually measures. Each read as a healthy green test, and **none was caught by the
threat register**, because a control that is *present but inert* satisfies a register that
asks only whether the mitigation exists.

All five were found by mutation: an empty `exchange` and a zero-cent `transfer` each counting
as a transaction (liveness bypasses), a tick-boundary property that survived zeroing the
residual, a two-source agreement property that was `0 == 0`, and `check_goods` whose entire
body could be deleted with the suite still green.

The register above is therefore necessary but not sufficient, and the phase's real security
property is the mutation discipline rather than the table. Recorded here so a later phase does
not read a clean register as evidence that its controls bite.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-08-31 | 41 | 41 | 0 | /gsd-secure-phase 02 |

---

## Sign-Off

- [x] All threats have a disposition (41 mitigate, 0 accept, 0 transfer)
- [x] Accepted risks documented — none exist
- [x] `threats_open: 0` confirmed
