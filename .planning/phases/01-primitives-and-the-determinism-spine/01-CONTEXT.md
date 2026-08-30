# Phase 1: Primitives and the Determinism Spine - Context

**Gathered:** 2026-08-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 delivers the project's **vocabulary** — money, identity, configuration, randomness —
with the correctness properties that every later phase depends on and that no later phase can
add cheaply. Concretely: the `Money(i64)` cents newtype, generational `FirmId`, the strict TOML
config loader, the seeded `ChaCha8Rng` sub-stream façade, the hand-rolled fixed-draw samplers,
the `pow_frac_det` numeric primitive, and the clippy/CI wall that makes the determinism bans
fail the build rather than document a preference.

**Zero economics.** ROADMAP Ordering Constraint 1: Phases 1–4 contain no economic rule. Nothing
in this phase prices, hires, produces or trades. Ledger (Phase 2), tick pipeline (Phase 3) and
harness (Phase 4) are also out of scope — this phase builds only the types they are written in.

Requirements in scope: CORE-01 … CORE-11 (11 requirements).

</domain>

<decisions>
## Implementation Decisions

**How these were reached.** The user was presented with eight gray areas surfaced by
`01-RESEARCH.md` §"Decisions a `/gsd-discuss-phase` should settle before planning locks". The
user responded "[No preference]" to both selection rounds and then, explicitly:
*"Im unfamiliar with the field you will have to decide."* Every decision below is therefore
**Claude's, made under delegated authority**, and each is grounded in a first-hand measurement
recorded in `01-RESEARCH.md` rather than in preference. They are recorded as locked so the
planner does not re-litigate them, but any of them may be revisited by the user on request —
the rationale is written out for exactly that reason.

### RNG and determinism

- **D-01: RNG sub-streams are addressed by a bit-packed `set_stream(u64)` nonce, not by hashed
  child seeds.** Layout is `tick:24 | agent:24 | purpose:16` (high bits → low bits).
  Rationale: the packing is **bijective**, so distinct `(tick, agent, purpose)` tuples produce
  distinct nonces by arithmetic rather than by a collision-resistance argument; and it measured
  **237 ms vs 1390 ms** (5.9×) for 3.65 M sub-streams against the SHA-256 child-seed
  alternative. `ChaCha8Rng` gives 2^64 streams per seed, each 1 ZiB long, so the address space
  is not a constraint. — **Reversibility:** one-way — re-keying changes every draw site *and*
  changes the trajectory of every committed run and snapshot, so the golden logs and `insta`
  snapshots would all have to be regenerated and re-reviewed.
- **D-02: The bit allocation is fixed at 24/24/16 and is not to be "tuned" narrower.** The
  realistic need is 12/8/5 bits; the surplus is deliberate headroom so that adding a purpose or
  raising the tick count never forces a re-key. `purpose` is a `#[repr(u16)]` enum with
  explicitly assigned discriminants — **discriminants are append-only and never renumbered**,
  because renumbering silently re-keys history. — **Reversibility:** one-way — same blast
  radius as D-01.
- **D-03: The `agent` field of the nonce carries `FirmId.slot`, never `gen`.** A respawned firm
  in the same slot must not inherit the previous occupant's keystream position, and `gen` must
  not widen the key. Households use their index directly. Interaction with CORE-06 noted by
  research; the planner must not let `FirmId`'s two fields both leak into the key.
- **D-04: Re-entering an already-used sub-stream key is a defect, and the API must make it
  hard.** Research verified the hazard by execution: re-entering the same key **replays the
  same values**. The `Rngs` façade hands out short-lived `Stream` scopes; the planner should
  include a debug-build used-key guard (or an equivalent construction that cannot be re-opened
  within a tick) so a double-open fails loudly in tests rather than silently correlating two
  decisions.
- **D-05: Samplers are hand-rolled and fixed-draw (partial Fisher-Yates).** `rand`'s own
  `random_range`, `Uniform::sample` and `seq::index::sample` were all verified **not**
  fixed-draw. A variable draw count would defeat D-01's isolation guarantee from inside.
- **D-06: `HashMap`/`HashSet` are banned outright on every path, with no `Lookup` escape
  wrapper.** CLAUDE.md permits lookup-only use; this phase declines to build the hatch. Every
  v1 relation is dense-integer keyed (`Vec` by ID) or small enough for `BTreeMap`. Add a
  wrapper only if a later phase demonstrably needs one — not building it is the cheapest way
  to keep the lint honest. — **Reversibility:** reversible — adding the wrapper later is a
  local change.

### Money

- **D-07: `Money` overflow is handled by a split API, resolving the ROADMAP/CLAUDE.md
  conflict.** Operator impls (`Add`, `Sub`, `AddAssign`, `Neg`, `Sum`) route through `checked_*`
  and `.expect(...)` and therefore **panic in every build profile** — satisfying ROADMAP
  criterion 1. A named API (`checked_add`, `checked_sub`, `try_scale`) returns
  `Result<Money, MoneyOverflow>` and is used **only at config ingestion**, where an absurd
  user-supplied `total_money_cents` should produce a named `ConfigError` rather than a panic —
  satisfying CLAUDE.md's `thiserror` table. Verified: the operator panics in debug, in default
  release, and in overflow-checked release. **The planner must state both halves explicitly so
  the executor does not implement one and delete the other.**
- **D-08: `Money` does not implement `Sum` via `fold(0, +)` on the raw `i64`** — it routes
  through the checked `Add`. A raw fold is the one path that would wrap silently.
- **D-09: `Money::split(n)` distributes the remainder to the first `r = amount % n` recipients
  by ascending index**, and the proptest strategy must include a case where `a % n != 0`.
  ROADMAP criterion 1 is explicit about this: without the non-even case, a `vec![a/n; n]`
  implementation that destroys `r` cents passes the test.
- **D-10: `overflow-checks = true` in `[profile.release]` ships anyway, as a second belt.**
  D-07 already holds without it; CORE-02 sets it because raw `i64` arithmetic elsewhere in the
  sim has no such protection — verified that default release **silently wrapped**
  `i64::MAX - 1 + 6`.

### The float boundary

- **D-11: `expected_demand` is `f64`, restricted.** Restricted means: only `+ − × ÷ sqrt`
  (all IEEE-754 correctly-rounded), confined to `src/numeric.rs` plus the field itself, with one
  named crossing function `demand_to_units(f64) -> i64` using `round()` and a saturating `as`
  cast, and a `debug_assert!(x.is_finite())` at the crossing.
  Rationale: the deciding argument is **not** precision (measured divergence from the `i64`
  route over 3,650 ticks is 2.1e-3 units — economically invisible) but that MKT-01's
  `(m/P̄)^0.9` **forces the float domain to exist regardless**. Given it must exist, the `i64`
  milli-unit route's headline benefit evaporates while its costs — rescaling every derived
  formula by hand, an undocumented 0.003-unit dead band, a truncation story — remain.
  — **Reversibility:** costly — the field type propagates into the log schema, the price and
  wage rules (Phase 9) and the harness's dtype assertions (Phase 4).
- **D-12: `pow_frac_det` ships in Phase 1 with `bits = 40`. The `powf` ban is not weakened and
  α is not changed to fit a closed integer form.** `x^α` for `0 < α < 1` is computed by binary
  expansion of the exponent using only `sqrt` and `*`. Measured against `powf`: worst relative
  error **1.9e-12** over 20,000 inputs, and **bit-identical across 100,000 invocations** —
  which is precisely the property `powf` does not have (std: *"precision is non-deterministic …
  can even differ within the same execution"*). This primitive belongs in Phase 1 because Phase
  1 writes the clippy list that bans `powf`, and the list cannot be written honestly without
  deciding how Phase 7 computes its exponent. **This tension is noted in no other planning
  document — the planner should not be surprised to find a Phase 7 concern in a Phase 1 plan.**
- **D-13: `expected_demand` is logged at full round-trip precision, never truncated.**

### Configuration and provenance

- **D-14: Numerical-method constants live in code, not in the TOML config.** `pow_frac_det`'s
  bit count and the milli/ppm scale factors are not economic parameters. They are `const`s in
  `src/numeric.rs`, documented, with a `# GRADE: PROJECT` entry in `config/PROVENANCE.md`
  recording why they are not config. Rationale: putting a Newton-style iteration count into an
  economics config invites someone to tune it, and tuning it silently changes every trajectory.
  This requires the CORE-10 amendment in D-16.
- **D-15: Config strictness is enforced beyond `deny_unknown_fields`.** `Option<T>` is banned in
  config structs — research verified it is a **hidden serde default with no attribute to grep
  for**, so a `grep` for `#[serde(default)]` alone does not satisfy ROADMAP criterion 3. The
  missing-key test is an **exhaustive loop over every field**, not a spot check.
- **D-16: Source-grade annotation follows the scheme already defined in-repo — it is not
  reinvented.** `.planning/research/SUMMARY.md:169` defines A / B / C / PROJECT, and the graded
  37-row parameter table already exists at `SUMMARY.md:171-209`. The config-annotation task is
  **transcription plus a schema**, not research. Convention: a TOML comment block above each key
  carrying `GRADE`, `SOURCE` and `CADENCE`.

### Requirement amendments (user delegated this explicitly)

- **D-17: CORE-03 is amended in `REQUIREMENTS.md`, because it is unpassable as written.** It
  requires `StdRng` **and** `SmallRng` absent from the dependency graph. Research verified from
  crate source and by compiling that `StdRng` can be feature-gated out but **`SmallRng` cannot
  be removed from `rand` 0.10.2**. (CLAUDE.md's claim that `SmallRng` and the `small_rng`
  feature were removed in 0.10 is factually wrong and should be corrected in the same commit.)
  Amended form, split into two testable clauses:
  (a) `StdRng` and `SysRng` are absent from the dependency graph — enforced by the
      `default-features = false, features = ["std","chacha"]` feature set, and verified by a
      test asserting `rand::rng()` / `SysRng` do not compile;
  (b) `SmallRng`, Xoshiro and any other non-portable generator are never *used* — enforced by
      clippy `disallowed_types` plus a grep test.
  Without this amendment the Phase 1 gate reads as failed forever.
- **D-18: CORE-10 is amended to scope "parameter" to simulation/economic parameters**, carving
  out non-economic numerical-method constants per D-14, with the rationale recorded inline.
  Both amendments are committed with their rationale so the change is auditable rather than a
  quiet loosening of a gate.

### CORE-11 — the Lengnick Table 1 verification

- **D-19: Phase 1 ships the provenance machinery; the paper verification itself moves to a
  blocking gate on Phase 6.** Phase 1 writes `config/PROVENANCE.md` enumerating all 18
  Lengnick-attributed rows, each marked `GRADE: B | UNVERIFIED`, above a
  **domain-knowledge-free verification procedure**: open the published JEBO article, and for
  each row record `agrees` / `differs (paper says X)` / `not in Table 1`. Phase 1 does **not**
  block on it and does **not** schedule another automated fetch.
  Rationale: verification is egress-blocked on all six candidate hosts and this is now the
  **second independent research pass to hit the same wall** — a further agent attempt is waste.
  It is deferred to Phase 6 rather than blocking Phase 1 because **no phase before 6 consumes
  the values** (Phases 1–4 contain no economics; Phase 5 is goods and production), so blocking
  the project's first phase on an out-of-band human action buys nothing and risks a stall.
  Deferred, not dropped: Phase 6 is the first consumer and is already research-flagged as the
  model's widest-sensitivity region.
- **D-20: An agent must never transcribe Table 1 from training memory.** Every
  Lengnick-attributed number currently in the repo is `[ASSUMED]`, inherited from
  `SUMMARY.md`, and none has been read from a primary source. A discrepancy found later must be
  **written down and the config updated with a note**, never silently overwritten — ROADMAP
  criterion 5 requires exactly this.

### Lint enforcement (the gate that makes the rest real)

- **D-21: Lint levels go in `Cargo.toml`'s `[lints.clippy]` table**, not a `#![deny(...)]` crate
  attribute — it applies to every target without a per-file attribute and survives file
  additions.
- **D-22: CI runs `cargo clippy --all-targets --all-features -- -D warnings`.** Plain
  `cargo clippy` **does not lint `tests/`** — verified. A determinism hazard introduced in a
  test would otherwise pass.
- **D-23: The banned-`f64`-method list is *generated* from local std source, not typed by
  hand**, and every entry carries a negative test proving it blocks. Research verified two
  silent holes: clippy **silently ignores `disallowed-methods` paths it cannot resolve**, and a
  `type` alias **silently defeats `disallowed_types`**. So the `HashMap` ban (D-06) needs a grep
  test alongside the clippy rule.
- **D-24: ROADMAP criterion 4 is satisfied by a negative test, not by the lints' existence.**
  `tests/lints.sh` must pass on clean code and **fail** when a hazard is deliberately
  introduced. A lint never observed to block has never been shown to work — the same standard
  Phase 2 applies to its invariants.
- **D-25: `sha2` is pinned to 0.10.x.** 0.11.0 breaks the `{:x}` hex idiom used for the config
  hash.
- **D-26: `--seed` is recorded in `run_meta.json` as the *effective* seed** — the CLI override
  when present, the config value otherwise. Recording the config value while running the
  override makes a run unreproducible from its own metadata.

### Claude's Discretion

The user delegated the entire phase ("Im unfamiliar with the field you will have to decide"),
so beyond D-01…D-26 the planner has full discretion on anything not fixed by CLAUDE.md's
Technology Stack section or by CORE-01…CORE-11 — module decomposition, test naming, error enum
shape, the exact `Rngs` façade signature, and file-level organisation within the structure
CLAUDE.md §9 mandates (single crate, `lib.rs` + thin `main.rs`, no workspace).

**One standing instruction that survives this delegation:** where a decision above cites a
measurement, the planner should treat the measurement as the reason and not substitute a
plausible-sounding alternative rationale. Where the planner disagrees with a decision, it should
say so in the plan rather than quietly implementing something else.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 1 primary sources
- `.planning/phases/01-primitives-and-the-determinism-spine/01-RESEARCH.md` — **the load-bearing
  document for this phase.** Every decision above traces to it. Of particular weight:
  §"Architecture Patterns" Patterns 1–7 (RNG packing, fixed-draw samplers, `Money`, lint wiring,
  generational `FirmId`, config strictness, `pow_frac_det`); §"Common Pitfalls" 1–10 (each is a
  silently-passing gate if ignored); §"Code Examples" 1–7 (verified, compile-tested wiring);
  §"Open Questions" (the eight decisions above, with costs).
- `.planning/ROADMAP.md` §"Phase 1: Primitives and the Determinism Spine" — goal and the five
  success criteria the phase is graded against.
- `.planning/REQUIREMENTS.md:12-22` — CORE-01 … CORE-11 verbatim. **Note D-17/D-18: CORE-03 and
  CORE-10 are to be amended in this phase.**

### Project-level constraints
- `./.claude/CLAUDE.md` — authoritative on stack selection. §1 (seeded RNG), §2 (std determinism
  hazards), §4 (configuration), §5 (integer money), §6 (fixed-point/float boundary), §7
  (testing), §9 (project layout), and the "What NOT to Use" table. Research flags **one factual
  error**: the row claiming `SmallRng` / feature `small_rng` was removed in `rand` 0.10 — see
  D-17.
- `.planning/PROJECT.md` — core value, constraints, and the Key Decisions table.
- `.planning/STATE.md` — records the CORE-11 blocker and the Phase 1 light research flag.

### Parameter provenance (feeds D-16, D-19, D-20)
- `.planning/research/SUMMARY.md:169` — the A/B/C/PROJECT source-grade scheme, **already defined
  — do not reinvent it**.
- `.planning/research/SUMMARY.md:171-209` — the graded 37-row parameter table, of which 18 rows
  are the unverified Lengnick Table 1 values.
- `.planning/research/STACK.md` — stack rationale; §465 independently reaches the D-11 `f64`
  recommendation, though without the MKT-01 argument that actually decides it.
- `.planning/research/PITFALLS.md`, `.planning/research/ARCHITECTURE.md` — project-round research.

### To be created by this phase
- `config/PROVENANCE.md` — per-parameter grade/source/cadence, the 18 `UNVERIFIED` Lengnick
  rows, and the `GRADE: PROJECT` rationale for the D-14 code constants.

</canonical_refs>

<code_context>
## Existing Code Insights

**The repository contains no source code.** No `Cargo.toml`, no `.rs` files, no
`.planning/codebase/` maps. Phase 1 is greenfield — every file it touches, it creates.

### Reusable Assets
- None in code. The reusable assets are **documentary**: `01-RESEARCH.md` §"Code Examples" 1–7
  contains verified, compile-tested skeletons for `Cargo.toml`, `rust-toolchain.toml`,
  `clippy.toml` (with the generated float list), the `Money` overflow shape, criterion 2's RNG
  isolation test, criterion 3's exhaustive missing-key loop, and criterion 4's negative lint
  test. These were executed on this machine against `rustc 1.94.1` and should be adapted, not
  re-derived.

### Established Patterns
- No code patterns exist yet. **This phase sets them**, and Phases 2–11 are written inside them.
  The patterns being established: IDs as `Vec` indices with generational `FirmId`; money as a
  checked newtype; RNG access only through the `Rngs` façade; the float domain confined to
  `src/numeric.rs`; every comparator tie-broken by agent ID.

### Integration Points
- **Phase 2 (ledger)** consumes `Money`, its checked API and `split`. D-07's split API is the
  contract the ledger's `transfer()` is written against.
- **Phase 3 (tick pipeline)** consumes the `Rngs` façade and the `purpose` enum; D-02's
  append-only discriminant rule binds every later phase that adds a draw site.
- **Phase 3 (log seam)** consumes the config hash and D-26's effective-seed recording.
- **Phase 4 (harness)** consumes the `*_cents` integer convention and D-13's full-precision
  `expected_demand` logging — its dtype assertions depend on both.
- **Phase 6 (labour)** is the first consumer of the Lengnick values and carries D-19's
  verification gate.
- **Phase 7 (goods market)** consumes `pow_frac_det` for MKT-01's `(m/P̄)^0.9`.
- **Phase 9 (firm planning)** consumes `expected_demand`'s `f64` representation and the
  `demand_to_units` crossing function.

</code_context>

<specifics>
## Specific Ideas

The user made no specific design requests — they stated they are unfamiliar with the domain and
delegated the decisions. The "specifics" of this phase are therefore the **measurements** that
drove each decision, and they are the thing to preserve:

- Bit-packed `set_stream`: **237 ms** vs **1390 ms** (SHA-256 child seeds) for 3.65 M
  sub-streams. Isolation demonstrated by execution — sub-stream `(10, 7, LabourSample)`
  consumed 7 draws instead of 4, and `(10, 7, GoodsSample)` was bit-identical.
- Default `[profile.release]` **silently wrapped** `i64::MAX - 1 + 6`; the `Money` operator
  panicked in all three profiles.
- `pow_frac_det`: worst relative error **1.9e-12** over 20,000 inputs; **bit-identical across
  100,000 invocations**.
- `f64` vs `i64` milli-units for `expected_demand`: max divergence **2.064e-3 units** over
  3,650 ticks; the `i64` route carries a **0.003-unit dead band**.
- Three enforcement holes found in the inherited plan (clippy skips `tests/` without
  `--all-targets`; unresolvable `disallowed-methods` paths are silently ignored; a `type` alias
  defeats `disallowed_types`) and one factual error (`SmallRng` removal).

</specifics>

<deferred>
## Deferred Ideas

- **Lengnick Table 1 paper verification** — deferred from Phase 1 to a blocking gate on
  **Phase 6**, the first phase that consumes the values (D-19). Not dropped; the machinery and
  the procedure ship in Phase 1.
- **A `Lookup` wrapper for lookup-only `HashMap` use** — deliberately not built (D-06). Revisit
  only if a later phase demonstrably needs a non-`Ord`-keyed map that `BTreeMap` and `Vec`-by-ID
  cannot serve.
- **`ChaCha12Rng` / `ChaCha20Rng`** — same API and portability guarantee at ~1.5× / ~3× cost.
  Named in research assumption A3 as the mitigation if ChaCha8's nonce-separated stream
  independence is ever doubted. Not a v1 concern.
- **Cross-machine reproducibility on i686** — research assumption A4 notes the `f64`
  recommendation would need revisiting on a 32-bit x86 target (x87 excess precision). No target
  triple is named in any planning document; x86-64/aarch64 is assumed. Revisit only if a 32-bit
  target appears.
- **Everything in `REQUIREMENTS.md` §"Out of Scope" and §"v2 Requirements"** — banks, government,
  multiple goods, capital, R&D, demographics, stock market, geography, endogenous firm founding,
  GUI, reusable plotting toolkit, scaling beyond 200 agents.

No scope creep arose during discussion — the user delegated rather than proposing additions.

</deferred>

---

*Phase: 1-primitives-and-the-determinism-spine*
*Context gathered: 2026-08-30*
