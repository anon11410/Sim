# Phase 1: Primitives and the Determinism Spine — Research

**Researched:** 2026-08-30
**Domain:** Deterministic Rust primitives — seeded RNG sub-streams, integer money, strict TOML config, lint-enforced determinism hazards
**Confidence:** HIGH on everything mechanical (every load-bearing claim was compiled and run on this machine against rustc 1.94.1 / rand 0.10.2). LOW on the Lengnick Table 1 verification, which is **egress-blocked in this environment** — see CORE-11.

---

## User Constraints

**CONTEXT.md does not exist for this phase.** No `/gsd-discuss-phase` has been run. Research
was therefore scoped by ROADMAP.md Phase 1, REQUIREMENTS.md CORE-01…CORE-11, and
`./.claude/CLAUDE.md` (treated as authoritative on stack selection per the invocation).

### Decisions a `/gsd-discuss-phase` should settle before planning locks

These are the choices this research surfaced that are **not** settled by any existing document.
Each is flagged again in `## Open Questions` with the recommendation and its cost.

1. **RNG sub-stream keying: bit-packed `set_stream` vs. hashed child seeds.** Research
   recommends bit-packed. Cost of getting it wrong is a rewrite of every draw site.
2. **`expected_demand` representation: `f64` vs `i64` milli-units.** Research recommends `f64`,
   restricted. Both are defensible; the roadmap explicitly flags this as the phase's second
   research question.
3. **How `(m/P̄)^0.9` (MKT-01, Phase 7) is computed given that `f64::powf` is banned.** This is
   a Phase 1 decision because the primitive belongs with the other primitives and the clippy
   ban is written in Phase 1. Research recommends shipping a `pow_frac_det` primitive now.
   *(This tension is not noted anywhere in the existing planning documents.)*
4. **Whether numerical-method constants (the `pow_frac_det` bit count, the milli-unit scale)
   live in the TOML config or as code constants**, given CORE-10's "no parameter hardcoded in
   logic". They are not economic parameters. Research recommends code constants with a
   documented rationale, but CORE-10 as written arguably forbids that.
5. **Whether `Money` overflow panics or returns `Result`** — ROADMAP criterion 1 says "panics",
   CLAUDE.md's table says `thiserror` carries `MoneyOverflow`. Research recommends the split
   (see `## Architecture Patterns → Pattern 3`) but the split is a choice.

### Claude's Discretion

Everything not fixed by CLAUDE.md's Technology Stack section or by CORE-01…CORE-11.

### Deferred Ideas (OUT OF SCOPE)

Everything in REQUIREMENTS.md `## Out of Scope` and `## v2 Requirements`. In particular this
phase builds **no economics** — ROADMAP Ordering Constraint 1: *"Ledger, invariants, tick
pipeline and log schema precede any economic rule (Phases 1–4 contain zero economics)."*

---

## Phase Requirements

| ID | Description (verbatim, REQUIREMENTS.md:12-22) | Research Support |
|----|-------------|------------------|
| CORE-01 | "All monetary values use a `Money` newtype over `i64` minor units (cents) with checked arithmetic that panics on overflow regardless of build profile" | Pattern 3 + Pitfall 4. Verified first-hand: a `checked_add(...).expect(...)` newtype panics in debug, default release, and overflow-checked release. |
| CORE-02 | "`[profile.release]` sets `overflow-checks = true` (Cargo defaults it off)" | Verified first-hand: default release **silently wrapped** `i64::MAX - 1 + 6`; with the flag it panicked. Code Example 4. |
| CORE-03 | "All randomness derives from one master seed via `ChaCha8Rng`; `StdRng` and `SmallRng` are absent from the dependency graph" | **Pitfall 1 — this requirement is not satisfiable as written.** `StdRng` can be feature-gated out; `SmallRng` **cannot**. Verified from crate source and by compiling. Mitigation given. |
| CORE-04 | "RNG draws are namespaced into per-purpose sub-streams keyed on `(master_seed, tick, agent_id, purpose)`, so changing the draw count in one market cannot perturb another" | **Pattern 1** — the headline finding. Bit-packed `ChaCha8Rng::set_stream`. Isolation property proven by execution. |
| CORE-05 | "Sampling uses fixed-draw algorithms (partial Fisher-Yates), never rejection sampling" | Pattern 2 + Pitfall 3. Verified that `rand`'s own `random_range`, `Uniform::sample` and `seq::index::sample` are all *not* fixed-draw. Hand-rolled skeleton supplied and executed. |
| CORE-06 | "Firm identity is generational (`FirmId { slot, gen }`) and accessors return `Option`, so a stale ID after respawn is a typed miss rather than a silent hit on a different firm" | Pattern 5. Pure design; nothing to verify. Note the interaction with the RNG key encoding (agent field must carry `slot`, not `gen`). |
| CORE-07 | "`clippy.toml` bans `HashMap`/`HashSet` on behaviour paths and the 31 non-deterministic `f64` methods, enforced in CI" | Pattern 4 + Pitfalls 2, 5, 6. Full 33-path list enumerated from local std source; wiring verified to exit 101; three enforcement holes found. |
| CORE-08 | "Crate is `lib.rs` plus a thin `main.rs` so integration tests can reach all code" | Recommended Project Structure. No verification needed. |
| CORE-09 | "`Cargo.lock` and `rust-toolchain.toml` are committed; no `rayon` dependency and no `-C target-cpu=native`" | Code Example 6. `rust-toolchain.toml` pin verified to resolve on this machine. |
| CORE-10 | "Every simulation parameter loads from a TOML config with `deny_unknown_fields` and no serde defaults (a serde default is a hidden hardcoded parameter)" | Pattern 6 + Pitfall 7. Exact error strings for unknown key / missing key / removed value captured. **`Option<T>` is a hidden default with no attribute to grep for.** |
| CORE-11 | "Lengnick Table 1 values are verified against the published paper, and every config value is annotated with its source grade (A/B/C/PROJECT)" | `## Source Grades and Lengnick Table 1`. Grading scheme already defined in-repo and quoted verbatim. **Paper verification is BLOCKED — all five candidate hosts are egress-denied.** |

---

## Project Constraints (from CLAUDE.md)

`./.claude/CLAUDE.md` is unusually prescriptive. Extracted actionable directives the planner
must not contradict:

**Mandatory**
- Rust 1.94.1, **edition 2024**, pinned via `rust-toolchain.toml` ("load-bearing for determinism, not hygiene").
- `rand` **0.10.2**, `default-features = false`, features `["std", "chacha"]`; RNG is `ChaCha8Rng`.
- `Money(i64)` cents newtype, **no crate**. `[profile.release] overflow-checks = true`.
- `serde` 1.0.229 + `toml` 1.1.4, `#[serde(deny_unknown_fields)]`.
- `thiserror` in `src/lib.rs`; `anyhow` **only** in `main.rs`.
- `proptest` with committed `.proptest-regressions`.
- Single crate, `lib.rs` + thin `main.rs`. **No workspace.**
- Committed `Cargo.lock`.
- `clap` with exactly three flags: `--config`, `--seed`, `--out`.
- Every comparator tie-broken by agent ID: `sort_unstable_by_key(|&f| (price, f))`.
- IDs as vector indices; `Vec<Household>` / `Vec<Firm>`.

**Forbidden**
- `StdRng`, `SmallRng`, `fastrand`, `rand::rng()`, `SysRng`.
- `HashMap`/`HashSet` **iterated** on a behaviour path.
- `f64` for money; `rust_decimal`; `From<f64>`/`Mul<f64>`/decimal `Display` on `Money`.
- The 31 non-deterministic `f64` methods on the behaviour path.
- `-C target-cpu=native` in `.cargo/config.toml` or `RUSTFLAGS`.
- `rayon` / `std::thread::spawn` / any threading.
- `SystemTime::now()` / `Instant::now()` anywhere except `run_meta.json`.
- Pointer formatting (`{:p}`), `std::env::vars()` read by the sim, env-var config overrides.
- `Rc<RefCell<…>>` for agents.
- `figment`, `config` (layering weakens reproducibility).
- `sort_unstable_by` on a non-total comparator.

**One CLAUDE.md claim is factually wrong and is corrected below:** the row
`| SmallRng, ReseedingRng, feature small_rng | Removed / not portable |`. See Pitfall 1.

**Two CLAUDE.md claims were confirmed exactly:** the 31-method `f64` count (31 in
`std/src/num/f64.rs` + 2 in `core`), and default-release integer overflow wrapping.

---

## Summary

Phase 1 is small in code and large in consequence. Almost every claim it rests on is
mechanically checkable, and this research checked them by compiling and running code on this
machine against the exact pinned toolchain — `rustc 1.94.1 (e408947bf 2026-03-25)` — rather
than by reasoning from documentation. That turned up **three enforcement holes and one factual
error** in the inherited plan, each of which would have shipped as a silently-passing gate.

The headline result is the RNG sub-stream scheme (CORE-04), which the roadmap correctly flags
as the phase's most expensive-to-change decision. `ChaCha8Rng` in `chacha20` 0.10.2 exposes
`set_stream(u64)` — a **64-bit nonce giving 2^64 independent keystreams per seed**, each 1 ZiB
long. Because `(tick, agent, purpose)` fits in 64 bits with room to spare, the key can be
**bit-packed rather than hashed**, which makes it *bijective* — collision-free by construction
rather than by cryptographic argument — and roughly **6× cheaper** than the SHA-256-derived
child-seed alternative (237 ms vs 1390 ms for 3.65 M sub-streams, measured). The isolation
property CORE-04 exists to buy was demonstrated directly: adding three extra draws to one
sub-stream left another sub-stream's output bit-identical.

The second decision — `f64` vs `i64` milli-units for `expected_demand` — turned out to be
entangled with a problem no existing document notices. MKT-01's consumption rule is
`(m / P̄)^0.9`, which needs `f64::powf`, which is **on the list Phase 1 is required to ban**.
Phase 1 therefore cannot write the clippy list without deciding how Phase 7 computes that
exponent. The resolution is cheap and was verified here: `x^α` for `0 < α < 1` can be computed
using only `sqrt` and `*` — both IEEE-754 correctly-rounded and both *absent* from the banned
list — by binary expansion of the exponent. Measured against `powf`, worst relative error is
**1.9 × 10⁻¹²** over 20 000 inputs, and the result is **bit-identical across 100 000
invocations**, which is exactly the property `powf` does not have.

**Primary recommendation:** ship the RNG as a `Rngs` façade that hands out short-lived
`Stream` scopes keyed by a bit-packed `(tick:24 | agent:24 | purpose:16)` `set_stream` nonce,
with hand-rolled fixed-draw samplers; keep `expected_demand` as restricted `f64` and ship a
`pow_frac_det` primitive so the `powf` ban never needs an escape hatch; wire clippy through
`Cargo.toml`'s `[lints.clippy]` table and run it in CI as `cargo clippy --all-targets
--all-features -- -D warnings`, because plain `cargo clippy` does not lint `tests/`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Randomness namespacing | `src/rng.rs` (library core) | — | The only module that may construct a `ChaCha8Rng`. Every other module receives a `Stream`, never a generator. |
| Money representation & arithmetic | `src/money.rs` (library core) | Cargo build profile (`overflow-checks`) | Newtype is the primary guard; the profile is the second belt for raw `i64` that escapes the newtype. |
| Agent identity | `src/ids.rs` (library core) | — | `HouseholdId`, `FirmId{slot,gen}`, `GoodId`, `Account`. Pure data. |
| Parameter ingestion | `src/config.rs` (library core) | `main.rs` (CLI) + TOML file (data) | `main.rs` reads the path and owns `anyhow`; the library owns the schema and `thiserror`. |
| Determinism enforcement | **Build tooling** (`clippy.toml`, `Cargo.toml [lints]`, CI) | Type system (newtype wrappers) | Clippy stops *accidental* use; only a wrapper type with no `iter()` stops deliberate use. Neither alone is enough — see Pitfall 5. |
| Toolchain reproducibility | Repo files (`rust-toolchain.toml`, `Cargo.lock`) | CI | Not code. Verification is `git ls-files`, not a test. |
| Deterministic transcendental math | `src/numeric.rs` (library core) | `clippy.toml` (the ban that makes it necessary) | One module owns every float operation the ban would otherwise force an `#[allow]` for. |
| Source-grade provenance | TOML comments + a checked-in annotation convention | Test asserting every key has one | Documentation mechanism, not a code mechanism — see CORE-11. |

---

## Standard Stack

Every version below was resolved live against crates.io on 2026-08-30 via `cargo add --dry-run`
and matches CLAUDE.md exactly. Nothing in CLAUDE.md's stack table needed correcting on version.

### Core (Phase 1 needs these)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rand` | **0.10.2** `default-features = false`, features `["std","chacha"]` | The one seeded RNG | Feature-gating makes `rand::rng()` and `StdRng` **not compile** — verified below. [VERIFIED: `cargo add --dry-run`; crate source read] |
| `chacha20` | **0.10.2** (transitive, via `rand`'s `chacha` feature) | `ChaCha8Rng` implementation | Supplies `set_stream` / `set_word_pos`, which `rand` re-exports unchanged. [VERIFIED: `~/.cargo/registry/src/*/chacha20-0.10.2/src/rng.rs:193-325`] |
| `serde` | **1.0.229** (feature `derive`) | Config deserialisation | Declaration-order fields; `deny_unknown_fields`. |
| `toml` | **1.1.4+spec-1.1.0** | Config format | **Verified it refuses float→int coercion** — the exact failure mode CLAUDE.md rejects `config` for. |
| `sha2` | **0.10.9** (pin the 0.10 line) | Config-bytes hash | See Pitfall 8 — **0.11.0 is the current release and it breaks the `{:x}` idiom.** |
| `thiserror` | **2.0.20** | `ConfigError`, `MoneyOverflow` in the lib | CLAUDE.md mandate. |
| `anyhow` | **1.0.104** | Error plumbing in `main.rs` only | CLAUDE.md mandate. |
| `clap` | **4.6.6** (feature `derive`) | `--config`, `--seed`, `--out` | CLAUDE.md mandate. |

### Supporting (dev-dependencies for Phase 1)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `proptest` | **1.11.0** | `Money::split` sum property, `a - b + b == a` | Criterion 1 explicitly requires property tests over non-evenly-dividing amounts. Commit `.proptest-regressions`. |

### Not needed until later phases

`csv` 1.4.0, `serde_json` 1.0.151, `insta` 1.48.0, `assert_cmd` 2.2.2 (Phase 3);
`indexmap` 2.14.1 (only if a non-`Ord` map key ever appears — prefer `Vec`-by-ID).

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| bit-packed `set_stream` | SHA-256(`master ‖ tick ‖ agent ‖ purpose`) → `from_seed` | Measured **5.9× slower** (1390 ms vs 237 ms per 3.65 M sub-streams). Collision-free only probabilistically, vs. bijectively. Its one advantage — an unbounded key space — is unneeded: 64 bits is ~10⁵ × more than the model can use. |
| bit-packed `set_stream` | `set_word_pos(u128)` within one stream | The 68-bit word position is a *position*, not a namespace: two keys mapping to nearby positions overlap keystream. `set_stream` is the API designed for namespacing. Also note `set_stream` **erases** `word_pos`, so they cannot be combined without ordering care. |
| `ChaCha8Rng` | `ChaCha12Rng` / `ChaCha20Rng` | Same portability guarantee, same API, ~1.5×/3× slower. Choose if the reduced round count ever feels uncomfortable for nonce-separated stream independence. At 200 agents the cost is invisible. |
| `sha2` 0.10.9 | `sha2` 0.11.0 | 0.11 returns `hybrid_array::Array`, which does **not** implement `LowerHex` — verified by compile failure. Pin 0.10 or hand-roll hex. |
| restricted `f64` for `expected_demand` | `i64` milli-units | See `## The `f64` vs `i64` Decision`. Measured divergence over 3650 ticks: 2.1 × 10⁻³ units. Both work. |

**Installation:**
```bash
cargo add rand@0.10.2 --no-default-features --features std,chacha
cargo add serde@1.0.229 --features derive
cargo add toml@1.1.4
cargo add sha2@0.10.9
cargo add thiserror@2.0.20
cargo add anyhow@1.0.104
cargo add clap@4.6.6 --features derive
cargo add --dev proptest@1.11.0
```

**Resulting dependency tree** (verified with `cargo tree` for the rand portion —
note `getrandom` is **absent**, which is the point of the feature gating):

```
├── rand v0.10.2
│   ├── chacha20 v0.10.2
│   │   ├── cfg-if v1.0.4
│   │   ├── cpufeatures v0.3.1
│   │   └── rand_core v0.10.1
│   └── rand_core v0.10.1
```

---

## Package Legitimacy Audit

Run 2026-08-30 via `gsd-tools query package-legitimacy check --ecosystem crates`.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `rand` | crates.io | since 2015-02-03 | 31.4 M/wk | github.com/rust-random/rand | **OK** | Approved |
| `serde` | crates.io | since 2014-12-05 | 22.2 M/wk | github.com/serde-rs/serde | **OK** | Approved |
| `toml` | crates.io | since 2014-11-11 | 15.7 M/wk | github.com/toml-rs/toml | **OK** | Approved |
| `sha2` | crates.io | since 2016-05-06 | 18.2 M/wk | github.com/RustCrypto/hashes | **OK** | Approved |
| `thiserror` | crates.io | since 2019-10-09 | 26.5 M/wk | github.com/dtolnay/thiserror | **OK** | Approved |
| `anyhow` | crates.io | since 2019-10-05 | 15.7 M/wk | github.com/dtolnay/anyhow | **OK** | Approved |
| `clap` | crates.io | since 2015-03-01 | 17.3 M/wk | github.com/clap-rs/clap | **OK** | Approved |
| `proptest` | crates.io | since 2017-06-18 | 3.5 M/wk | github.com/proptest-rs/proptest | **OK** | Approved |
| `chacha20` | crates.io | since 2016-10-06 | 7.4 M/wk | github.com/RustCrypto/stream-ciphers | **OK** | Approved |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

Every package above was **also** independently confirmed to exist and resolve on the correct
ecosystem registry by `cargo add --dry-run` on this machine, and all except `proptest` are
already named in `./.claude/CLAUDE.md`'s authoritative stack table. Cargo has no `postinstall`
equivalent; `build.rs` scripts present in the graph are `typenum`/`version_check`
(`sha2` 0.10 transitive) and `cpufeatures`, all first-party RustCrypto/dtolnay infrastructure.
No `checkpoint:human-verify` gate is required for any install in this phase.

---

## Architecture Patterns

### System Architecture Diagram

```
      config.toml (data)          --seed <u64>          --out <dir>
            │                          │                     │
            │  raw bytes               │                     │
            ├──────────────┐           │                     │
            ▼              ▼           ▼                     ▼
   ┌─────────────────┐  ┌──────────────────┐        ┌──────────────────┐
   │ config::load()  │  │ sha2::Sha256     │        │ (Phase 3 sink)   │
   │ toml + serde    │  │ over file BYTES  │        │  run directory   │
   │ deny_unknown    │  └────────┬─────────┘        └──────────────────┘
   │ no defaults     │           │ config_hash
   └────────┬────────┘           │
            │ Params             │              ┌───────────────────────┐
            │  (all values       └─────────────▶│ run_meta.json         │
            │   integer or       ┌─────────────▶│  seed, hash, toolchain│
            │   restricted f64)  │              │  EXCLUDED from diff   │
            ▼                    │              └───────────────────────┘
   ┌──────────────────────────────────────────────────────────────────┐
   │                    Ctx { params, rngs, tick, … }                 │
   └───────┬───────────────────────────────┬──────────────────────────┘
           │                               │
           ▼                               ▼
   ┌────────────────────┐         ┌─────────────────────────────────┐
   │ money::Money(i64)  │         │ rng::Rngs  (owns master seed)   │
   │  checked +/-       │         │   .stream(tick, agent, purpose) │
   │  split(n) exact    │         └───────────────┬─────────────────┘
   │  NO f64 crossings  │                         │ set_stream(bitpacked key)
   └─────────┬──────────┘                         ▼
             │                        ┌───────────────────────────────┐
             │                        │ rng::Stream  (short-lived)    │
             │                        │  below(n)      exactly 1 draw │
             │                        │  coin_ppm(p)   exactly 1 draw │
             │                        │  sample_k(k)   exactly k draws│
             │                        │  draws() -> u32  (log this)   │
             │                        └───────────────┬───────────────┘
             │                                        │
             ▼                                        ▼
   ╔══════════════════════════════════════════════════════════════════╗
   ║   Phase 2+ consumers: books, world, phases/*  (NO ECONOMICS HERE)║
   ╚══════════════════════════════════════════════════════════════════╝
             ▲                                        ▲
             │  enforced by                           │  enforced by
   ┌─────────┴────────────────────────────────────────┴───────────────┐
   │ clippy.toml + [lints.clippy] + CI `--all-targets -- -D warnings`  │
   │  disallowed-types:   HashMap, HashSet, SmallRng, Xoshiro*         │
   │  disallowed-methods: 33 f64 paths (+33 f32), SystemTime::now      │
   └───────────────────────────────────────────────────────────────────┘
   ┌───────────────────────────────────────────────────────────────────┐
   │ numeric::pow_frac_det(x, α)  — sqrt+mul only, no banned method    │
   │   the reason the ban never needs an #[allow] escape hatch          │
   └───────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
Sim/
├── rust-toolchain.toml     # channel = "1.94.1"  (VERIFIED: resolves on this machine)
├── Cargo.toml              # [profile.release] overflow-checks; [lints.clippy]
├── Cargo.lock              # COMMITTED — part of the reproducibility contract
├── clippy.toml             # disallowed-types / disallowed-methods
├── .cargo/                 # ABSENT by design — no config.toml, no target-cpu=native
├── config/
│   └── baseline.toml       # every parameter + source-grade annotation comments
├── src/
│   ├── lib.rs              # pub mod money; ids; config; rng; numeric;  (thiserror)
│   ├── main.rs             # clap, 3 flags, anyhow — ~40 lines, no logic
│   ├── money.rs            # Money(i64), checked ops, split(n)
│   ├── ids.rs              # HouseholdId, FirmId{slot,gen}, GoodId, Account
│   ├── config.rs           # Params, deny_unknown_fields, config_hash()
│   ├── rng.rs              # Rngs, Stream, Purpose, fixed-draw samplers
│   └── numeric.rs          # the ONLY module allowed to touch f64
└── tests/
    ├── determinism_rng.rs  # criterion 2
    ├── config_strict.rs    # criterion 3
    └── money_props.rs      # criterion 1 (proptest)
```

---

### Pattern 1 — RNG sub-streams: bit-packed `set_stream` (CORE-04) ★ the phase's key decision

**What:** One master 32-byte seed. Every draw site opens a short-lived `Stream` addressed by a
`u64` nonce that bit-packs `(tick, agent, purpose)`. `ChaCha8Rng::set_stream` selects the
keystream; `word_pos` resets to 0, so each `(tick, agent, purpose)` names one deterministic
sequence independent of every other.

**Verified API facts** [VERIFIED: `~/.cargo/registry/src/index.crates.io-*/chacha20-0.10.2/src/rng.rs:102-325`, read this session]:

- `rand::rngs::ChaCha8Rng` is a re-export of `chacha20::ChaCha8Rng`, gated on `feature = "chacha"`:
  `rand-0.10.2/src/rngs/mod.rs:115-116` reads verbatim
  `#[cfg(feature = "chacha")]` / `pub use chacha20::{ChaCha8Rng, ChaCha12Rng, ChaCha20Rng};`
- `pub fn set_stream(&mut self, stream: u64)` — exists, `rng.rs:263`.
- `pub fn get_stream(&self) -> u64` — `rng.rs:272`.
- `pub fn set_word_pos(&mut self, word_offset: u128)` — `rng.rs:224`.
- `pub fn set_block_pos(&mut self, block_pos: u64)` / `get_block_pos` — `rng.rs:242`, `rng.rs:251`.
- Crate doc, verbatim: *"This RNG implementation uses a 64-bit counter and 64-bit stream
  identifier (a.k.a nonce). A 64-bit counter over 64-byte (16 word) blocks allows 1 ZiB of
  output before cycling, and the stream identifier allows 2^64 unique streams of output per
  seed."*
- Doc warning, verbatim: *"**This value will be erased when calling `set_stream()`, so call
  `set_stream()` before calling `set_word_pos()`**"*. `set_stream` internally calls
  `set_block_pos(0)`.

**Why bit-packing, not hashing.** The address space needed is tiny against 2^64:
`tick < 3650` (12 bits), `agent < 220` (8 bits), `purpose < ~32` (5 bits) — 25 bits used out of
64. Allocating `tick:24 | agent:24 | purpose:16` leaves headroom for 16.7 M ticks and 16.7 M
agents and is **bijective**, so *distinct tuples produce distinct nonces by arithmetic*, not by
a collision-resistance argument. A SHA-256 child seed gives the same isolation, costs 5.9×
more, and is only probabilistically collision-free. CLAUDE.md's rule *"never persist a hash of
a Rust value; persist a hash of bytes"* is respected trivially: there is no hash at all.
`sha2` remains for the config-bytes hash, where it is the right tool.

**Measured, this session** (release build, this machine):

| Scheme | 3 650 000 sub-streams | Notes |
|---|---|---|
| `from_seed` + bit-packed `set_stream` | **237 ms** | Recommended |
| `from_seed` with a memcpy-derived seed | 268 ms | ChaCha has no key schedule, so `from_seed` is nearly free too |
| SHA-256(master‖tick‖agent‖purpose) → `from_seed` | **1390 ms** | 5.9× the cost, no benefit |

At the realistic shape — `3650 ticks × 220 agents × 4 purposes = 3 212 000` sub-streams — the
recommended scheme cost **237 ms**, comfortably inside the "a 200-agent decade completes in
seconds" constraint.

**Isolation property, demonstrated by execution** (this is criterion 2's substance):

```
3.  goods stream unperturbed by extra labour draws: true
3b. labour prefix still stable:                     true
4.  9600 substreams, first-u64 collisions:          0
5.  set_stream resets word_pos:                     true (before=0 after=0)
1.  same-seed identical:                            true
2.  different-seed differs:                         true
```

Test 3 is the one that matters: sub-stream `(10, 7, LabourSample)` consumed **7** draws in the
second run instead of **4**, and sub-stream `(10, 7, GoodsSample)` produced bit-identical
output. That is CORE-04's guarantee, observed rather than asserted.

**⚠ The one hazard this design has, verified:** re-entering the *same* key replays the *same*
values.

```
6. HAZARD re-entering same key replays: true
```

So a key must be opened **at most once per run**. If a site genuinely needs two independent
draw sequences for the same `(tick, agent)`, that is two `Purpose` variants, not two visits.
The planner should make this a documented invariant and, ideally, a debug-mode assertion (a
`BTreeSet<u64>` of issued keys under `#[cfg(debug_assertions)]`).

**`Purpose` must have hand-assigned discriminants.** `#[repr(u16)]` with explicit `= 1`, `= 2`,
… values. Relying on declaration order means inserting a variant renumbers every later one and
silently re-keys every sub-stream after it — the exact class of change this pattern exists to
make safe. Reserve gaps (10, 20, 30…) per subsystem, and never reuse a retired number.

**`FirmId{slot,gen}` interaction (CORE-06 × CORE-04).** The `agent` field of the key must carry
`slot`, **not** the generation. Two firms occupying the same slot in different generations are
different agents but never coexist at the same tick, so `(tick, slot, purpose)` is still unique.
Including `gen` would work too but wastes bits; excluding `slot` would collide. State this
explicitly in the plan — it is the sort of thing that gets decided wrong once and found in
Phase 10.

**Example** — compiled and run this session, output shown after:

```rust
use rand::rngs::ChaCha8Rng;
use rand::{Rng, SeedableRng};

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Purpose {
    ActivationOrderHouseholds = 1,
    ActivationOrderFirms      = 2,
    LabourSample              = 3,
    EmployedSearchCoin        = 4,
    GoodsSample               = 5,
    PriceInactionCoin         = 6,
    PriceStep                 = 7,
    WageStep                  = 8,
    SupplierRevision          = 9,
    PlanningOffsetInit        = 10,
    BankruptcyOwnerDraw       = 11,
}

pub struct Rngs { master: [u8; 32] }
pub struct Stream(ChaCha8Rng, u32); // generator + draw counter

impl Rngs {
    pub fn new(master_seed: u64) -> Self {
        let mut s = [0u8; 32];
        s[..8].copy_from_slice(&master_seed.to_le_bytes());
        Self { master: s }
    }
    /// key = tick(24) | agent(24) | purpose(16).  Bijective => collision-free.
    pub fn stream(&self, tick: u32, agent: u32, p: Purpose) -> Stream {
        assert!(tick < (1 << 24) && agent < (1 << 24));
        let key = ((tick as u64) << 40) | ((agent as u64) << 16) | (p as u16 as u64);
        let mut r = ChaCha8Rng::from_seed(self.master);
        r.set_stream(key);          // resets word_pos to 0
        Stream(r, 0)
    }
}
```

### Pattern 2 — Fixed-draw samplers, hand-rolled (CORE-05)

**What:** never call `rand`'s range or index samplers on the behaviour path. Provide
`below(n)` (exactly one 64-bit draw, multiply-high), `coin_ppm(p)` (exactly one draw), and
`sample_k` (partial Fisher-Yates, exactly `k` draws).

**Why — verified from `rand` 0.10.2 source, not assumed:**

| `rand` API | Algorithm | Draw count | Source |
|---|---|---|---|
| `rng.random_range(a..b)` | Canon's method, **biased** variant (default feature set) | **1 or 2**, conditional on the drawn value | `distr/uniform_int.rs:177-213` — the comment reads verbatim *"Sample single value, Canon's method, biased"* and *"if the sample is biased... generate a new sample to reduce bias"* |
| `Uniform::new(a,b).sample(rng)` | Lemire, **unbiased** | **unbounded loop** | `distr/uniform_int.rs:141-156` — `let hi = loop { … if lo >= thresh { break hi; } };` |
| `random_range` with `features=["unbiased"]` | Canon's, unbiased | **unbounded loop** | `distr/uniform_int.rs:218-250` — *"In contrast to the biased sampler, we use a loop"* |
| `seq::index::sample(rng, len, k)` | dispatches to Floyd / in-place FY / **rejection**, chosen by **`f32` arithmetic on `len` and `k`** | varies by algorithm | `seq/index.rs:240-282` — the dispatch comment cites a *performance* PR, i.e. the thresholds are tuning, not contract |

Empirically both `random_range(0..20u32)` and `Uniform::sample` consumed exactly one 32-bit
word in 200 000 trials each (the second-draw branch fires with probability ≈ 20/2³², i.e. never
in practice) — but "never in practice" is not "never", and `seq::index::sample`'s algorithm
choice is explicitly a performance heuristic that `rand` may retune in a patch release. Since
`Cargo.lock` is committed this is not a *reproducibility* break; it is a **stability-across-
upgrade** break, which is what CORE-05 protects against.

**Note the relationship to CORE-04.** Sub-streams make a variable draw count *harmless across*
sites — a rejection loop in the goods market can no longer shift the labour market. What
CORE-05 still buys on top is (a) a per-tick draw-count series that is a *meaningful* divergence
localiser, and (b) no unbounded loop on the behaviour path at all. Both are worth having; the
requirement is not redundant, but its justification is narrower than the raw text implies.

**Verified skeleton** (executed; `sample_k(20, 5)` draw counts observed were exactly `{5}`;
`below(20)` over 200 000 draws gave min 9 850 / max 10 205 against an expectation of 10 000):

```rust
impl Stream {
    /// EXACTLY one 64-bit draw. Multiply-high. Bias <= n / 2^64. No loop.
    pub fn below(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        self.1 += 1;
        ((self.0.next_u64() as u128 * n as u128) >> 64) as u64
    }
    /// Coin at probability p_ppm (parts per million). Exactly one draw.
    pub fn coin_ppm(&mut self, p_ppm: u32) -> bool {
        self.below(1_000_000) < p_ppm as u64
    }
    /// Partial Fisher-Yates: EXACTLY `k` draws, always.
    pub fn sample_k(&mut self, pool: &mut [u32], k: usize) -> Vec<u32> {
        let n = pool.len();
        assert!(k <= n);
        let mut out = Vec::with_capacity(k);
        for i in 0..k {
            let j = i + self.below((n - i) as u64) as usize;
            pool.swap(i, j);
            out.push(pool[i]);
        }
        out
    }
    /// Log this per tick — it is the divergence localiser.
    pub fn draws(&self) -> u32 { self.1 }
}
```

Probabilities enter as **parts-per-million integers** (`theta_ppm = 750_000`), never as `f64`.
This keeps θ, π, ψ_price, ψ_quant entirely in the integer domain and removes a whole class of
float-threshold questions.

### Pattern 3 — `Money`: panic on the operator, `Result` on the named API (CORE-01, CORE-02)

**The apparent conflict, resolved.** ROADMAP criterion 1 says *"`Money` arithmetic panics on
overflow in both debug and release profiles"*. CLAUDE.md's supporting-libraries table says
`thiserror` carries a `MoneyOverflow` variant. Both are satisfiable simultaneously:

- **Operator impls** (`Add`, `Sub`, `AddAssign`, `Neg`, `Sum`) route through `checked_*` and
  `.expect("Money overflow …")`. These **panic in every profile** — verified. Overflow here is
  a program bug (money in this model is a fixed pile that cannot approach `i64::MAX`), and a
  panic is the correct response.
- **Named checked API** (`checked_add`, `checked_sub`, `try_scale`) returns
  `Result<Money, MoneyOverflow>` for the one place overflow is *expected* to be handled
  gracefully: **config ingestion**, where a user-supplied `total_money_cents` could legitimately
  be absurd and should produce a named `ConfigError`, not a panic.

This is the split. State it in the plan so the executor does not implement one and delete the
other.

**Verified overflow behaviour, this session** (three separate `cargo run` invocations):

| Build | raw `i64::MAX - 1 + 6` | `Money` operator `+` | `Money::checked_add` |
|---|---|---|---|
| `cargo run` (debug) | **panics** | **panics** | `None` |
| `cargo run --release`, **default profile** | **silently wraps** | **panics** | `None` |
| `cargo run --release` + `overflow-checks = true` | **panics** | **panics** | `None` |

The middle row is CORE-02's entire justification, observed directly. The `Money` column is
CORE-01's "regardless of build profile", observed directly — and note it holds **even without**
CORE-02, which is why both belts are wanted rather than either.

**`Money::split(n)` and criterion 1's property test.** The remainder must be distributed
deterministically by ascending recipient index: the first `r = amount % n` recipients each get
one extra cent. The property is `split(a, n).iter().sum() == a` **for all** `(a, n)` with
`n > 0`, and criterion 1 is explicit that the proptest strategy must generate amounts that do
*not* divide evenly — otherwise a `vec![a / n; n]` implementation that destroys `r` cents
passes. A concrete generator: `(1i64..1_000_000, 2u32..64)` with a `prop_assume!` on
`a % n != 0` for a dedicated non-even case, plus the unrestricted case for coverage.

`Money` must **not** implement `Sum` via `fold(0, +)` on the raw `i64` — route it through the
checked `Add`.

### Pattern 4 — Making the determinism lints actually block (CORE-07)

**Wiring, verified to exit 101.** Put the lint *levels* in `Cargo.toml` (this is preferred over
a `#![deny(...)]` crate attribute because it applies to every target without a per-file
attribute, and it survives file additions):

```toml
# Cargo.toml
[lints.clippy]
disallowed_types   = "deny"
disallowed_methods = "deny"
```

and the lint *contents* in `clippy.toml` at the crate root:

```toml
# clippy.toml
disallowed-types = [
  { path = "std::collections::HashMap",   reason = "iteration order is nondeterministic; use Vec-by-id or BTreeMap" },
  { path = "std::collections::HashSet",   reason = "iteration order is nondeterministic; use BTreeSet" },
  { path = "rand::rngs::SmallRng",        reason = "non-portable: rand may replace the algorithm (see CORE-03)" },
  { path = "rand::rngs::Xoshiro256PlusPlus", reason = "not the project RNG" },
  { path = "rand::rngs::Xoshiro128PlusPlus", reason = "not the project RNG" },
]
disallowed-methods = [
  { path = "std::time::SystemTime::now", reason = "wall clock is not an input to the sim" },
  { path = "std::time::Instant::now",    reason = "wall clock is not an input to the sim" },
  # ... the 33 f64 paths and 33 f32 paths, below
]
```

**Verified behaviours** (each observed on this machine, rustc/clippy 1.94.1):

- `cargo clippy` with the above **exits 101** and prints `error: use of a disallowed type …` /
  `error: use of a disallowed method …` with the `reason` string attached as a `note:`. No
  `-D` flag on the command line is required — the `[lints]` table is sufficient.
- The type lint fires on the **`use` statement itself**, not only at the usage site.
- `#[allow(clippy::disallowed_types)]` on an item **does** silence it at that site.
- `f64::abs_sub` resolves and fires (it is `#[deprecated]`, not removed).
- The four unstable methods (`gamma`, `ln_gamma`, `erf`, `erfc`) **do not resolve**, and clippy
  **silently ignores unresolvable paths** — no warning, no error. They are uncallable on stable
  anyway, so this is not a hole, but see Pitfall 6 for why it matters procedurally.

**The complete banned-method list, enumerated from local rustc 1.94.1 std source, not memory.**
[VERIFIED: `$(rustc --print sysroot)/lib/rustlib/src/rust/library/std/src/num/f64.rs` and
`.../core/src/num/f64.rs`, parsed this session]. The verbatim doc-comment marker is:

> **# Unspecified precision**
>
> The precision of this function is non-deterministic. This means it varies by platform, Rust
> version, and can even differ within the same execution from one invocation to the next.

`std/src/num/f64.rs` carries it on **31** methods — this confirms CLAUDE.md's "31" exactly:

```
abs_sub  acos  acosh  asin  asinh  atan  atan2  atanh  cbrt  cos  cosh
erf  erfc  exp  exp2  exp_m1  gamma  hypot  ln  ln_1p  ln_gamma  log
log10  log2  powf  powi  sin  sin_cos  sinh  tan  tanh
```

`core/src/num/f64.rs` carries it on **2** more:

```
to_degrees  to_radians
```

**Total 33 paths per float type.** The identical 33 exist on `f32`, `f16` and `f128` (the
disclaimer appears in `std/src/num/f32.rs`, `f16.rs`, `f128.rs` too) — **and the `f64::` entries
do not cover `f32`**, verified by compiling `x.powf(2.0)` on an `f32` with only `f64::powf`
banned: it passed. Ban both `f64::*` and `f32::*`; `f16`/`f128` are unstable and cannot appear.

**Safe and correctly rounded — deliberately NOT banned:** `+ - * / %`, `sqrt`, `mul_add`,
`abs`, `copysign`, `floor`, `ceil`, `round`, `trunc`, `rem_euclid`, comparisons. `sqrt` being
safe is what makes Pattern 7 possible.

**CI invocation.** See Pitfall 2 — plain `cargo clippy` is not enough.

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Pattern 5 — Generational `FirmId` (CORE-06)

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirmSlot(pub u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirmId { pub slot: FirmSlot, pub gen: u32 }
```

Accessors return `Option<&Firm>` / `Option<&mut Firm>`, comparing the stored `gen` against the
ID's `gen`. Respawn is **in place** at `gen + 1` (BANK-03: `Vec::swap_remove` is never used on
an agent collection). `Ord` is derived, so `(slot, gen)` gives a total order for the
tie-breaking rule that LABR-09 requires. The log identity is `(slot, gen)`.

Nothing here needed verification — it is a well-known generational-arena pattern. The one
non-obvious consequence is its interaction with Pattern 1's key encoding, stated there.

### Pattern 6 — Config strictness (CORE-10)

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Params { pub sim: Sim, pub firm: Firm, /* … */ }
```

`deny_unknown_fields` on **every** struct, not just the root — verified that a stray table
`[oops]` is caught only because the *root* has it, and a stray key inside `[sim]` is caught only
because `Sim` has it.

**Exact error output, captured this session** with `toml` 1.1.4 + `serde` 1.0.229. These are the
strings a verification command can grep for:

| Corruption | Error |
|---|---|
| Unknown key `houseolds = 1` inside `[sim]` | `TOML parse error at line 6, column 1` … `unknown field \`houseolds\`, expected one of \`ticks\`, \`seed\`, \`households\`` |
| **Missing** key `households` | `TOML parse error at line 2, column 1` … ``missing field `households` `` — **the field is named** |
| Unknown table `[oops]` | `unknown field \`oops\`, expected \`sim\` or \`firm\`` |
| **Removed value** `theta_milli =` | `TOML parse error at line 9, column 14` … `string values must be quoted, expected literal string` (a *parse* error, before serde runs) |
| Float where int: `lambda_milli = 250.0` | ``invalid type: floating point `250.0`, expected i64`` — **confirms `toml` does not coerce**, the exact reason CLAUDE.md rejects the `config` crate |
| String where int: `seed = "42"` | `invalid type: string "42", expected u64` |

Every message is prefixed `TOML parse error at line N, column M` and carries a caret span. All
three of criterion 3's cases (unknown key / missing key / removed value) produce a **named**
error. Good grep anchors: `unknown field`, `missing field`, `invalid type`.

**Config hash** — over the **raw file bytes**, per CLAUDE.md's "persist a hash of bytes":

```rust
use sha2::{Digest, Sha256};
let bytes = std::fs::read(path)?;
let digest = Sha256::digest(&bytes);
let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
```

Verified stable across repeated computation. Hashing the bytes (not the parsed `Params`) means
a comment change also changes the hash — which is *correct*, because the comments carry the
source grades that CORE-11 makes load-bearing.

### Pattern 7 — `pow_frac_det`: the primitive that keeps the `powf` ban honest ★

**The problem no existing document notices.** MKT-01 (Phase 7) is
`budget = (m / P̄)^0.9`. REQUIREMENTS.md:81 states it verbatim: *"The household spending budget
is `(m / P_bar)^0.9`"*. Computing it needs `f64::powf` — **which CORE-07 requires Phase 1 to
ban**, and which std documents as varying *"within the same execution from one invocation to the
next"*. An `#[allow(clippy::disallowed_methods)]` escape hatch would therefore not merely
weaken a lint; it would break byte-identical reproducibility, the project's Core Value.

**The resolution.** For `0 < α < 1`, expand α in binary: `x^α = ∏_{k : bit k of α set} x^(2^-k)`,
and `x^(2^-k)` is `k` repeated square roots. `sqrt` is **IEEE-754 mandated correctly rounded**
and is **absent from the 33-method disclaimer list** (verified above); `*` is correctly rounded.
So the whole computation uses only operations with a single, uniquely determined result.

**Measured, this session:**

```
pow_frac_det(x,0.9,40) vs powf: worst relative error = 1.929e-12   (over x = 0.01 .. 199.99)
pow_frac_det bit-identical across 100k invocations: true
  bits=20 -> 605.846037334285
  bits=30 -> 605.847680091327
  bits=40 -> 605.847682499673
  bits=52 -> 605.847682501241
```

```rust
/// Deterministic x^alpha for x > 0, 0 < alpha < 1, using ONLY IEEE-754
/// correctly-rounded operations. No method from the banned list.
pub fn pow_frac_det(x: f64, alpha: f64, bits: u32) -> f64 {
    debug_assert!(x > 0.0 && (0.0..1.0).contains(&alpha));
    let mut acc  = 1.0f64;
    let mut root = x;
    let mut a    = alpha;
    for _ in 0..bits {
        root = root.sqrt();              // correctly rounded per IEEE-754
        a *= 2.0;
        if a >= 1.0 { acc *= root; a -= 1.0; }
    }
    acc
}
```

`bits = 40` is the recommended constant (relative error ~2e-12, well below any economically
meaningful resolution, and 12 iterations cheaper than full precision). **`bits` must be a fixed
committed constant** — changing it changes every trajectory, exactly like changing a parameter.
Whether it belongs in the TOML (CORE-10 literal reading) or as a code constant with a
documented rationale is Open Question 4.

Phase 1 need not use this function. Phase 1 needs to **decide it exists**, because otherwise the
clippy list written in Phase 1 becomes a blocker in Phase 7 and gets weakened under deadline.

---

### Anti-Patterns to Avoid

- **A `type` alias to escape `disallowed_types`.** Verified: `pub type LookupMap<K,V> =
  std::collections::HashMap<K,V>;` behind an `#[allow]` makes every downstream use site
  **completely invisible** to the lint. This is not a legitimate escape hatch — it is a hole.
  See Pitfall 5 for the correct construct.
- **`#[serde(default)]` anywhere.** Criterion 3 greps for it. But see Pitfall 7 — the grep is
  necessary and *not sufficient*.
- **A single global `ChaCha8Rng` threaded through the tick.** This is what CORE-04 exists to
  prevent, and PITFALLS.md:564 records the cost: *"Every bug fix that changes draw count changes
  the whole trajectory; you can never attribute an economic change to a code change."*
- **Deriving `Purpose` discriminants from declaration order.** Inserting a variant renumbers
  everything after it and silently re-keys every sub-stream downstream.
- **Reusing a `(tick, agent, purpose)` key twice in one run.** Verified to replay identical
  values. Add a `Purpose` variant instead.
- **Calling `rand::seq::index::sample` or `SliceRandom::shuffle` on the behaviour path.** Both
  are `rand`-internal algorithm choices that a patch release may retune.
- **Combining `set_stream` and `set_word_pos` in the wrong order.** `set_stream` erases
  `word_pos`; the crate docs say so in bold.
- **`sort_unstable_by(|a, b| a.price.partial_cmp(&b.price).unwrap())`.** Panics on NaN and is
  not a total order. Never sort by a float; sort by `(price_cents, id)`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Seeded, portable PRNG | Any LCG/xorshift/PCG of your own | `rand::rngs::ChaCha8Rng` | ChaCha is a *specified* stream cipher, so the algorithm cannot silently change; `rand` documents the reproducibility guarantee only for the named portable generators. |
| Sub-stream namespacing | A hand-rolled counter mixed into the seed | `ChaCha8Rng::set_stream(u64)` | It is the nonce the cipher was designed around; 2^64 streams × 1 ZiB each, and the crate documents the independence property. |
| Cryptographic hash for the config | Anything | `sha2::Sha256` over file bytes | Already a dependency; the only correct answer for "hash of bytes". |
| TOML parsing / strictness | A hand-rolled key checker | `serde` + `#[serde(deny_unknown_fields)]` | Produces a *named*, spanned error for unknown, missing and mistyped fields — verified above. |
| Overflow detection on `i64` | Manual range checks | `i64::checked_add` + `[profile.release] overflow-checks` | Both verified; the belt-and-braces combination is cheaper than either alone would be to get right. |
| Property-test shrinking | Manual counterexample minimisation | `proptest` | Integrated shrinking keeps shrunk cases *valid*, and `.proptest-regressions` makes a rare failure permanent. |
| CLI parsing | Hand-rolled `std::env::args` | `clap` derive | Three flags, but the help/error handling is free. |

### DO Hand-Roll (the inverse list — unusually important in this project)

| Problem | Hand-roll it | Why the library is wrong here |
|---|---|---|
| Uniform integer in `[0, n)` | `below(n)` via multiply-high, exactly 1 draw | `random_range` is Canon-biased with a conditional second draw; `Uniform::sample` is an unbounded Lemire loop. Both verified from source. |
| Sample `k` distinct from `n` | partial Fisher-Yates, exactly `k` draws | `seq::index::sample` dispatches between three algorithms using `f32` heuristics tuned for *performance*, not stability. |
| Shuffle for activation order | Fisher-Yates over your own `Stream` | `SliceRandom::shuffle` is the same story; also it takes a `&mut R` and would bypass the draw counter. |
| `x^α` for fractional α | `pow_frac_det` (Pattern 7) | `powf` is on the banned list and std documents it as non-deterministic *within one execution*. |
| Money type | `Money(i64)` newtype | Every candidate crate reopens the float boundary CLAUDE.md deliberately closed. |
| Remainder-preserving division | `Money::split(n)` | No crate does the "first `r` recipients get one extra cent, by ascending ID" rule you need for LEDG-03. |

**Key insight:** in this project the usual library-vs-hand-roll calculus is inverted for
anything that consumes randomness. A library optimises for *statistical quality per nanosecond*
and reserves the right to retune; this project needs *a fixed, auditable number of draws with a
stable algorithm*, and will trade quality for it without hesitation at 200 agents. Everywhere
else — hashing, parsing, error types, CLI — take the library.

---

## Common Pitfalls

### Pitfall 1: CORE-03 is not satisfiable as written — `SmallRng` cannot be removed from the graph ★

**What goes wrong:** CORE-03 says *"`StdRng` and `SmallRng` are absent from the dependency
graph."* CLAUDE.md reinforces it: *"`SmallRng`, `ReseedingRng`, feature `small_rng` — Removed /
not portable"*. A plan task written as "verify `SmallRng` is absent" will either fail
permanently or, worse, be marked done on a grep that does not test what it claims.

**Why it happens:** the `small_rng` *feature* was removed in rand 0.10 — but the **type became
unconditional**, not absent. `rand-0.10.2/src/rngs/mod.rs` reads verbatim:

```
97: mod small;
98: mod xoshiro128plusplus;
99: mod xoshiro256plusplus;
...
106: pub use self::small::SmallRng;
107: pub use xoshiro128plusplus::Xoshiro128PlusPlus;
108: pub use xoshiro256plusplus::Xoshiro256PlusPlus;
...
110: #[cfg(feature = "std_rng")]
111: pub use self::std::StdRng;
```

Lines 97-108 have **no `#[cfg]`**. Only `StdRng` (line 110) is feature-gated.

**Verified by compiling**, under `default-features = false, features = ["std", "chacha"]`:

```
error[E0433]: failed to resolve: could not find `StdRng` in `rngs`     <- StdRng IS gone
error[E0423]: expected function, found module `rand::rng`              <- rand::rng() IS gone
```
but
```
SmallRng IS available: 1012762419733073422 1012762419733073422
```
(`SmallRng::from_seed([7u8;32])` compiled and ran; its output is identical to
`Xoshiro256PlusPlus::from_seed([7u8;32])`, confirming `SmallRng` is an alias for it on 64-bit.)

**How to avoid:** restate CORE-03 as two separable, individually testable claims.
1. *`StdRng` and `rand::rng()`/`SysRng` are absent from the dependency graph* — **true and
   testable**, and the test is a compile failure. Add a `tests/` compile-fail case or a
   `cargo tree` assertion that `getrandom` is not present (verified absent).
2. *`SmallRng` and the Xoshiro generators are never **used*** — enforced by three
   `clippy.toml disallowed-types` entries (verified to fire) plus a `grep -r SmallRng src/`
   test. "Absent from the graph" is unachievable without forking `rand`.

**Warning signs:** a plan task phrased as "confirm `SmallRng` absent" with no stated mechanism.

### Pitfall 2: `cargo clippy` without `--all-targets` does not lint `tests/` ★

**What goes wrong:** criterion 4 says *"`cargo clippy` **fails the build** when code introduces a
`HashMap`/`HashSet` on a behaviour path"*. A CI job running plain `cargo clippy` passes green
while `tests/` is full of `HashMap`.

**Verified this session** — the same crate, with a `HashMap` in `tests/it.rs` and the lints
configured:

| Command | Exit |
|---|---|
| `cargo build` | **0** (clippy lints are not rustc lints) |
| `cargo test` | **0** |
| `cargo clippy` | **0** ← the trap |
| `cargo clippy --all-targets` | **101** |

**How to avoid:** the CI command and the plan's verification command must both be

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Warning signs:** any verification step in the plan that reads `cargo clippy` bare.

### Pitfall 3: `rand`'s own samplers are neither fixed-draw nor stable across upgrades

Covered in full under Pattern 2. The short form: `random_range` can consume a second word,
`Uniform::sample` loops without bound, and `seq::index::sample` picks between three algorithms
using `f32` thresholds that a `rand` patch release may retune. All three verified from the
vendored 0.10.2 source. Hand-roll `below` / `sample_k`.

### Pitfall 4: default `[profile.release]` silently wraps

Verified: `i64::MAX - 1 + 6` produced no panic and the wrapped value in a default release build,
and panicked with `overflow-checks = true`. Because CLAUDE.md's `Money` design routes operators
through `checked_add`, a *money* overflow panics regardless — but any raw `i64` on the behaviour
path (goods units, headcounts, tick counters) is unprotected without CORE-02. Set it.

One consequence to note in the plan: keep `panic = "unwind"` (the default). If anyone sets
`panic = "abort"` for the release profile, `#[should_panic]` tests and `catch_unwind`-based
negative tests stop working — and Phase 2's negative invariant tests depend on that machinery.

### Pitfall 5: a `type` alias silently defeats `disallowed_types` ★

**What goes wrong:** the sim legitimately wants a `HashMap` for O(1) point lookups (CLAUDE.md
permits it: *"Must use `HashMap` for lookup performance only — and never iterate it"*). The
obvious escape hatch is a type alias behind an `#[allow]`. Verified: **every downstream use of
that alias is invisible to clippy.** The lint is then decorative.

**How to avoid:** use a **newtype wrapper struct**, not an alias, in exactly one module:

```rust
// src/lookup.rs -- the ONLY module with this allow
#![allow(clippy::disallowed_types)]
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash};
use std::collections::hash_map::DefaultHasher;

/// Point-lookup only. Deliberately exposes NO iteration.
pub struct Lookup<K, V>(HashMap<K, V, BuildHasherDefault<DefaultHasher>>);

impl<K: Eq + Hash, V> Lookup<K, V> {
    pub fn new() -> Self { Self(HashMap::default()) }
    pub fn get(&self, k: &K) -> Option<&V> { self.0.get(k) }
    pub fn insert(&mut self, k: K, v: V) -> Option<V> { self.0.insert(k, v) }
    pub fn len(&self) -> usize { self.0.len() }
    // NO iter(), NO keys(), NO values(), NO IntoIterator, NO Debug that prints contents.
}
```

The enforcement is the **absence of `iter()`**, not the lint. Clippy stops the accident; the
missing method stops the deliberate act. A plan verification step should assert
`grep -c 'fn iter' src/lookup.rs == 0` and that `#![allow(clippy::disallowed_types)]` appears in
exactly one file.

Also note: `DefaultHasher`'s hash values are not stable across Rust releases, so even a
`Lookup` must never have its contents persisted or its ordering observed.

### Pitfall 6: clippy silently ignores `disallowed-methods` paths it cannot resolve ★

**What goes wrong:** a typo in one of 66 float paths (`f64::log_2` for `f64::log2`) is accepted
without a diagnostic. The list *looks* complete and one entry does nothing.

**Verified:** with all 33 `f64` paths configured, a file calling 28 of them produced exactly 28
errors and **no warning at all** about `gamma`, `ln_gamma`, `erf`, `erfc` (unstable → path does
not resolve). `abs_sub` *does* resolve and fires.

**How to avoid:** make the list self-testing. Add a `tests/` file (or a `#[cfg(test)]` module)
that calls every banned method once, and a CI step asserting
`cargo clippy --all-targets 2>&1 | grep -c 'disallowed method' == 58` (29 resolvable `f64` +
29 resolvable `f32`). Then a typo shows up as a count mismatch, not as silence. The four
unstable methods are the known, documented exclusions.

### Pitfall 7: `Option<T>` is a serde default with no attribute to grep for ★

**What goes wrong:** criterion 3 says *"`grep` finds no `#[serde(default)]` anywhere"*. Verified
that this grep passes while the config still silently defaults:

```
struct P { a: u32, b: Option<u32>, #[serde(default)] c: u32 }
toml::from_str::<P>("a = 1")  ->  Ok(P { a: 1, b: None, c: 0 })
```

Field `b` defaulted to `None` **with no attribute present**. A parameter typed `Option<f64>`
is exactly the "hidden hardcoded parameter" CORE-10 exists to forbid, and the specified grep
does not see it.

**How to avoid:** the verification is two greps plus one positive test.
1. `! grep -rq 'serde(default' src/` — no explicit defaults.
2. `! grep -rq 'Option<' src/config.rs` — no implicit defaults. (If an optional parameter is
   ever genuinely wanted, it must be a documented, reviewed exception with a `# GRADE: PROJECT`
   annotation.)
3. A test that deletes each key in turn from the shipped config and asserts every deletion
   produces `missing field \`<name>\`` — the only check that actually proves it. At ~30
   parameters this is a cheap loop over `toml::Value` in a test, not 30 hand-written cases.

`Vec<T>` behaves correctly (verified: an absent `Vec` field errors with `missing field`), so
only `Option` is the hazard.

### Pitfall 8: `sha2` 0.11.0 breaks the `{:x}` hex idiom

Verified by compile failure: `sha2` 0.11.0's `Sha256::digest` returns `hybrid_array::Array`,
which does not implement `LowerHex`:

```
error[E0277]: the trait bound `Array<u8, UInt<...>>: LowerHex` is not satisfied
```

`cargo add sha2` today resolves **0.11.0**, not the 0.10.x that CLAUDE.md specifies. Pin
`sha2 = "0.10.9"` explicitly, or hex-encode by hand. A secondary consequence of choosing 0.10:
it pulls `cpufeatures 0.2.17` while `chacha20 0.10.2` pulls `cpufeatures 0.3.1`, so the graph
carries both — harmless, but it will show up in `cargo tree` review.

### Pitfall 9: reusing an RNG sub-stream key replays it

Verified. Covered under Pattern 1. Add a debug-mode issued-key set.

### Pitfall 10: `--seed` must be recorded as the *effective* seed

CLAUDE.md specifies `--seed` overrides `config.seed`. That makes the effective seed an input
that is **not** in the config file, which is the class of thing CLAUDE.md otherwise forbids
(*"An env var is an input that is not in the committed config and not in the log; a run
configured that way cannot be reproduced from the repository"*). The mitigation CLAUDE.md
already names — *"recorded in `run_meta.json` as the effective seed"* — is load-bearing, and
`run_meta.json` is excluded from the determinism diff. Make sure the plan pairs them: the seed
override is only safe **because** the effective seed is recorded. CAL-06 ("reproducible from
the committed config and seed") depends on it.

---

## The `f64` vs `i64` Decision for `expected_demand`

The roadmap flags this as the phase's second research question. Both options work. Here is the
cost of each, measured.

### The case for `f64`, restricted

`expected_demand += λ · (last_sales − expected_demand)` uses only `+`, `−`, `×`. IEEE-754
requires these to be **correctly rounded** — one uniquely determined result per input pair.
Rust has no `-ffast-math` on stable, does not reassociate float expressions, and does not
contract into FMA. Same binary, same platform, repeated runs: bit-identical. On x86-64 and
aarch64 (SSE2 / NEON, true 64-bit registers, no x87 excess precision) this also holds
*across* machines, which is what Phase 11 criterion 5 needs. The i686 x87 hazard does not
apply — this project has no 32-bit x86 target.

Costs of choosing it:
- One named crossing function, `demand_to_units(f64) -> i64`, with `round()` (correctly
  rounded) and a saturating `as` cast (defined and deterministic in Rust).
- A NaN/∞ guard at the crossing. NaN *sign and payload are non-deterministic* per std's
  primitive docs, and NaN poisons comparisons silently.
- The 66-entry clippy list must be written and kept honest (Pitfall 6).
- `expected_demand` must be logged at full round-trip precision, never truncated.

### The case for `i64` milli-units

`ed += (λ_milli · (obs_milli − ed)) / 1000`. Exact, no float domain, no crossing function, no
NaN, no clippy list needed for the behaviour path.

**Measured, this session**, λ = 0.25, 3650 ticks, `obs = 40 + (t mod 7)`:

```
ed_int  = 42408 milli (42.408000 units)
ed_f64  = 42.408044
max |difference| over the whole run = 2.064e-3 units
integer update dead-band: |obs - ed| must exceed 3 milli-units (0.003 units) to move ed
```

A 0.005 % terminal divergence and a 0.003-unit dead band. Economically invisible at
productivity 3 and inventories in the tens.

Costs of choosing it:
- Every derived formula must be rescaled by hand: PLAN-04's band `[0.25, 1.0] × E` becomes
  `inv·4000 ≤ ed` / `inv·1000 ≥ ed`; LABR-01's `L_d = E / productivity` becomes
  `(ed + 500) / (1000 · productivity)` with an explicit rounding choice.
- Truncating integer division rounds toward zero, so the update systematically undershoots in
  both directions — self-correcting, but it must be *documented* rather than discovered.
- **It does not remove the float domain**, because MKT-01's `(m/P̄)^0.9` still needs it (Pattern
  7). The clippy list is required either way. This is the argument that actually decides it.

### Recommendation

**Use `f64`, restricted to `+ − × ÷ sqrt`, confined to `src/numeric.rs` plus the
`expected_demand` field, with one named crossing function and a `debug_assert!(x.is_finite())`
at the crossing.** The `i64` route's headline benefit — abolishing the float domain — is not
available, because the consumption exponent needs floats regardless. Given the domain must
exist, keeping `expected_demand` in it costs nothing extra and keeps the code readable against
the published formula, which matters for CORE-11-style verification against the paper.

This agrees with STACK.md:465's recommendation, but for a reason STACK.md did not have: it did
not notice that MKT-01 forces the float domain to exist.

Confidence: **HIGH** on the mechanics (all measured); **MEDIUM** on the recommendation, since a
reasonable person could still prefer the integer route for its total absence of a rounding
story. This is Open Question 2 for `/gsd-discuss-phase`.

---

## Source Grades and Lengnick Table 1 (CORE-11)

### The A/B/C/PROJECT scheme is already defined in-repo — do not invent it

[VERIFIED: `/home/user/Sim/.planning/research/SUMMARY.md:169`, read this session. Quoted verbatim:]

> Every value carries its source grade: **A** = model authors' own code, **B** = annotated
> replication citing the paper's table/equation numbers, **C** = derived arithmetic, **PROJECT**
> = a choice with no published precedent. These close the gaps in the project's own parameter
> table and are the single most valuable research output.

The complete graded parameter table already exists at `SUMMARY.md:171-209` (37 rows). The
config-annotation task is therefore **transcription plus a schema**, not research. The
convention should be a TOML comment block above each key:

```toml
[firm]
# Price adjustment bound, x(1 +/- U(0, upsilon))
# GRADE: B | SOURCE: Lengnick 2013 Table 1 (via newwayland/baseline-economy, annotated) | CADENCE: month
price_step_ppm = 20_000        # 0.02

# Demand-expectation smoothing, E += lambda(obs - E)
# GRADE: A | SOURCE: Caiani et al., jmab SimpleAdaptiveExpectation `adaptiveParam` | CADENCE: period
lambda_ppm = 250_000           # 0.25
```

A test should assert that **every** key in the shipped config is preceded by a comment line
matching `^# GRADE: (A|B|C|PROJECT) \|`. That converts criterion 5's first clause from a
review item into an automated gate. Parsing the raw TOML text with a small regex is sufficient;
`toml` discards comments, so this must read the file as text.

### The paper verification is BLOCKED in this environment

CORE-11's second clause — *"Lengnick Table 1 values are verified against the published paper"* —
**could not be performed.** Every candidate host is denied by the network egress proxy:

| Source | Result |
|---|---|
| `sciencedirect.com` (publisher of record, JEBO 86 (2013) 102-120) | egress-blocked (403 CONNECT) |
| `legacy.econ.tuwien.ac.at/.../jebo_2013_agent_based_macroeconomics_a_baseline_model.pdf` (open-access mirror of the published PDF — **the most promising target**) | egress-blocked |
| `macau.uni-kiel.de/.../Dissertation_Lengnick.pdf` (Kiel dissertation containing the chapter) | egress-blocked |
| `econstor.eu` / `ideas.repec.org` (working-paper version) | egress-blocked |
| `sim4edu.com/sims/20/description` (JavaScript replication with a parameter list) | egress-blocked |
| `github.com/newwayland/baseline-economy` (the grade-B replication the values came from) | reachable as a web page, but the repo's code/README is **not** exposed to this session's GitHub API scope |

STATE.md already records this as a known blocker: *"Lengnick Table 1 values are grade B (from an
annotated replication, not read from the paper) — verification is an explicit Phase 1 task
(CORE-11)."* SUMMARY.md:323 records the same for the original research round: *"**The primary
paper PDFs were egress-blocked** — no Lengnick value here was read from the paper."*

**This is now the second independent research pass to hit the same wall.** The planner should
therefore treat CORE-11's verification clause as a **`checkpoint:human-verify` task**, not an
agent task, and should not schedule another automated attempt. Concretely:

- **Task shape:** a human with journal access (or an unproxied network) opens the TU Wien PDF —
  which is the *published* JEBO article, i.e. grade A for this purpose — and checks the 18
  Lengnick-attributed rows of `SUMMARY.md:171-209` against Table 1.
- **Deliverable:** a `config/PROVENANCE.md` recording, per row, `agrees` / `differs (paper says
  X)` / `not in Table 1`. Criterion 5 requires *"any discrepancy recorded rather than silently
  adopted"* — so a differing value must be written down **and** the config updated with a note,
  never overwritten silently.
- **Blast radius if skipped:** the 18 grade-B values feed Phases 6, 7 and 9, and STATE.md
  already names the reservation-wage/wage-step coupling as *"the widest-sensitivity parameter
  region in the model"*. This is the cheapest de-risking action available in the whole project.
- **What must NOT happen:** an agent transcribing Table 1 from training memory. Every
  Lengnick-attributed number in this document is `[ASSUMED]` and inherited from
  `SUMMARY.md`; none was read from a primary source in this session.

---

## Code Examples

### 1. `Cargo.toml` — verified wiring

```toml
[package]
name = "sim"
version = "0.1.0"
edition = "2024"

[dependencies]
rand      = { version = "0.10.2", default-features = false, features = ["std", "chacha"] }
serde     = { version = "1.0.229", features = ["derive"] }
toml      = "1.1.4"
sha2      = "0.10.9"          # NOT 0.11 -- see Pitfall 8
thiserror = "2.0.20"
clap      = { version = "4.6.6", features = ["derive"] }
anyhow    = "1.0.104"

[dev-dependencies]
proptest = "1.11.0"

[profile.release]
overflow-checks = true        # CORE-02 -- verified: default release silently wraps

[lints.clippy]
disallowed_types   = "deny"   # verified: makes `cargo clippy` exit 101
disallowed_methods = "deny"
```

### 2. `rust-toolchain.toml` — verified to resolve on this machine

```toml
[toolchain]
channel    = "1.94.1"
components = ["clippy", "rustfmt"]
profile    = "minimal"
```

`rustup show active-toolchain` then reports
`1.94.1-x86_64-unknown-linux-gnu (overridden by '…/rust-toolchain.toml')`, and `rustc --version`
reports `rustc 1.94.1 (e408947bf 2026-03-25)`.

### 3. `clippy.toml` — generate the float list, don't type it

Typing 66 paths by hand invites the silent-typo failure of Pitfall 6. Generate it from the
enumerated list and commit the generated file:

```bash
python3 - <<'PY' > clippy.toml
f64_methods = """powi powf exp exp2 ln log log2 log10 abs_sub cbrt hypot
sin cos tan asin acos atan atan2 sin_cos
exp_m1 ln_1p sinh cosh tanh asinh acosh atanh
gamma ln_gamma erf erfc to_degrees to_radians""".split()
assert len(f64_methods) == 33
print('# GENERATED -- see docs. 33 methods per float type carry std\'s')
print('# "# Unspecified precision ... can even differ within the same execution".')
print('disallowed-types = [')
for p, r in [("std::collections::HashMap", "nondeterministic iteration order"),
             ("std::collections::HashSet", "nondeterministic iteration order"),
             ("rand::rngs::SmallRng", "non-portable; CORE-03"),
             ("rand::rngs::Xoshiro256PlusPlus", "not the project RNG"),
             ("rand::rngs::Xoshiro128PlusPlus", "not the project RNG")]:
    print(f'  {{ path = "{p}", reason = "{r}" }},')
print(']')
print('disallowed-methods = [')
for t in ("f64", "f32"):
    for m in f64_methods:
        print(f'  {{ path = "{t}::{m}", reason = "unspecified precision (std)" }},')
for p in ("std::time::SystemTime::now", "std::time::Instant::now"):
    print(f'  {{ path = "{p}", reason = "wall clock is not an input to the sim" }},')
print(']')
PY
```

*(This exact generator shape was used to produce the list that was compile-tested; 28 of the
28 stable, callable `f64` methods fired, plus `abs_sub` = 29.)*

### 4. `Money` — the verified overflow shape

```rust
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("money overflow: {lhs} {op} {rhs}")]
pub struct MoneyOverflow { pub lhs: i64, pub op: &'static str, pub rhs: i64 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
         serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Money(i64);          // private field: no construction outside this module

impl Money {
    pub const ZERO: Money = Money(0);
    /// The ONLY constructor. Used in config parsing and initial endowment only.
    pub const fn from_cents(c: i64) -> Money { Money(c) }
    pub const fn cents(self) -> i64 { self.0 }

    /// Named checked API -- Result, for config ingestion.
    pub fn checked_add(self, o: Money) -> Result<Money, MoneyOverflow> {
        self.0.checked_add(o.0).map(Money)
            .ok_or(MoneyOverflow { lhs: self.0, op: "+", rhs: o.0 })
    }
}

// Operator API -- panics in EVERY profile. Verified.
impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, o: Money) -> Money {
        Money(self.0.checked_add(o.0).expect("Money overflow on add"))
    }
}
// Deliberately absent: From<f64>, Into<f64>, Mul<f64>, decimal Display,
// and `impl Sum` via fold(0, +) on the raw i64.
```

### 5. Criterion 2's test, in the shape the criterion asks for

```rust
#[test]
fn same_master_seed_identical_streams() {
    let a = Rngs::new(20260830);
    let b = Rngs::new(20260830);
    let xa: Vec<u64> = (0..64).map(|i| a.stream(i, 0, Purpose::PriceStep).below(u64::MAX)).collect();
    let xb: Vec<u64> = (0..64).map(|i| b.stream(i, 0, Purpose::PriceStep).below(u64::MAX)).collect();
    assert_eq!(xa, xb);
}

#[test]
fn different_master_seed_differs() {          // the counter-check criterion 2 demands
    let a = Rngs::new(20260830);
    let b = Rngs::new(20260831);
    assert_ne!(a.stream(0,0,Purpose::PriceStep).below(u64::MAX),
               b.stream(0,0,Purpose::PriceStep).below(u64::MAX));
}

#[test]
fn extra_draws_in_one_purpose_cannot_perturb_another() {   // CORE-04, the whole point
    let r = Rngs::new(20260830);
    let baseline: Vec<u64> = {
        let mut s = r.stream(10, 7, Purpose::GoodsSample);
        (0..4).map(|_| s.below(1_000_000)).collect()
    };
    // simulate a code change that adds three draws to the labour market
    { let mut s = r.stream(10, 7, Purpose::LabourSample); for _ in 0..7 { s.below(1_000_000); } }
    let after: Vec<u64> = {
        let mut s = r.stream(10, 7, Purpose::GoodsSample);
        (0..4).map(|_| s.below(1_000_000)).collect()
    };
    assert_eq!(baseline, after);
}

#[test]
fn distinct_keys_give_distinct_streams() {
    let r = Rngs::new(20260830);
    let mut seen = std::collections::BTreeSet::new();
    for tick in 0..40 { for agent in 0..40 { for p in ALL_PURPOSES {
        assert!(seen.insert(r.stream(tick, agent, p).below(u64::MAX)), "collision");
    }}}
}

#[test]
fn sample_k_consumes_exactly_k_draws() {      // CORE-05
    let r = Rngs::new(1);
    let mut s = r.stream(0, 0, Purpose::GoodsSample);
    let mut pool: Vec<u32> = (0..20).collect();
    let picked = s.sample_k(&mut pool, 5);
    assert_eq!(s.draws(), 5);
    assert_eq!(picked.iter().collect::<std::collections::BTreeSet<_>>().len(), 5);
}
```

*(The first four of these were executed in equivalent form this session and all passed; the
collision test ran over 9 600 sub-streams with zero collisions.)*

### 6. Criterion 3's config test — the exhaustive missing-key loop

```rust
#[test]
fn every_key_is_required() {
    let raw = std::fs::read_to_string("config/baseline.toml").unwrap();
    let doc: toml::Value = toml::from_str(&raw).unwrap();
    for (table, key) in enumerate_leaf_keys(&doc) {           // small helper
        let mut mutated = doc.clone();
        remove(&mut mutated, &table, &key);
        let err = toml::from_str::<Params>(&toml::to_string(&mutated).unwrap())
            .expect_err(&format!("{table}.{key} is not required -- hidden default?"));
        assert!(err.to_string().contains(&format!("missing field `{key}`")),
                "wrong error for {table}.{key}: {err}");
    }
}

#[test]
fn unknown_key_is_rejected() {
    let raw = std::fs::read_to_string("config/baseline.toml").unwrap();
    let err = toml::from_str::<Params>(&format!("{raw}\n[sim]\nhouseolds = 1\n")).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}
```

The first test is what actually enforces CORE-10; the `grep` for `#[serde(default)]` is a
cheap complement, not a substitute (Pitfall 7).

### 7. Criterion 4's negative test for the lints

```bash
# tests/lints.sh -- run in CI
set -e
cargo clippy --all-targets --all-features -- -D warnings   # must pass on clean tree

# and must FAIL when a hazard is introduced
cat > /tmp/hazard.rs <<'EOF'
pub fn h() -> std::collections::HashMap<u32,u32> { std::collections::HashMap::new() }
pub fn g(x: f64) -> f64 { x.powf(2.0) }
EOF
cp /tmp/hazard.rs src/_hazard.rs
echo 'mod _hazard;' >> src/lib.rs
! cargo clippy --all-targets --all-features -- -D warnings   # MUST fail
git checkout src/lib.rs && rm src/_hazard.rs
```

A lint never observed to fire has never been shown to work — the same discipline PITFALLS.md
applies to invariants.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `rand_chacha` crate for `ChaCha8Rng` | `rand` `chacha` feature → `chacha20` crate | rand 0.10.0 (changelog #1642) | Byte-identical output; `rand_chacha`'s own README now calls itself *"formerly the implementation behind `StdRng`"*. Use the `chacha` feature. |
| `RngCore` trait / `Rng` extension trait | **`Rng`** is the core trait; **`RngExt`** is the extension | rand 0.10 | Verified: `rand-0.10.2/src/lib.rs:59` `pub use rand_core::{CryptoRng, Rng, SeedableRng, …};` and `:72` `pub use rng::{Fill, RngExt};`. Any 0.9-era snippet is wrong. |
| `rng.gen()` / `gen_range()` | `rng.random()` / `random_range()` | rand 0.10 | — |
| `choose_multiple` | `sample` | rand 0.10 | — but hand-roll it anyway (Pattern 2). |
| feature `small_rng` gating `SmallRng` | **`SmallRng` is unconditional; only `std_rng` gates `StdRng`** | rand 0.10 | Verified. This is the CORE-03 correction (Pitfall 1). |
| `OsRng` | `SysRng`, gated on feature `sys_rng` | rand 0.10 | Absent under our feature set — verified, and `getrandom` is not in the tree. |
| `toml` 0.8 API notes | `toml` **1.x**; default features `std, serde, parse, display` | toml 1.0 | Do not assume 0.8 guidance applies. |
| `sha2` 0.10 `GenericArray` (impls `LowerHex`) | `sha2` 0.11 `hybrid_array::Array` (**does not**) | sha2 0.11.0 | Pin 0.10.9 or hex by hand. Verified by compile failure. |

**Deprecated/outdated:**
- `rand::rngs::StdRng` / `SmallRng` for reproducible work — rand's own docs disclaim portability.
- `fastrand` — no cross-version output-stability contract.
- `f64::abs_sub` — `#[deprecated]` in std but still present and still lint-bannable.
- `figment` 0.10.19 — last released 2024-05-17; a `figment2` fork exists.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The 18 Lengnick-attributed parameter values in `SUMMARY.md:171-209` (α=0.9, productivity=3, υ=0.02, θ=0.75, φ∈[0.25,1.0], ϑ∈[1.025,1.15], δ=0.019, γ=24, χ=0.1, ×0.9 decay, n=7, ζ=0.01, ψ=0.25/0.25, β=5, π=0.1, 21-day month, …) match the published Table 1. | Source Grades / CORE-11 | **HIGH.** These drive Phases 6, 7 and 9. Grade B, from an annotated replication. Not read from the paper in this session or the previous one. Human verification required. |
| A2 | The published paper is reachable at `legacy.econ.tuwien.ac.at/lva/compeco.se/artikel/jebo_2013_agent_based_macroeconomics_a_baseline_model.pdf`. | CORE-11 | LOW. If it is not, the human falls back to journal access. The URL came from a web search result title, not from a fetch. |
| A3 | ChaCha8's nonce-separated keystreams are statistically independent enough for a 200-agent ABM. | Pattern 1 | LOW. ChaCha's nonce exists for exactly this purpose and the best published distinguishers reach 7 rounds. Mitigation if ever doubted: switch to `ChaCha12Rng`/`ChaCha20Rng` — same API, same portability guarantee, ~1.5×/3× cost. |
| A4 | The build target is x86-64 or aarch64 (no i686 x87 excess precision). | `f64` vs `i64` | MEDIUM if wrong. On i686 the `f64` recommendation would need revisiting. Nothing in the planning documents names a target triple. |
| A5 | `getrandom` being absent from `cargo tree` means no OS entropy path exists. | Standard Stack | LOW. The tree was read directly; `rand::rng()` and `SysRng` were also confirmed to not compile. |
| A6 | 3 212 000 sub-streams is the realistic upper bound (3650 ticks × 220 agents × ~4 purposes). | Pattern 1 | LOW. If purposes reach 10 per agent-tick, cost scales linearly to ~0.6 s — still fine. |
| A7 | The `Vec<Household>`/`Vec<Firm>` counts (200/20) and 3650 ticks are the Phase 1 sizing assumptions. | throughout | LOW. Stated in PROJECT.md and REQUIREMENTS.md. |

---

## Open Questions

1. **RNG sub-stream keying scheme.**
   - What we know: `set_stream(u64)` exists and works; bit-packing is bijective and 5.9× cheaper
     than SHA-256 child seeds; isolation verified by execution.
   - What's unclear: nothing mechanical. The remaining question is whether the project wants the
     unbounded key space a hash would give.
   - Recommendation: **bit-packed `set_stream`, `tick:24 | agent:24 | purpose:16`.** Lock it in
     `/gsd-discuss-phase`. The bit allocation itself is worth confirming with the user, since
     widening `purpose` later re-keys everything.

2. **`f64` vs `i64` milli-units for `expected_demand`.**
   - What we know: both are deterministic; divergence over 3650 ticks is 2.1e-3 units;
     `+ − × ÷ sqrt` are correctly rounded; the float domain must exist regardless because of
     MKT-01.
   - What's unclear: only preference.
   - Recommendation: **restricted `f64`**, confined to `src/numeric.rs` plus the field itself.

3. **How `(m/P̄)^0.9` is computed (blocks the clippy list).**
   - What we know: `powf` is banned and genuinely non-deterministic; `pow_frac_det` reproduces it
     to 1.9e-12 using only `sqrt` and `*`, bit-identically across 100k invocations.
   - What's unclear: whether the project would rather change α to a value with a closed integer
     form, or accept the approximation.
   - Recommendation: **ship `pow_frac_det` in Phase 1** with `bits = 40`. Do not weaken the ban.

4. **Where numerical-method constants live.**
   - What we know: CORE-10 says *"Every simulation parameter loads from a TOML config … no serde
     defaults"*. `pow_frac_det`'s `bits`, and the milli/ppm scale factors, change trajectories
     but are not economics.
   - What's unclear: whether CORE-10 is meant to cover them.
   - Recommendation: **code constants**, documented, with a `# GRADE: PROJECT` note in
     `config/PROVENANCE.md` explaining why they are not config. Confirm with the user — a strict
     reading of CORE-10 says otherwise.

5. **`Money` overflow: panic vs `Result`.**
   - Recommendation: **both**, split by API surface (Pattern 3). Needs one line of confirmation
     so the executor does not implement one and delete the other.

6. **How CORE-03 is restated so it is testable.**
   - Recommendation: split into "StdRng/SysRng absent from the graph" (testable, true) and
     "SmallRng/Xoshiro never used" (clippy + grep). Needs a REQUIREMENTS.md amendment or an
     explicit planner note, otherwise the phase gate is unpassable as written.

7. **Is `HashMap` needed at all in v1?**
   - What we know: CLAUDE.md permits lookup-only use; Pitfall 5 shows the escape hatch is
     delicate.
   - Recommendation: **ban it outright in Phase 1 with no `Lookup` wrapper**, and add the wrapper
     only if a later phase demonstrably needs it. Every relation in the v1 model is dense-integer
     keyed (`Vec` by ID) or small enough for `BTreeMap`. Not building the escape hatch is the
     cheapest way to keep the lint honest.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `rustc` | everything | ✓ | 1.94.1 (e408947bf 2026-03-25) | — |
| `cargo` | everything | ✓ | 1.94.1 (29ea6fb6a 2026-03-24) | — |
| `rustup` | `rust-toolchain.toml` pin | ✓ | present; `1.94.1` channel synced | — |
| `clippy` | CORE-07 | ✓ | `clippy-x86_64-unknown-linux-gnu` installed | — |
| `rustfmt` | hygiene | ✓ | installed | — |
| `rust-src` | verifying std claims | ✓ | installed (used for the 33-method enumeration) | — |
| crates.io registry | dependency resolution | ✓ | `index.crates.io` is on the proxy no-proxy list; `cargo add` resolved live | vendored cache already holds rand/chacha20/rand_core/cfg-if/cpufeatures |
| `python3` | the `clippy.toml` generator, Phase 4 harness | ✓ | `/usr/local/bin/python3` | — |
| `uv` | Phase 4 Python env | ✓ | `/root/.local/bin/uv` | — |
| `gh` CLI | — | ✗ | not installed | GitHub API is scope-limited anyway; not needed by Phase 1 |
| Academic publisher access (ScienceDirect / EconStor / TU Wien / Kiel) | **CORE-11** | ✗ | egress-blocked | **No fallback available to an agent.** Requires `checkpoint:human-verify`. |

**Missing dependencies with no fallback:**
- Primary-source access to Lengnick (2013) Table 1 — blocks CORE-11's verification clause. The
  annotation clause (source grades in the config) is unaffected and fully deliverable.

**Missing dependencies with fallback:**
- `gh` CLI — not needed by this phase.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (libtest) + `proptest` 1.11.0 |
| Config file | none — `tests/` directory convention; Wave 0 creates it |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test --all-targets && cargo clippy --all-targets --all-features -- -D warnings` |

Note the full-suite command includes clippy: for this phase clippy **is** a test, because
criterion 4 is a lint assertion.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CORE-01 | `Money` ops panic on overflow in debug **and** release | unit | `cargo test --lib money:: && cargo test --release --lib money::` | ❌ Wave 0 (`src/money.rs`) |
| CORE-01 | `split(n)` sums exactly, non-even amounts | property | `cargo test --test money_props` | ❌ Wave 0 (`tests/money_props.rs`) |
| CORE-02 | default release wraps, checked release panics | unit | `cargo test --release --lib money::raw_i64_overflow_panics` | ❌ Wave 0 |
| CORE-03 | `StdRng`/`rand::rng()` do not resolve | compile-fail / grep | `! grep -rq 'StdRng\|SmallRng\|Xoshiro' src/` + `! cargo tree \| grep -q getrandom` | ❌ Wave 0 (`tests/lints.sh`) |
| CORE-04 | same seed → same stream | unit | `cargo test --test determinism_rng same_master_seed` | ❌ Wave 0 (`tests/determinism_rng.rs`) |
| CORE-04 | different seed → different stream | unit | `cargo test --test determinism_rng different_master_seed` | ❌ Wave 0 |
| CORE-04 | extra draws in purpose A cannot perturb purpose B | unit | `cargo test --test determinism_rng extra_draws_cannot_perturb` | ❌ Wave 0 |
| CORE-04 | distinct keys → distinct streams (9 600 sample) | unit | `cargo test --test determinism_rng distinct_keys` | ❌ Wave 0 |
| CORE-05 | `sample_k(n, k)` consumes exactly `k` draws | unit | `cargo test --test determinism_rng sample_k_exact_draws` | ❌ Wave 0 |
| CORE-05 | `below(n)` consumes exactly 1 draw for all `n` | property | `cargo test --test determinism_rng below_one_draw` | ❌ Wave 0 |
| CORE-06 | stale `FirmId` at old `gen` resolves to `None` | unit | `cargo test --lib ids::stale_gen_is_none` | ❌ Wave 0 (`src/ids.rs`) |
| CORE-07 | clippy fails on an introduced `HashMap` / `powf` | script | `bash tests/lints.sh` | ❌ Wave 0 (`tests/lints.sh`) |
| CORE-07 | all 58 resolvable float bans actually fire | script | `bash tests/lints.sh` (count assertion) | ❌ Wave 0 |
| CORE-08 | integration tests can `use sim::*` | compile | `cargo test --test determinism_rng` (its existence proves it) | ❌ Wave 0 |
| CORE-09 | lockfile + toolchain committed, no rayon, no target-cpu | script | `git ls-files --error-unmatch Cargo.lock rust-toolchain.toml && ! grep -q rayon Cargo.toml && ! test -e .cargo/config.toml` | ❌ Wave 0 |
| CORE-10 | unknown key rejected with a named error | unit | `cargo test --test config_strict unknown_key` | ❌ Wave 0 (`tests/config_strict.rs`) |
| CORE-10 | **every** key is required (exhaustive deletion loop) | unit | `cargo test --test config_strict every_key_is_required` | ❌ Wave 0 |
| CORE-10 | no `#[serde(default)]`, no `Option<` in config.rs | script | `! grep -rq 'serde(default' src/ && ! grep -q 'Option<' src/config.rs` | ❌ Wave 0 |
| CORE-10 | config hash reproducible | unit | `cargo test --test config_strict hash_is_stable` | ❌ Wave 0 |
| CORE-11 | every config key carries `# GRADE: (A\|B\|C\|PROJECT)` | unit | `cargo test --test config_strict every_key_has_a_source_grade` | ❌ Wave 0 |
| CORE-11 | Table 1 checked against the paper | **manual-only** | `checkpoint:human-verify` → `config/PROVENANCE.md` | ❌ Wave 0 — **cannot be automated; publisher access is egress-blocked** |

### Sampling Rate

- **Per task commit:** `cargo test --lib` (sub-second at this size)
- **Per wave merge:** `cargo test --all-targets && cargo clippy --all-targets --all-features -- -D warnings && bash tests/lints.sh`
- **Phase gate:** the full suite green in **both** profiles —
  `cargo test --all-targets && cargo test --release --all-targets` — because CORE-01/CORE-02 are
  profile-dependent and a debug-only run cannot see the wrapping bug.

### Wave 0 Gaps

- [ ] `src/money.rs`, `src/ids.rs`, `src/config.rs`, `src/rng.rs`, `src/numeric.rs`, `src/lib.rs`, `src/main.rs` — no source exists yet
- [ ] `tests/determinism_rng.rs` — covers CORE-04, CORE-05
- [ ] `tests/config_strict.rs` — covers CORE-10, CORE-11 (annotation clause)
- [ ] `tests/money_props.rs` — covers CORE-01 (proptest)
- [ ] `tests/lints.sh` — covers CORE-07, CORE-09 (a shell script, because it must assert that a
      build *fails*, which libtest cannot express)
- [ ] `clippy.toml`, `rust-toolchain.toml`, `Cargo.toml [lints]` / `[profile.release]`
- [ ] `config/baseline.toml` with `# GRADE:` annotations, and `config/PROVENANCE.md`
- [ ] `.proptest-regressions` — will be created on first counterexample; must be committed
- [ ] Framework install: **none needed.** libtest is built in; `proptest` is a `dev-dependency`.

**Nyquist note:** this phase's tests run in well under a second, so per-task sampling is free.
The one expensive gate is running the suite twice (debug + release), which is still seconds.

---

## Security Domain

`security_enforcement: true`, `security_asvs_level: 1`. This is a **local, offline,
single-binary numerical simulation** with no network, no server, no authentication, no user
accounts, no persistence beyond files it writes into its own `--out` directory, and one
trusted input (a config file in the repository). Most ASVS categories are structurally
inapplicable; the honest analysis is short rather than padded.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | **no** | No identities, no sessions, no network surface. |
| V3 Session Management | **no** | No sessions. |
| V4 Access Control | **no** | Single local process; OS file permissions are the only boundary. |
| V5 Input Validation | **yes** | `serde` + `toml` with `#[serde(deny_unknown_fields)]`, no defaults, no `Option`. Verified to reject unknown fields, missing fields, and type mismatches (including float→int, which it does **not** coerce). The `--config` and `--out` paths come from `clap`; `--seed` is `u64`-parsed by `clap`. |
| V6 Cryptography | **partially** | `sha2::Sha256` is used **only** as a content digest for the config file and for log-file comparison in determinism tests — no key material, no secrets, no signatures. `ChaCha8Rng` is used as a **deterministic PRNG, not a cipher**; its seed is a *published* run parameter recorded in `run_meta.json` and is deliberately not secret. No hand-rolled cryptography. |
| V7 Error handling & logging | **yes (weakly)** | `thiserror` in the lib, `anyhow` in `main.rs`. TICK-06 already forbids wall-clock, hostname, path and PID in the diffed logs — which is a *determinism* rule that happens to also be an information-disclosure control. |
| V12 File handling | **yes (weakly)** | `--out` is an operator-supplied path the process writes into. Not attacker-controlled in any realistic threat model, but the plan should not `create_dir_all` on a path assembled from config *content*. |
| V14 Configuration | **yes** | `Cargo.lock` committed; `rust-toolchain.toml` pinned; supply chain audited above (all 9 crates OK, all first-party maintainers). |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious/typo'd crate in the dependency graph (slopsquatting) | Tampering | Package Legitimacy Audit above (all 9 OK, 3.5 M–31 M weekly downloads, all with named upstream repos); committed `Cargo.lock`; `cargo tree` reviewed and minimal (5 transitive crates for the RNG path). |
| Integer overflow producing a silently wrong value | Tampering | CORE-01 + CORE-02, both verified. This is simultaneously the project's #1 *correctness* control and its only real memory-safety-adjacent risk. |
| Panic-based denial of service from a malformed config | DoS | Config parse errors return `Result` and are reported by `main.rs`; `Money::from_cents` is the only construction point and config-supplied money goes through the `Result`-returning checked API, not the panicking operator. |
| Path traversal via `--out` | Tampering | Operator-supplied, not attacker-supplied. Do not build output paths from config *content*; keep `--out` a plain `PathBuf` joined only with fixed filenames. |
| Unsafe code | Tampering / memory safety | **There should be none.** Recommend `#![forbid(unsafe_code)]` in `src/lib.rs` — free, and it makes the claim auditable. Note `chacha20` and `cpufeatures` do contain `unsafe` for SIMD; that is accepted, audited RustCrypto code. |
| Secrets in logs | Info disclosure | No secrets exist. The seed is deliberately public. |

**Net assessment: LOW risk surface.** The security controls that matter here are the same
controls that deliver determinism — pinned toolchain, committed lockfile, strict input
validation, checked arithmetic, no threading, no ambient entropy. `#![forbid(unsafe_code)]` is
the only addition this analysis recommends that is not already required by CORE-01…CORE-11.

---

## Sources

### Primary (HIGH confidence) — first-hand on this machine, 2026-08-30

- **`rustc 1.94.1` / `cargo 1.94.1` / `clippy` 1.94** — every compile, run and lint result quoted
  in this document was produced here, in
  `/tmp/claude-0/-home-user-Sim/f7120805-c842-4be4-a5bb-5beb6db0da92/scratchpad/{rngprobe,clipprobe,ovf,cfg,pow}`.
- **Local rustc std source** (`rust-src` component,
  `$(rustc --print sysroot)/lib/rustlib/src/rust/library/{std,core}/src/num/f64.rs`) — the
  verbatim "# Unspecified precision" disclaimer and the exact 31 + 2 method enumeration, parsed
  programmatically rather than transcribed.
- **Vendored crate source, read directly:**
  - `~/.cargo/registry/src/index.crates.io-*/chacha20-0.10.2/src/rng.rs` — `set_stream`,
    `set_word_pos`, `set_block_pos`, `get_stream`, `serialize_state`; the 2^64-streams and
    1 ZiB doc text.
  - `~/.cargo/registry/src/index.crates.io-*/rand-0.10.2/src/rngs/mod.rs:97-119` — the feature
    gating that proves `SmallRng` is unconditional and `StdRng` is not.
  - `.../rand-0.10.2/src/lib.rs:56-72` — `Rng` / `RngExt` / `rng()` gating.
  - `.../rand-0.10.2/src/distr/uniform_int.rs:130-250` — Lemire vs Canon, biased vs unbiased,
    the rejection loops.
  - `.../rand-0.10.2/src/seq/index.rs:230-282` — the `f32`-heuristic algorithm dispatch.
  - `.../rand-0.10.2/Cargo.toml:58-85` — the feature table (`chacha`, `std_rng`, `sys_rng`,
    `thread_rng`, `unbiased`).
- **crates.io**, live via `cargo add --dry-run` — all 13 version numbers.
- **`gsd-tools query package-legitimacy check --ecosystem crates`** — the audit table.

### Secondary (MEDIUM confidence) — in-repo, read this session

- `/home/user/Sim/.claude/CLAUDE.md` — authoritative stack. Confirmed on 12 points; corrected on
  1 (`SmallRng`).
- `/home/user/Sim/.planning/REQUIREMENTS.md:12-22` (CORE-01…CORE-11, quoted verbatim above),
  `:81` (MKT-01), `:104` (PLAN-05), `:145-164` (Out of Scope).
- `/home/user/Sim/.planning/ROADMAP.md` — Phase 1 in full; Phases 2-11 skimmed for what these
  primitives must support.
- `/home/user/Sim/.planning/PROJECT.md`, `/home/user/Sim/.planning/STATE.md`.
- `/home/user/Sim/.planning/research/SUMMARY.md:169` (the grade scheme, quoted verbatim),
  `:171-209` (the graded parameter table), `:323` (the prior egress block).
- `/home/user/Sim/.planning/research/STACK.md:395-475` (the float boundary section, whose
  31-method count this research independently confirmed).
- `/home/user/Sim/.planning/research/PITFALLS.md:424, :450, :564, :638, :667` (the sub-stream
  `DESIGN` mandate).

### Tertiary (LOW confidence) — web, for CORE-11 only

- WebSearch for Lengnick (2013) Table 1 — located five candidate hosts, **all egress-blocked**.
  No parameter value in this document was obtained from any of them. Nothing here was
  transcribed from training memory.
  - [Agent-based macroeconomics: A baseline model — ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S0167268112002806)
  - [Open-access PDF mirror (TU Wien)](https://legacy.econ.tuwien.ac.at/lva/compeco.se/artikel/jebo_2013_agent_based_macroeconomics_a_baseline_model.pdf)
  - [Lengnick dissertation (Kiel)](https://macau.uni-kiel.de/servlets/MCRFileNodeServlet/dissertation_derivate_00005979/Dissertation_Lengnick.pdf)
  - [Working-paper version (RePEc)](https://ideas.repec.org/p/zbw/cauewp/201104.html)
  - [newwayland/baseline-economy — the grade-B replication](https://github.com/newwayland/baseline-economy)

---

## Metadata

**Confidence breakdown:**

- **Standard stack: HIGH** — every version resolved live against crates.io; every crate audited
  OK; the whole rand dependency subtree read from source rather than documentation.
- **Architecture (Patterns 1-7): HIGH** — the RNG scheme, the fixed-draw samplers, the overflow
  semantics, the clippy wiring, the config error shapes and `pow_frac_det` were all compiled and
  executed here, and the outputs are quoted. The only design element not exercised is Pattern 5
  (generational IDs), which is a standard pattern with nothing to verify.
- **Pitfalls: HIGH** — all ten were reproduced. The four marked ★ (CORE-03 unsatisfiable,
  `cargo clippy` scope, the type-alias hole, the `Option<T>` hole) each defeat a stated Phase 1
  success criterion and none appear in any existing planning document.
- **`f64` vs `i64` recommendation: MEDIUM** — the mechanics are HIGH (measured), but the
  recommendation is a judgement call a reasonable person could reverse.
- **CORE-11 / Lengnick Table 1: LOW, and explicitly unverified.** All five sources are
  egress-blocked. The parameter values remain grade B. This is the second research pass to be
  blocked; it needs a human, not another agent attempt.

**Research date:** 2026-08-30
**Valid until:** 2026-09-29 (30 days). The stack is stable and the lockfile pins it; the two
things that would age this document are a `rand` 0.11 (which would change the API table again)
and a `sha2` 0.10→0.11 forced migration.
