# Stack Research

**Domain:** Deterministic closed-economy agent-based macro simulation (Lengnick / BAM class) — Rust simulation core + Python offline log-analysis harness
**Researched:** 2026-08-30
**Confidence:** HIGH (versions and behavioural claims verified against crates.io/PyPI APIs, crate tarball source, the local `rustc 1.94.1` std source, and first-hand compile-and-run experiments — not recalled)

> **How to read this document.** Two properties decide whether this build succeeds: **byte-identical reproducibility from a seed** and **money conservation to the cent**. Every recommendation below is justified against one or both. Where a normally-reasonable choice threatens either, it is listed under *What NOT to Use* with the specific failure mode.

---

## Toolchain Baseline

| Item | Value | Note |
|------|-------|------|
| Rust toolchain | **1.94.1** (2026-03-25) — verified present on this machine | Pin it. See §9. |
| Edition | **2024** | Required by `rand` 0.10 (MSRV 1.85). |
| Python | **3.13** | `pandas` 3.0.5 needs ≥3.11, `numpy` 2.5.2 needs ≥3.12. 3.13 satisfies everything. |

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `rand` | **0.10.2** (`default-features = false`, features `["std", "chacha"]`) | The one seeded RNG | Feature-gating away `std_rng`/`sys_rng`/`thread_rng` makes `rand::rng()` **not exist**, so no code path can accidentally draw from OS entropy. Verified by compile failure. |
| `rand::rngs::ChaCha8Rng` | via `chacha20` 0.10.2 | The RNG algorithm | One of `rand`'s documented *"named portable generators … with the additional guarantees of reproducibility."* ChaCha is a specified stream cipher, so the algorithm cannot silently change. |
| `serde` | **1.0.229** (feature `derive`) | Config + log (de)serialisation | Derive emits struct fields in **declaration order** — a deterministic wire format for free. |
| `toml` | **1.1.4+spec-1.1.0** | Config file format | Serde-native, `Value::Integer` is `i64` (money in cents parses natively, no float round-trip), human-diffable, no layering machinery you must reason about. |
| `csv` | **1.4.0** | Per-tick time series → `ticks.csv` | Fixed schema, one header line, 3,650 rows. Numbers via `itoa` + `ryu` (shortest round-trip) ⇒ byte-identical output. `\n` terminator by default, not platform CRLF. One-line read in pandas. |
| `serde_json` | **1.0.151** | Per-event stream → `events.jsonl` | Heterogeneous event payloads + provenance blobs + additive schema evolution. Numbers via `itoa` + `zmij` (shortest round-trip), map keys `BTreeMap`-sorted ⇒ byte-identical output. |
| `clap` | **4.6.6** (feature `derive`) | `--config`, `--seed`, `--out` | Only three flags. Everything else must live in the TOML. |
| **Money newtype** | (no crate) | `struct Money(i64)` in cents | See §5. No crate is warranted and every candidate crate is worse. |
| `pandas` | **3.0.5** | Analysis dataframes | 3,650 rows — there is no performance argument, only an ergonomics one, and `statsmodels` consumes/returns pandas natively. |
| `statsmodels` | **0.15.0** | `acf` for output autocorrelation | The literature convention; gives the biased estimator + confidence bands in one call. |
| `matplotlib` | **3.11.1** | Diagnostic charts | Stdlib of plotting. Nothing else needed. |

### Supporting Libraries — Rust

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `indexmap` | **2.14.1** | `IndexMap`/`IndexSet` with deterministic insertion order | Only if you need map semantics on a non-`Ord` key. Prefer `BTreeMap`, prefer a `Vec` indexed by ID most of all. |
| `thiserror` | **2.0.20** | Typed errors in the lib (`InvariantViolation`, `ConfigError`, `MoneyOverflow`) | The invariant halt needs a structured error carrying tick, agent id and transaction — not a string. |
| `anyhow` | **1.0.104** | Error plumbing in `main.rs` only | Never in `src/lib.rs`. |
| `proptest` | **1.11.0** | Property tests for money helpers, price rule, market matching | See §7. |
| `insta` | **1.48.0** (features `["json"]`) | Snapshot the first N ticks of `ticks.csv` and `events.jsonl` | Locks behaviour so an accidental change to a rule shows up as a reviewable diff. |
| `assert_cmd` | **2.2.2** | End-to-end golden-run test of the built binary | Proves determinism at the artefact level, not just in-process. |
| `sha2` | 0.10.x | Hash config bytes and log bytes | Used by the determinism test and by `run_meta.json`. |

### Supporting Libraries — Python

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `numpy` | **2.5.2** | Gini coefficient, array maths | Hand-roll Gini in ~8 lines. Do not add an inequality package. |
| `pytest` | **9.1.1** | Runs the acceptance harness as pass/fail tests | Section-7 criteria become `assert`s, not eyeballing. |
| `uv` | **0.12.7** | Python env + lockfile | Fast, single binary, produces `uv.lock` — the Python half of reproducibility. |
| `ruff` | **0.16.5** | Lint/format | Optional. One tool, no config. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `rust-toolchain.toml` | Pins the exact rustc version | **Load-bearing for determinism**, not hygiene. See §9. |
| `Cargo.lock` (committed) | Pins exact crate versions | A minor bump of `rand` can legally change distribution algorithms. The lockfile is part of the reproducibility contract. |
| `cargo insta review` | Accept/reject snapshot changes | Makes "did the economy change?" a deliberate human decision. |
| `.cargo/config.toml` | **Must not** set `target-cpu=native` | See §6. |

---

## 1. Seeded RNG

### Recommendation

```toml
[dependencies]
rand = { version = "0.10.2", default-features = false, features = ["std", "chacha"] }
```

```rust
use rand::rngs::ChaCha8Rng;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{Rng, RngExt, SeedableRng};   // note: BOTH Rng and RngExt

let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed);
```

**Confidence: HIGH** — compiled and executed on rustc 1.94.1. Identical output across repeated runs and across debug/release:

```
[12578764544318200737, 17529487244874322312, 7886285670807131020] range=34
f=0.7371560746401922 shuffle=[4, 2, 5, 3, 1] pick=Some(20) sample=[1, 8, 3]
```

### The reproducibility guarantee, precisely

`rand` 0.10.2's own source documents a two-tier split. Verbatim from `src/rngs/mod.rs`:

> **Standard generators.** These use selected best-in-class algorithms. They are deterministic but **not portable: the algorithms may be changed in any release and may be platform-dependent.**
>
> **Named portable generators.** These are similar to the standard generators, but **with the additional guarantees of reproducibility**: `Xoshiro256PlusPlus`, `Xoshiro128PlusPlus`, `ChaCha8Rng`, `ChaCha12Rng` and `ChaCha20Rng`.

And from `src/rngs/std.rs` on `StdRng`:

> Non-portable: **any future library version may replace the algorithm** and results may be platform-dependent. (For a portable version, use the `chacha20` crate directly.) … note that, even with a fixed seed, **output is not portable**.

**This is the whole answer to the question.** `StdRng` and `SmallRng` are the exact hazard described in the brief — an RNG whose output changes on a version bump. They are explicitly disclaimed. `ChaCha8Rng` is explicitly guaranteed.

### The crate-selection landscape as of 2026-08

| Crate | Current | Verdict |
|-------|---------|---------|
| `rand` 0.10.2 + `chacha` feature | 2026-07-02 | **Use this.** |
| `chacha20` 0.10.2 | 2026-08-27 | The actual implementation. Depending on it directly also works and is one fewer layer, but you lose `random_range`/`shuffle`/`choose`, which you want. |
| `rand_chacha` 0.10.0 | 2026-02-02 | Still published and correct, but its own README now says it *"was formerly the implementation behind `rand::rngs::StdRng`"*. **rand 0.10.0 replaced its `rand_chacha` dependency with `chacha20`** (changelog #1642: *"This changes the implementation behind `StdRng`, but the output remains the same"*). Using `rand_chacha` in a new 2026 build is swimming against the ecosystem. |
| `rand_pcg` 0.10.2 | 2026-04-11 | Portable and value-stable, tiny state. Legitimate but weaker statistically than ChaCha8 and buys nothing at 200 agents. |
| `fastrand` 2.5.0 | 2026-07-19 | **Do not use.** Its whole selling point is a small fast Wyrand-family generator with no cross-version output-stability contract. It is designed for "I need a random number", not "I need this number again in 2028". |

### rand 0.10 API breakage you will hit

`rand` 0.10 (2026-02-08) is a substantial break from the 0.9 API most material describes. Plans must use the new names:

| Old (0.8/0.9) | New (0.10) |
|---|---|
| `Rng` (extension trait) | **`RngExt`** |
| `RngCore` | **`Rng`** (moved up from `rand_core`) |
| `rng.gen()` / `gen_range()` | `rng.random()` / `rng.random_range()` |
| `OsRng`, `rand::thread_rng()` | `SysRng`, `rand::rng()` — *neither exists under our feature set, by design* |
| `choose_multiple` / `choose_multiple_weighted` | `sample` / `sample_weighted` |
| `SmallRng`, `ReseedingRng`, feature `small_rng` | Removed / not portable |

**Practical note:** you need `use rand::{Rng, RngExt}` — `Rng` supplies `next_u64`, `RngExt` supplies `random`/`random_range`. Importing only one gives a confusing `no method named ...` error.

### Determinism rules for RNG use

1. **Exactly one `ChaCha8Rng`, threaded through the tick loop by `&mut`.** Never construct a second RNG, never `clone()` one into a subsystem — the moment two generators exist, the number of draws each takes becomes an implicit ordering dependency.
2. **`rand`'s `seq` module samples `usize` indices as `u32` where possible** specifically to make results identical on 32- and 64-bit targets (`src/seq/mod.rs`). Use `IndexedRandom::sample` / `SliceRandom::shuffle` rather than hand-rolling index draws, and you inherit that portability.
3. **Distributions are only "typically expected to be portable"** (`Distribution` trait docs) — they are not covered by the named-generator guarantee across major versions. This is why `Cargo.lock` is committed. If you want cross-version stability for a specific draw, build it from `next_u64()` yourself.
4. **Record the seed in `run_meta.json` every run**, and accept `--seed` as a CLI override of the config value so a failing run can be replayed by copy-paste.

---

## 2. Determinism Hazards in the Rust Standard Library

**Confidence: HIGH** — read from the local `rustc 1.94.1` std source.

### `HashMap` / `HashSet` — the primary hazard

`std::collections::HashMap` docs state it is randomly seeded from *"a secure source of randomness provided by the host"*, and crucially:

> **each `HashMap` instance uses a different seed**

So iteration order varies not only between runs but **between two maps in the same run**. Any `for (k, v) in map` that influences agent behaviour destroys reproducibility. The same applies to `HashSet`.

**What to use instead, in priority order:**

| Situation | Use | Why |
|-----------|-----|-----|
| Agent state keyed by dense integer ID (the 95% case here) | **`Vec<Household>` / `Vec<Firm>`, ID = index** | Already mandated by the brief. Iteration order is index order. No hashing at all. |
| Sparse relation, e.g. firm → owning household | **`Vec<HouseholdId>` indexed by `FirmId`**, or `BTreeMap<FirmId, HouseholdId>` | The brief requires ownership to be a *relation*; a `Vec<(FirmId, HouseholdId)>` edge list sorted by key is the most future-proof and the most deterministic. |
| Genuinely need a map with `Ord` keys | **`BTreeMap` / `BTreeSet`** | Iteration is sorted key order. Deterministic by construction, no extra dependency. |
| Need a map with insertion-order iteration, or non-`Ord` keys | **`indexmap` 2.14.1** (`IndexMap`, `IndexSet`) | Deterministic insertion order. Note: `IndexMap::swap_remove` changes order — use `shift_remove` if order matters after removal. |
| Must use `HashMap` for lookup performance only | `HashMap<K, V, BuildHasherDefault<DefaultHasher>>` **and never iterate it** | Fixed hasher removes run-to-run variance but order is still arbitrary and may change with a std upgrade. Lookup-only is safe; iteration is not. |
| Need to iterate any hash map | Collect keys into a `Vec`, `sort_unstable()`, iterate that | The explicit-sort escape hatch. |

**Enforce it with a lint**, not vigilance:

```rust
// src/lib.rs
#![deny(clippy::disallowed_types)]
```
```toml
# clippy.toml
disallowed-types = [
  { path = "std::collections::HashMap", reason = "nondeterministic iteration order; use BTreeMap, IndexMap or a Vec indexed by ID" },
  { path = "std::collections::HashSet", reason = "nondeterministic iteration order; use BTreeSet or IndexSet" },
]
```

### Sorting — the subtle hazard

`core::slice::sort` contains **no entropy source** (verified by grep), so `sort_unstable` is deterministic for a given input *and a given toolchain*. But the relative order of **equal** elements is unspecified and may change when you upgrade rustc.

This bites directly: households sort sampled firms by price and buy cheapest-first. Two firms at the same price is common. If tie order is unspecified, a toolchain bump silently reallocates demand and the whole trajectory diverges.

**Rule: always sort by a total key that ends in the agent ID.**

```rust
candidates.sort_unstable_by_key(|&f| (firms[f].price, f));   // price, then FirmId — total order
```

With a total key, stability is irrelevant and the sort is deterministic forever. Do this for firm price ordering, wage/vacancy ordering, and any ranking that feeds a decision.

### Everything else in std to avoid on the behaviour path

| Hazard | Why | Instead |
|--------|-----|---------|
| `SystemTime::now()`, `Instant::now()` | Obvious, but easy to sneak into a log record | Tick number is the only clock. Wall-clock time may appear **only** in `run_meta.json`, which is excluded from the determinism diff. |
| Any thread / `rayon` / `std::thread::spawn` | Interleaving is nondeterministic | Single-threaded, as the brief requires. |
| Pointer addresses, `{:p}`, `ptr as usize` | ASLR | Never format a pointer into a log. |
| `f64` `Hash`/ordering via `partial_cmp().unwrap()` | Panics on NaN; also `sort_by` with a non-total comparator is UB-adjacent | `f64::total_cmp` — but better: never sort by a float at all. |
| `std::env::vars()` read by the sim | Invisible input that is not in the committed config | All input comes from the config file + `--seed`. |
| `HashMap` iteration inside `serde` (`serde_json::Value::Object` without `preserve_order`) | Actually safe — `serde_json`'s `Map` is a `BTreeMap` by default | Fine. Do not enable `preserve_order` unless you want insertion order; either is deterministic. |
| `DefaultHasher`'s hash values across Rust versions | Not stable across releases | Never persist a hash of a Rust value; persist a hash of *bytes* (`sha2` over the file). |

---

## 3. Structured Logging to Disk

### Recommendation: two files, two formats

| Stream | Format | Crate | File |
|--------|--------|-------|------|
| Per-tick time series (~10 numeric fields × 3,650 rows) | **CSV** | `csv` 1.4.0 + `serde` | `runs/<id>/ticks.csv` |
| Per-event stream (bankruptcy, hire, fire, dividend, + provenance) | **JSONL** (newline-delimited JSON) | `serde_json` 1.0.151 | `runs/<id>/events.jsonl` |
| Run metadata (seed, config hash, versions, wall-clock) | JSON | `serde_json` | `runs/<id>/run_meta.json` — **excluded from the determinism diff** |

**Confidence: HIGH** on the format choice and byte-determinism; **HIGH** on the crate versions.

### Why these, on the four axes asked

| | **CSV (`csv` 1.4.0)** | **JSONL (`serde_json`)** | **Parquet/Arrow (59.2.0)** |
|---|---|---|---|
| **Python-side read** | `pd.read_csv(p)` — one line, correct dtypes, `int64` for cents | `pd.read_json(p, lines=True)` — one line; heterogeneous fields become NaN-padded columns, which is exactly right for an event stream | `pd.read_parquet(p)` — one line, but requires `pyarrow` (a ~100 MB wheel) for a 3,650-row file |
| **Byte-identical across runs** | **Yes.** Numbers via `itoa`/`ryu`, fixed field order from serde derive, `\n` terminator (not platform CRLF) | **Yes.** Numbers via `itoa`/`zmij`, struct field order from derive, map keys `BTreeMap`-sorted | **Fragile.** Embeds a `created_by` writer-version string, and page/dictionary layout plus compression output are writer-version dependent. Byte-stable only if you freeze `parquet` exactly — and you cannot `diff` the result to find *where* it broke |
| **Append performance** | Trivial. `csv::Writer` over a `BufWriter`; 3,650 rows is microseconds | Trivial. `serde_json::to_writer` + `b"\n"` per event | Poor fit. Parquet is columnar and batch-oriented; appending means buffering row groups in memory and finalising at close |
| **Schema evolution** | Poor — adding a column changes every row and breaks column-position readers. **Acceptable here because the tick schema is fixed and small** | **Excellent** — add a field, old readers ignore it; `#[serde(default)]` on the read side handles missing fields; different event variants coexist in one file | Good, but at a cost you have no reason to pay |
| **Diffability / grep** | `diff a/ticks.csv b/ticks.csv` points at the exact tick | `grep '"firm_id":7' events.jsonl` reconstructs one agent's history from a shell | Opaque binary. **This alone disqualifies it**, given the brief says logs are diffed to prove determinism |

**Verdict: Parquet/Arrow is the wrong tool here and should be explicitly rejected.** `arrow` 59.2.0 pulls a large dependency tree, produces bytes you cannot eyeball, and its only advantages (columnar scan speed, compression) are irrelevant at 3,650 rows × 10 columns — a file that is roughly 300 KB as plain CSV.

### Event stream shape

Use an **internally tagged enum** so every line is self-describing and one file holds all event kinds:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Hire { tick: u32, firm: FirmId, household: HouseholdId, wage_cents: i64,
           provenance: HireProvenance },
    Fire { tick: u32, firm: FirmId, household: HouseholdId, reason: FireReason },
    Bankruptcy { tick: u32, firm: FirmId, owner: HouseholdId, residual_cash_cents: i64,
                 workers_released: Vec<HouseholdId> },
    Dividend { tick: u32, firm: FirmId, owner: HouseholdId, amount_cents: i64,
               buffer_cents: i64 },
    PriceChange { tick: u32, firm: FirmId, from_cents: i64, to_cents: i64,
                  provenance: PriceProvenance },
}
```

`tick` first in every variant makes `sort`/`grep` on the raw file natural, and satisfies "sufficient to reconstruct any agent's history without re-running." The `provenance` structs satisfy the brief's forward-compatibility requirement that decisions carry their inputs from the first tick.

### Determinism rules for logging

1. **Money is logged as integer cents in fields named `*_cents`.** Never write `"12.34"`. If you write a decimal string, pandas reads it as `float64` and the conservation audit degrades from an exact integer equality to a tolerance comparison — which is precisely the failure the brief is guarding against.
2. **No wall-clock timestamps, no durations, no hostname, no path, no PID in `ticks.csv` or `events.jsonl`.** Those go in `run_meta.json`, which is not part of the determinism diff.
3. **Floats are written at full round-trip precision.** Serde's default `f64` output (`ryu`/`zmij`) is shortest-round-trip and lossless. Do **not** apply `{:.4}` formatting to `expected_demand` in the log — you would be throwing away the bits the diff is meant to compare.
4. Wrap the file in `BufWriter` and `flush()` explicitly before exit. An unflushed tail is a phantom determinism failure.
5. The determinism test compares `sha256(ticks.csv) ++ sha256(events.jsonl)` between two runs of the same seed and config.

### What NOT to use for logging

| Avoid | Why | Instead |
|-------|-----|---------|
| `tracing` + `tracing-subscriber` JSON layer **for the data log** | Its JSON output carries timestamps, thread ids, span ids and level — **all nondeterministic or noise**. It also reorders fields via a visitor. It cannot produce a byte-identical file. | `serde_json` writing your own `Event` enum. `tracing` is fine on **stderr for human diagnostics only** — and even then, disable the timestamp for clean test output. |
| `log` + `env_logger` for the data log | Same problem, plus it is string-formatted, not machine-readable | Same |
| `bincode` / `postcard` / any binary serde format | Compact, but not diffable, not greppable, and needs a Rust decoder to inspect — the Python side would need a schema port | CSV + JSONL |
| One combined log file | Mixing a fixed 10-column series with variable-shape events forces the tick series into JSON, tripling its size and making `pd.read_csv` unusable | Two files |

---

## 4. Configuration

### Recommendation: `serde` 1.0.229 + `toml` 1.1.4 — nothing more

**Confidence: HIGH**

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]      // typo in a parameter name = hard error
pub struct Config {
    pub seed: u64,
    pub ticks: u32,
    pub burn_in: u32,
    pub households: HouseholdParams,
    pub firms: FirmParams,
    pub labour_market: LabourParams,
    pub goods_market: GoodsParams,
    pub goods: Vec<GoodDef>,        // "goods are data, not code"
}
```

```rust
let text = std::fs::read_to_string(&cli.config)?;
let cfg: Config = toml::from_str(&text)?;
```

### The three rules that make this the right choice

1. **`#[serde(deny_unknown_fields)]` on every struct.** A misspelled parameter is a startup error, not a silently ignored line. Without this, `lamda = 0.25` sits in the file doing nothing while you tune the wrong knob for an hour.
2. **No `#[serde(default)]` anywhere.** Every parameter must be present in the file. The brief says *"Every parameter exposed in a config file; none hardcoded in logic"* — a serde default **is** a hardcoded parameter, just hidden in a derive attribute. Ship `config/baseline.toml` containing every field with its Lengnick/BAM value, and let that file be the single source of truth. Missing field ⇒ deserialisation error ⇒ you find out immediately.
3. **Copy the config into the run directory and hash it.** `run_meta.json` records `{ seed, config_sha256, config_path, rustc_version, sim_version }`, and the exact config bytes are copied to `runs/<id>/config.toml`. This upgrades "same seed ⇒ same log" into the checkable claim "same seed **and same config** ⇒ same log", which is the claim that is actually true.

### Defaults plus overrides

You need almost no override machinery. The complete set:

- `--config <path>` (required) — which parameter file
- `--seed <u64>` (optional) — overrides `config.seed`, recorded in `run_meta.json` as the effective seed
- `--out <dir>` (optional) — where logs go; affects no behaviour

That is it, via `clap` 4.6.6 derive. For a parameter sweep, generate N TOML files from a script rather than adding CLI overrides for every knob — the generated file is then a reproducible artefact, whereas a shell one-liner is not.

### Alternatives and why not

| Option | Current | Verdict |
|--------|---------|---------|
| `serde` + `toml` | 1.0.229 / 1.1.4 | **Use this.** Zero layering, zero surprise, config file is the whole input. |
| `config` | 0.15.25 (2026-06-26, actively maintained) | Competent and current, but it exists to **layer** file + env + defaults + remote sources. Every layer is an invisible input that can differ between your machine and CI, which is exactly the class of bug that makes a run irreproducible. Its `Value` coercion will also happily hand you a float where you declared an int. **Reject on determinism grounds, not quality grounds.** |
| `figment` | 0.10.19 — **last released 2024-05-17**, roughly two years stale, and a community fork `figment2` exists | Its headline feature (provenance-tracked config errors) is genuinely nice, but the maintenance signal is poor and the layering objection above applies equally. **Do not use.** |
| Env-var overrides (12-factor style) | — | **Actively harmful here.** An env var is an input that is not in the committed config and not in the log; a run configured that way cannot be reproduced from the repository. |
| JSON / YAML config | — | JSON has no comments, and you want comments explaining every parameter's provenance ("λ = 0.25, Lengnick 2013 §4"). YAML adds a parser dependency and the Norway problem. TOML is Cargo's own format and reads well for flat parameter tables. |

---

## 5. Integer Money

### Recommendation: a newtype over `i64`. No crate.

**Confidence: HIGH**

**Range check.** `i64` spans ±9.22 × 10¹⁸ cents ≈ ±$9.2 × 10¹⁶. The simulated economy has 200 households and 20 firms; even at $1M each the total is ~10¹⁰ cents. You have nine orders of magnitude of headroom. (`i32` would give only ±$21.5 million *in cents* — ±$21,474.83 — which is far too tight. `i64` is the right width, `i128` is unnecessary.)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
         Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(i64);   // minor units (cents), private field
```

### Overflow strategy: belt **and** braces — both, not either

**Verified first-hand on cargo 1.94.1.** A default `[profile.release]` build silently wrapped `i64::MAX - 1 + 6` to `-9223372036854775804`. Adding `overflow-checks = true` to `[profile.release]` made the identical program panic with `attempt to add with overflow`. Cargo's defaults are `overflow-checks = true` for `dev`, **`false` for `release`**.

Silent wraparound would turn a conservation violation into a *plausible-looking negative balance* — the exact failure mode the brief calls out. So do both:

**Brace 1 — the profile flag (a backstop):**
```toml
[profile.release]
overflow-checks = true
debug-assertions = true      # keeps debug_assert! live in the shipped binary
```
At 200 agents × 3,650 ticks the cost is unmeasurable. This project has no performance budget to protect.

**Brace 2 — checked arithmetic inside the newtype (the contract):**
```rust
impl std::ops::Add for Money {
    type Output = Money;
    #[track_caller]
    fn add(self, rhs: Money) -> Money {
        Money(self.0.checked_add(rhs.0).expect("money overflow in Money::add"))
    }
}
// same for Sub, AddAssign, SubAssign
```
Implementing `Add`/`Sub` in terms of `checked_*` means ordinary `a + b` panics on overflow **regardless of build profile** — so a stray `cargo build --release` by a future contributor who edited the profile cannot reintroduce wraparound. The profile flag catches raw `i64` arithmetic that escapes the newtype; the newtype catches everything else.

Also provide, for cases where a caller must handle the failure rather than abort:
```rust
impl Money {
    pub fn checked_sub(self, rhs: Money) -> Option<Money> { ... }
    pub fn checked_mul_qty(self, qty: i64) -> Option<Money> { ... }   // price × quantity
}
```

### The other money hazards the newtype must close

| Hazard | Guard |
|--------|-------|
| **Money created from nothing** | Private field + no public `From<i64>` on the hot path. Construction only via `Money::from_cents(i64)`, and that function is used **only** in config parsing and initial endowment. Everything thereafter moves existing money. |
| **Integer division loses cents** | Dividends and pro-rata splits truncate. `1000 / 3 = 333` three times = 999; one cent has been destroyed. Provide `fn split(self, n: u32) -> Vec<Money>` that distributes the remainder deterministically (first `r` recipients get one extra cent, by ascending ID). **This is a conservation bug that will otherwise show up as slow, mysterious money leakage over 3,650 ticks.** |
| **`Sum` without checks** | Do not `impl Sum for Money` via `fold(0, +)` on the raw `i64`. Route it through the checked `Add`. |
| **Float ever touching money** | Deliberately do **not** implement `From<f64>`, `Into<f64>`, `Mul<f64>`, or `Display` as a decimal on `Money`. If a call site needs `expected_demand × price`, it must go through an explicit, named, single rounding function (§6). |
| **Negative balances** | `Money` is signed so that intermediate deltas and residuals can be negative, but the tick invariant asserts every *balance* is `>= Money::ZERO`. Do not use `u64` — you would lose the ability to represent a delta and would get a wrap on the first underflow. |

### Why no crate

| Crate | Why not |
|-------|---------|
| `rust_decimal` 1.42.1 | A 96-bit decimal with a runtime scale, built for *fractional* currency amounts and financial rounding modes. Here every amount is already an exact integer number of cents. It is slower, larger, has a mutable scale you must police, and — worst — it makes float-ish operations available and idiomatic. **It solves a problem you do not have and reopens one you closed.** |
| `rusty-money` / `money2` / similar | Multi-currency, formatting, exchange rates. One currency, no formatting requirements. Dead weight. |
| `fixed` / `fixed-point` crates | Binary fixed-point. Wrong representation for cents (which are a decimal minor unit) and adds a conversion boundary for nothing. |

A 60-line newtype gives you stronger guarantees than any of them, and the type name in every signature documents the domain.

---

## 6. Fixed-Point / Float Boundary

**Confidence: HIGH** — verified against `rustc 1.94.1` std source.

### Direct answer: `expected_demand` as `f64` does **not** threaten byte-identical reproducibility, provided you restrict which operations touch it.

The rule from `expected_demand += λ * (last_sales - expected_demand)` uses only `+`, `-`, `*`. Rust's std docs guarantee these are IEEE-754 correctly rounded — a single, uniquely determined result for a given pair of inputs. Same binary, same platform, repeated runs: **bit-identical, always**. Rust has no `-ffast-math` on stable, does not permit reassociation of float expressions, and does not contract `a*b + c` into a fused multiply-add. **No crate and no compiler flag is needed.**

### The 31 functions that would break it

`rustc 1.94.1`'s std source attaches this verbatim disclaimer to exactly 31 `f64` methods:

> **# Unspecified precision**
>
> The precision of this function is non-deterministic. This means it varies by platform, Rust version, and **can even differ within the same execution from one invocation to the next.**

The full list — these are **banned on the behaviour path**:

```
powi  powf  exp  exp2  ln  log  log2  log10  abs_sub  cbrt  hypot
sin  cos  tan  asin  acos  atan  atan2  sin_cos
exp_m1  ln_1p  sinh  cosh  tanh  asinh  acosh  atanh
gamma  ln_gamma  erf  erfc
```
plus `to_degrees` / `to_radians` in `core`.

Note the phrase *"within the same execution from one invocation to the next"* — this is stronger than a portability caveat. A single `powf` in a smoothing rule can break same-binary reproducibility. If the model ever wants geometric decay, write `x = x * rate` iteratively rather than `x = x0 * rate.powi(n)`.

**Safe and correctly rounded** (not in that list): `+ - * / %`, `sqrt`, `mul_add`, `abs`, `copysign`, `floor`, `ceil`, `round`, `trunc`, `rem_euclid`, comparisons.

### The float/money boundary — the design that keeps them apart

```
┌─ Integer domain (Money, i64 cents; quantities, i64 units) ───────────┐
│  All balances, prices, wages, transfers, inventories.                │
│  Conservation invariants live entirely here and are EXACT.           │
└──────────────────────────────────────────────────────────────────────┘
            ▲                                        │
            │  ONE named crossing function           │  i64 → f64 is exact
            │  with explicit rounding                │  for |n| < 2^53
            │                                        ▼
┌─ Float domain (f64) ─────────────────────────────────────────────────┐
│  expected_demand only. Operations restricted to + - * /.             │
│  Never an input to a conservation check.                             │
└──────────────────────────────────────────────────────────────────────┘
```

```rust
/// The ONLY place a float becomes an integer quantity.
/// Half-away-from-zero via f64::round (correctly rounded), then a
/// saturating cast (Rust `as` on float→int saturates, is defined, and is
/// deterministic — no UB, no platform variance).
#[inline]
pub fn demand_to_units(x: f64) -> i64 {
    debug_assert!(x.is_finite(), "expected_demand became {x}");
    x.max(0.0).round() as i64
}
```

Additional rules:

1. **`i64 → f64` is exact** for magnitudes below 2⁵³ ≈ 9.0 × 10¹⁵. Sales quantities are in the hundreds. No precision concern.
2. **Guard against NaN/±∞ explicitly.** NaN *sign and payload are non-deterministic* per std's primitive docs, and NaN poisons comparisons silently. `debug_assert!(x.is_finite())` at the crossing plus an invariant check catches a runaway rule immediately rather than three thousand ticks later.
3. **Never sort by, hash, or use as a map key** an `f64`. If you must order floats, `f64::total_cmp`.
4. **Log `expected_demand` at full round-trip precision** (serde default). Truncating to 4 dp in the log hides exactly the divergence the diff is looking for.
5. **Do not set `-C target-cpu=native`** in `.cargo/config.toml` or `RUSTFLAGS`. It will not reassociate floats (LLVM cannot without fast-math), so same-machine determinism survives — but it changes codegen between machines and destroys the useful property that the *same source* yields the *same log* on a colleague's laptop and in CI. There is no performance reason to want it here.
6. **Consider not needing floats at all.** If `λ` is a config value like `0.25`, `expected_demand` in **milli-units as `i64`** (`ed += (λ_milli * (sales*1000 - ed)) / 1000`) is exact, conserves nothing but is trivially reproducible, and removes the float domain entirely. This is worth 20 minutes of thought during the planning phase — but `f64` restricted to `+ - *` is genuinely safe and is the lower-friction default. **Recommendation: `f64`, with the banned-function list enforced by a clippy `disallowed-methods` entry.**

```toml
# clippy.toml — machine-enforce the banned list
disallowed-methods = [
  { path = "f64::powf", reason = "unspecified precision; non-deterministic across invocations" },
  { path = "f64::powi", reason = "unspecified precision" },
  { path = "f64::exp",  reason = "unspecified precision" },
  { path = "f64::ln",   reason = "unspecified precision" },
  # ... the rest of the 31
]
```

---

## 7. Testing

**Confidence: HIGH** on crate choice and versions; **HIGH** on the invariant pattern (it is the standard stock-flow-consistent accounting discipline the brief already cites via Caiani et al.).

### Property-based testing: `proptest` 1.11.0 — not `quickcheck`

| | `proptest` 1.11.0 (2026-03-24) | `quickcheck` 1.1.0 (2026-02-10) |
|---|---|---|
| Shrinking | **Integrated value-tree shrinking** — shrinks respect the generator's preconditions, so a shrunk counterexample is still a *valid* economy | Type-directed shrinking that routinely produces invalid inputs and then reports a false failure |
| Regression persistence | **`.proptest-regressions` file, committed to git.** A counterexample found once is replayed on every future run, forever | None. A rare failure found in CI is lost |
| Composing generators | `prop_compose!`, `Strategy` combinators — natural for "generate a *plausible* firm" | `Arbitrary` impls only; awkward for constrained domains |
| Activity | Actively developed | Effectively in maintenance |

The regression-persistence point is decisive for this project: a bankruptcy edge case found at seed 8,412,993 becomes a permanent, committed test rather than folklore.

**What to put under `proptest`** (properties, not examples):
- `Money::split(n)` — the parts always sum exactly back to the whole, for all `(amount, n)`.
- `Money` `Add`/`Sub` — never wrap; `a - b + b == a`.
- Price rule — output is never below unit labour cost, for any inventory/buffer/cost triple.
- Goods matching — never allocates more units than a firm's inventory; buyer spend never exceeds budget.
- Labour matching — a household is employed by at most one firm; a firm's headcount equals its employee list length.

### Snapshot testing: `insta` 1.48.0

```toml
insta = { version = "1.48.0", features = ["json"] }
```

- `assert_snapshot!` on the first 50 rows of `ticks.csv` — a text snapshot, reviewed with `cargo insta review`.
- `assert_json_snapshot!` on the first 50 events.
- Purpose: an accidental change to the wage rule shows up as a **reviewable diff of the economy**, not as a silent trajectory shift. This is the closest thing to a regression test for emergent behaviour.

**But snapshots are not the determinism test.** Keep them distinct:

| Test | What it proves | Crate |
|------|----------------|-------|
| `determinism_same_seed_byte_identical` — run 3,650 ticks twice **into two directories**, assert `sha256` of `ticks.csv` and `events.jsonl` match | Reproducibility | `sha2`, std |
| `determinism_across_process_boundary` — invoke the built binary twice, compare files | Reproducibility at the artefact level (catches global state, env leakage, allocator-order effects) | `assert_cmd` 2.2.2 |
| `different_seed_differs` — a different seed produces a *different* log | That the seed is actually wired in (guards against an accidentally-constant RNG) | std |
| `insta` snapshots | Behaviour has not drifted | `insta` |
| `proptest` properties | Invariants hold on adversarial inputs | `proptest` |

### The conservation invariant, checked cheaply every tick

The established pattern is **single mutation point + cheap total recompute**, in that order.

**1. Make zero-sum true by construction.** Every cent that moves goes through one function, and nothing else may write a balance:

```rust
impl Economy {
    /// The ONLY function in the codebase that mutates a cash balance.
    #[track_caller]
    fn transfer(&mut self, from: Account, to: Account, amount: Money, reason: Reason)
        -> Result<(), InvariantViolation>
    {
        if amount < Money::ZERO { return Err(InvariantViolation::NegativeTransfer { .. }); }
        let src = self.balance_mut(from);
        if *src < amount { return Err(InvariantViolation::Overdraft { from, amount, have: *src, .. }); }
        *src = *src - amount;
        *self.balance_mut(to) = *self.balance_mut(to) + amount;
        Ok(())
    }
}
```
Balance fields are private to the module; there is no `pub fn set_cash`. Zero-sum trade then cannot be violated — there is no code that could violate it.

**2. Recompute the total unconditionally every tick as a backstop.**

```rust
fn check_invariants(&self, tick: u32) -> Result<(), InvariantViolation> {
    let total: Money = self.households.iter().map(|h| h.cash)
        .chain(self.firms.iter().map(|f| f.cash))
        .try_fold(Money::ZERO, |a, b| a.checked_add(b))
        .ok_or(InvariantViolation::MoneyOverflow { tick })?;
    if total != self.initial_money_supply {
        return Err(InvariantViolation::MoneyNotConserved {
            tick, expected: self.initial_money_supply, found: total,
            delta: total - self.initial_money_supply,
        });
    }
    // goods conservation, non-negative balances, ... same shape
    Ok(())
}
```

**Cost: 220 `i64` additions per tick, ~800,000 for the whole 10-year run — well under a millisecond in total.** There is no reason to hide this behind `debug_assert!` or a feature flag. Run it in release, every tick, always. A conservation check that is compiled out of the binary you actually run is worth nothing.

**3. Halt with context, not a bare panic.** Return a `thiserror` enum carrying `tick`, the agent ids and the offending transfer, print it, and exit non-zero. `panic!("assertion failed")` at tick 2,847 tells you nothing; `MoneyNotConserved { tick: 2847, delta: Money(-3), .. }` after a `Dividend` event tells you the dividend split is losing remainder cents.

**Explicitly not needed:** `criterion` 0.8.2. There is no performance requirement — a decade must complete "in seconds", which it will by three orders of magnitude. Adding a benchmark harness is scope creep and invites optimisation that trades away clarity.

---

## 8. Python Analysis Stack

**Confidence: HIGH** on versions (PyPI, 2026-08-30); **HIGH** on the pandas-over-polars call for this specific workload.

### Recommendation

```toml
# analysis/pyproject.toml
[project]
requires-python = ">=3.13"
dependencies = [
  "pandas>=3.0.5,<4",
  "numpy>=2.5.2,<3",
  "statsmodels>=0.15.0,<0.16",
  "matplotlib>=3.11.1,<4",
]
[dependency-groups]
dev = ["pytest>=9.1.1", "ruff>=0.16.5"]
```
Managed with `uv` 0.12.7 (`uv sync`, `uv run pytest`), producing a committed `uv.lock`.

### pandas 3.0.5, not polars 1.44.1

Polars is the better library in general and would be the right call at 10⁸ rows. Here:

- The workload is **3,650 rows and maybe a few thousand events**. Every operation is instantaneous in either. There is no performance argument, so the decision reduces to friction.
- **`statsmodels` speaks pandas natively.** `acf`, `adfuller`, `hpfilter` take and return pandas `Series` with labelled output. With polars you insert `.to_numpy()` at every boundary and lose the labels — pure friction for zero gain.
- `pandas` 3.0.5 requires only `numpy` + `python-dateutil`; **`pyarrow` is an optional extra, not a hard dependency** (verified on PyPI). The install is small.
- The brief says *"acceptance harness, not a reusable toolkit."* Choose the boring, maximally-familiar tool.

**pandas 3.0 caveats to plan for** (it is a major release): copy-on-write is now the only behaviour, so chained assignment is gone; the default string dtype is PyArrow-backed where available. Neither matters for numeric CSV reading, but stale StackOverflow answers will assume 1.x/2.x semantics.

### Per-criterion tooling

| Acceptance criterion | Tool | How |
|---|---|---|
| **Conservation audit** | plain pandas, **integer** | `df["total_money_cents"].nunique() == 1`. This is an **exact `int64` equality** — only possible because the Rust side logs cents as integers. Assert `df["total_money_cents"].dtype == "int64"` first; if it came back as `float64`, the sim wrote decimals and the audit is already invalid. |
| **Unemployment band** | pandas | `u = df.loc[burn_in:, "unemployed"] / n_households`; assert `u.mean()` and `u.quantile([0.05, 0.95])` fall in the target band. |
| **Price-level stability** | pandas + numpy | Post-burn-in `price_level`: check no trend explosion (`np.polyfit` slope, or first-vs-last-decile ratio) and bounded coefficient of variation. |
| **Output autocorrelation** | **`statsmodels.tsa.stattools.acf`** | `acf(y, nlags=20, fft=False)` — returns the standard **biased** estimator with confidence bands, which is the ABM-literature convention. See the note below. |
| **Firm-size distribution / inequality** | numpy, hand-rolled Gini (~8 lines) | Sort sizes, `(2*np.arange(1,n+1) - n - 1) @ s / (n * s.sum())`. Plus a log-log rank-size plot. **Do not add an inequality or power-law package** for one formula. |
| **Seed-reproducibility diff** | `hashlib` (stdlib), then pandas to localise | `sha256` the raw bytes of `ticks.csv` and `events.jsonl` from two runs. If they differ, *then* load both with pandas and report the first tick where any column differs — that is a debugging aid, never the pass/fail test. |
| **Charts** | matplotlib only | `matplotlib.use("Agg")`, `fig.savefig(out / "unemployment.png", dpi=120)`. |

### On autocorrelation specifically

Three options, and they do not agree:

| Tool | Estimator | Verdict |
|------|-----------|---------|
| `statsmodels.tsa.stattools.acf(y, nlags=k)` | Biased (divides by `n`), with Bartlett confidence intervals | **Use this.** It is the convention in the macro-ABM literature, gives all lags plus significance bands in one call, and the band is what makes "output is persistent" a falsifiable claim rather than a number. |
| `pandas.Series.autocorr(lag=k)` | Pearson correlation of `y[:-k]` vs `y[k:]` — effectively the unbiased variant | Fine for a quick single lag-1 number. It will **not equal** `acf[k]`. If you report both without saying which, you will confuse yourself. |
| `numpy.correlate` / `np.corrcoef` | Whatever you hand-roll | You must demean, normalise, and choose the bias convention yourself. Easy to get subtly wrong; no upside. |

**Pick `acf`, state the convention in the harness docstring, and use it consistently.**

Optionally, if the criterion is autocorrelation of the *cyclical component* of output (as Lengnick-style papers report), `statsmodels.tsa.filters.hp_filter.hpfilter(y, lamb=...)` first. Deterministic (dense/sparse linear algebra, no randomness). Add it only if section 7 asks for it.

### Harness shape

Make the harness `pytest` tests, not a script:

```
analysis/
  pyproject.toml
  uv.lock
  acceptance/
    conftest.py          # --run-dir fixture; loads ticks.csv + events.jsonl once
    loaders.py           # the only place that knows the log schema
    test_conservation.py
    test_unemployment.py
    test_prices.py
    test_autocorrelation.py
    test_firm_sizes.py
    test_reproducibility.py
  charts.py              # writes PNGs; not a test
```

`uv run pytest analysis/acceptance --run-dir runs/<id>` then *is* the section-7 gate — every criterion is a named pass/fail with a diagnostic message, and CI can run it.

### What NOT to use on the Python side

| Avoid | Why |
|-------|-----|
| `polars` here | See above. Right library, wrong scale — pure friction against `statsmodels`. |
| `pyarrow` / `duckdb` | Only needed if you chose Parquet. You did not. |
| `seaborn`, `plotly` | The brief explicitly excludes a reusable plotting toolkit. matplotlib does six diagnostic charts fine. |
| `scipy` | Nothing here needs it directly; `statsmodels` pulls it transitively anyway. Do not add it as a direct dependency for one function. |
| `jupyter` notebooks as the harness | Non-reproducible cell order, diffs badly, cannot gate CI. Notebooks are fine for exploration; the acceptance criteria must be `pytest`. |
| `maturin` / `PyO3` bindings | Would couple the Rust and Python halves and violate the brief's "Python reads the sim's log files" constraint. The disk boundary is a feature — it forces the log to be complete. |

---

## 9. Project Layout

**Recommendation: a single Cargo crate with `lib.rs` + thin `main.rs`, and a sibling `analysis/` directory. Not a workspace.**

**Confidence: HIGH**

```
Sim/
├── Cargo.toml                 # single [package], no [workspace]
├── Cargo.lock                 # COMMITTED — part of the reproducibility contract
├── rust-toolchain.toml        # pins rustc 1.94.1 — see below
├── clippy.toml                # disallowed-types / disallowed-methods (§2, §6)
├── .cargo/config.toml         # (only if needed) — must NOT set target-cpu
├── src/
│   ├── lib.rs                 # the simulation — everything public for tests
│   ├── main.rs                # ~40 lines: clap, read config, run, write logs
│   ├── money.rs               # Money newtype (§5)
│   ├── ids.rs                 # HouseholdId, FirmId, GoodId newtypes
│   ├── config.rs              # Config structs (§4)
│   ├── economy.rs             # state + the single `transfer` mutation point
│   ├── goods.rs               # goods table + recipes ("goods are data")
│   ├── ownership.rs           # ownership as a relation
│   ├── tick/                  # one module per tick step, in tick order
│   │   ├── mod.rs             #   the fixed step sequence
│   │   ├── planning.rs  labour.rs  production.rs  wages.rs
│   │   └── goods_market.rs  accounting.rs  bankruptcy.rs
│   ├── invariants.rs          # the four checks (§7)
│   └── log/
│       ├── mod.rs             # RunWriter: opens ticks.csv, events.jsonl, run_meta.json
│       ├── tick_row.rs        # the CSV schema struct
│       └── event.rs           # the Event enum (§3)
├── tests/
│   ├── determinism.rs         # byte-identical, in-process and via assert_cmd
│   ├── invariants.rs
│   ├── properties.rs          # proptest
│   └── snapshots/             # insta
├── config/
│   └── baseline.toml          # EVERY parameter, with comments citing sources
├── analysis/                  # Python — entirely separate toolchain
│   ├── pyproject.toml  uv.lock
│   ├── acceptance/  charts.py
├── runs/                      # .gitignore'd sim output
└── .planning/
```

### Why single crate, not a workspace

- A workspace exists to manage **multiple crates**. You have one binary. A workspace adds a virtual manifest, a second `Cargo.toml`, and `-p` flags on every command, in exchange for nothing.
- The Python side is not a Cargo member and never will be — `analysis/` is just a directory with its own `pyproject.toml` and `uv.lock`. Two toolchains coexisting in one repo is completely ordinary and needs no Cargo involvement.
- Split into a workspace *later*, if and when the roadmap's step-3 chart toolkit or a second binary genuinely needs it. Splitting a crate is a mechanical refactor; you lose nothing by deferring.

### Why `lib.rs` + thin `main.rs` — the one layout decision that matters

With only `src/main.rs`, integration tests in `tests/` **cannot import your code at all** (a binary crate exposes nothing). You would be forced to put every test in `#[cfg(test)]` modules inside the binary, and `proptest`/`insta` against the tick loop would be painful. With `src/lib.rs` holding the simulation and `src/main.rs` doing nothing but argument parsing and calling `sim::run(cfg, &mut writer)`:

- `tests/determinism.rs` does `use sim::*` and runs the economy twice in-process.
- `proptest` reaches the price rule and `Money::split` directly.
- `main.rs` stays small enough that "did the CLI break?" is answered by reading 40 lines.

Design `run()` to take a config and a writer, and to return the final state — so the same entry point serves the binary, the tests and any future harness.

### `rust-toolchain.toml` is load-bearing here

```toml
[toolchain]
channel = "1.94.1"
components = ["rustfmt", "clippy"]
```

For most projects this is hygiene. Here it protects a documented guarantee: `sort_unstable`'s tie order among equal elements is unspecified and may change between rustc releases, and `rand`'s distribution algorithms are only *typically* portable across versions. Pinning the toolchain plus committing `Cargo.lock` makes "same seed ⇒ same log" a claim that survives a `cargo update` by a future contributor. Record the rustc version in `run_meta.json` so a divergence is diagnosable rather than mysterious.

### Output directory convention

One directory per run, self-describing:

```
runs/2026-08-30T19-51-02_seed42_a3f9c1/
├── config.toml       # exact bytes of the config used
├── run_meta.json     # seed, config sha256, rustc version, sim version, wall clock
├── ticks.csv         # ← in the determinism diff
└── events.jsonl      # ← in the determinism diff
```

The determinism test compares only `ticks.csv` and `events.jsonl`. `run_meta.json` deliberately carries the nondeterministic-but-useful metadata (timestamp, host) and is excluded. This separation is what lets you keep useful provenance without weakening the byte-identity claim.

---

## Installation

```bash
# Rust
cargo add rand --no-default-features --features std,chacha
cargo add serde --features derive
cargo add toml serde_json csv thiserror anyhow sha2
cargo add clap --features derive
cargo add --dev proptest insta --features json
cargo add --dev assert_cmd
# only if a non-Vec map is genuinely required:
# cargo add indexmap
```

Resulting key pins (verified against crates.io on 2026-08-30):

```toml
[dependencies]
rand        = { version = "0.10.2", default-features = false, features = ["std", "chacha"] }
serde       = { version = "1.0.229", features = ["derive"] }
serde_json  = "1.0.151"
toml        = "1.1.4"
csv         = "1.4.0"
clap        = { version = "4.6.6", features = ["derive"] }
thiserror   = "2.0.20"
anyhow      = "1.0.104"
sha2        = "0.10"

[dev-dependencies]
proptest    = "1.11.0"
insta       = { version = "1.48.0", features = ["json"] }
assert_cmd  = "2.2.2"

[profile.release]
overflow-checks  = true      # verified necessary: default release SILENTLY WRAPS
debug-assertions = true
```

```bash
# Python
cd analysis && uv sync        # pandas 3.0.5, numpy 2.5.2, statsmodels 0.15.0, matplotlib 3.11.1
uv run pytest acceptance --run-dir ../runs/<id>
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `rand` 0.10.2 + `chacha` feature | `chacha20` 0.10.2 directly | If you want the absolute minimum dependency and will hand-write `random_range`/`shuffle`. You lose `rand::seq`'s 32/64-bit index portability — not worth it. |
| `ChaCha8Rng` | `ChaCha20Rng` | If you ever want cryptographic-grade unpredictability. Same portability guarantee, ~3× slower, irrelevant at this scale. `ChaCha12Rng` is the middle. Any of the three is a defensible pick. |
| `ChaCha8Rng` | `rand_pcg` 0.10.2 / `rand_xoshiro` 0.8.1 | Portable and value-stable too, with tiny state. Choose if you need dozens of independent streams. You need one. |
| `serde` + `toml` | `config` 0.15.25 | If the project later genuinely needs env/file layering for deployment. It does not, and layering weakens reproducibility. |
| CSV for tick series | JSONL for tick series | If the tick schema starts changing every phase. It should not — it is 10 fixed numeric fields. Revisit only if you add a second good and the schema goes variable-width. |
| pandas 3.0.5 | polars 1.44.1 | If a later roadmap step scales to 10⁶ agents and log files reach gigabytes. Not this milestone. |
| Single crate | Cargo workspace | When a second binary or a genuinely separate reusable crate appears (roadmap step 3, the chart toolkit). Defer. |
| `f64` for `expected_demand` | `i64` milli-units fixed-point | If you want to eliminate the float domain entirely and are willing to write the scaling by hand. Legitimate and slightly safer; costs readability. |

---

## What NOT to Use

| Avoid | Why (specific failure mode) | Use Instead |
|-------|------------------------------|-------------|
| **`rand::rngs::StdRng`** | rand's own docs: *"Non-portable: any future library version may replace the algorithm … even with a fixed seed, output is not portable."* A `cargo update` silently changes every trajectory. | `rand::rngs::ChaCha8Rng` |
| **`rand::rngs::SmallRng`** | Same disclaimer; the `small_rng` feature was removed in 0.10 entirely. | `ChaCha8Rng` |
| **`fastrand` 2.5.0** | No cross-version output-stability contract. Designed for convenience randomness. | `ChaCha8Rng` |
| **`rand::rng()` / `SysRng`** | OS entropy — the direct antithesis of a seeded run. **Our feature set makes them not compile**, which is the point. | The single threaded `ChaCha8Rng` |
| **`std::collections::HashMap`/`HashSet` (iterated)** | Randomly seeded per-instance; iteration order varies run-to-run *and* map-to-map. Any behaviour derived from it is irreproducible. | `Vec` indexed by ID → `BTreeMap` → `IndexMap` |
| **`sort_unstable_by` on a non-total key** | Tie order among equal prices is unspecified and may change with a rustc upgrade, silently reallocating demand. | `sort_unstable_by_key(|&f| (price, f))` — ID as final tiebreaker |
| **Default `[profile.release]` (no `overflow-checks`)** | **Verified: `i64::MAX - 1 + 6` silently produced `-9223372036854775804`.** A conservation violation would masquerade as a plausible negative balance. | `overflow-checks = true` **and** `checked_add` inside `Money` |
| **`f64` for money** | Drift over 3,650 ticks destroys conservation. Also `0.1 + 0.2 != 0.3`. | `Money(i64)` in cents |
| **`rust_decimal` 1.42.1 for money** | Solves fractional decimal arithmetic — a problem you do not have — and reintroduces float-adjacent operations you deliberately closed off. Slower, larger, mutable scale. | `Money(i64)` newtype |
| **`f64::exp/ln/powf/powi` (+27 more) anywhere on the behaviour path** | std: *"precision is non-deterministic … can even differ within the same execution from one invocation to the next."* Breaks same-binary reproducibility. | `+ - * /` only; iterate rather than exponentiate |
| **`-C target-cpu=native`** | Changes codegen per machine; destroys "same source ⇒ same log across machines". No performance need. | Default codegen |
| **`tracing-subscriber` JSON layer as the data log** | Emits timestamps, thread/span ids; field order via a visitor. Cannot be byte-identical. | `serde_json` + your own `Event` enum. `tracing` on stderr for humans only |
| **Parquet / Arrow 59.2.0 for these logs** | Opaque binary that cannot be `diff`ed or `grep`ed — and the brief's determinism proof *is* a diff. Embeds a writer-version string. Large dep tree for a 300 KB dataset. | CSV + JSONL |
| **Money written as `"12.34"` in the CSV** | pandas reads `float64`; the conservation audit degrades from exact integer equality to a tolerance check — the exact failure the brief guards against. | Integer cents in `*_cents` columns |
| **`figment` 0.10.19** | Last release 2024-05-17 (~2 years stale); a `figment2` fork exists, signalling a maintenance gap. Plus layering weakens reproducibility. | `serde` + `toml` |
| **Env-var config overrides** | An input that is neither in the committed config nor the log; the run cannot be reproduced from the repo. | Generate a TOML file per experiment |
| **`quickcheck` 1.1.0** | Type-directed shrinking yields invalid economies; no regression persistence, so a rare CI failure is lost. | `proptest` 1.11.0 |
| **`criterion` 0.8.2** | No performance requirement exists; invites optimisation that trades away clarity. | Nothing |
| **`Rc<RefCell<…>>` for agents** | Explicitly named in the brief as the signal the design went wrong. Also introduces address-dependent behaviour risk. | `Vec<Household>`, IDs as indices |
| **Jupyter notebooks as the acceptance harness** | Cell-order dependent, diffs badly, cannot gate CI. | `pytest` under `analysis/acceptance/` |
| **`maturin` / PyO3 bindings** | Couples the halves and violates the "Python reads log files from disk" constraint; the disk boundary is what forces the log to be complete. | Two independent toolchains, one repo |

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `rand` 0.10.2 | `rand_core` 0.10.1, `chacha20` 0.10.2 | Verified by resolving a real `Cargo.lock`. |
| `rand` 0.10.2 | rustc ≥ **1.85**, edition 2024 | Local toolchain 1.94.1 satisfies this. |
| `rand` 0.10.x | **NOT** 0.9.x source examples | Large rename set: `Rng`→`RngExt`, `RngCore`→`Rng`, `gen`→`random`, `OsRng`→`SysRng`, `choose_multiple`→`sample`. Plans must not copy 0.9 snippets. |
| `rand_chacha` 0.10.0 | `rand_core` 0.10 | Interoperates, and produces **identical bytes** to `chacha20` (rand changelog #1642). Safe if you already used it; not the choice for new code. |
| `toml` 1.1.4 | `serde` 1.0.229 | `toml` reached 1.0 — do not assume 0.8 API notes still apply. Default features `std, serde, parse, display`. |
| `serde_json` 1.0.151 | `serde` ≥ 1.0.220 | Number formatting moved from `ryu` to `zmij`; both are shortest-round-trip and deterministic. |
| `pandas` 3.0.5 | Python ≥ 3.11; `numpy` ≥ 1.26 | `pyarrow` is an **optional extra**, not required. |
| `numpy` 2.5.2 | Python ≥ **3.12** | This is what forces Python 3.12+; pin 3.13 to be comfortable. |
| `statsmodels` 0.15.0 | `pandas` ≥ 1.4, `numpy` < 3, `scipy` ≥ 1.8 | Compatible with pandas 3.x. Pulls `scipy`, `patsy`, `formulaic` transitively. |

---

## Sources

**Primary / first-hand (HIGH confidence — this is why the overall rating is HIGH):**

- **crates.io API** (`/api/v1/crates/…`, queried 2026-08-30) — every Rust version number, publication date and dependency requirement in this document.
- **Crate source tarballs from `static.crates.io`**, extracted and read directly:
  - `rand-0.10.2` — `src/rngs/mod.rs` (portable vs non-portable generator taxonomy), `src/rngs/std.rs` (StdRng non-portability disclaimer, ChaCha12 backing), `src/seq/mod.rs` (32/64-bit index portability), `src/distr/distribution.rs` (distribution portability caveat), `Cargo.toml` (feature graph), `CHANGELOG.md` (0.10 breaking changes, `rand_chacha`→`chacha20`).
  - `rand_chacha-0.10.0` — README ("formerly the implementation behind `StdRng`"), `Cargo.toml`.
  - `toml-1.1.4+spec-1.1.0` — `src/lib.rs`, feature graph.
- **Local `rustc 1.94.1` std/core source** (via `rustup component add rust-src`):
  - `core/src/num/f64.rs`, `std/src/num/f64.rs` — the verbatim "Unspecified precision … non-deterministic" blurb and the **exact enumeration of the 31 affected methods**.
  - `core/src/primitive_docs.rs` — NaN sign/payload non-determinism, arithmetic vs bitwise op rules.
  - `std/src/collections/hash/map.rs`, `std/src/hash/random.rs` — "each `HashMap` instance uses a different seed".
  - `core/src/slice/sort/**` — confirmed **no entropy source** in std's sort.
- **First-hand compile-and-run experiments** on rustc/cargo 1.94.1:
  - Overflow: default `[profile.release]` wraps (`-9223372036854775804`); `overflow-checks = true` panics.
  - `rand = { version = "0.10.2", default-features = false, features = ["std","chacha"] }` compiles; `ChaCha8Rng::seed_from_u64(42)` gives identical output across repeated runs and across debug/release; `rand::rng()` **fails to compile** under this feature set.
- **PyPI JSON API** (queried 2026-08-30) — all Python versions, `requires_python`, and pandas 3.0.5's dependency list confirming `pyarrow` is an optional extra.

**Secondary (LOW–MEDIUM confidence — used only for cross-checking, superseded above where they disagreed):**

- WebSearch on the Rust config-crate landscape (`figment` vs `config`) — its qualitative conclusion favouring figment was **contradicted by crates.io release dates** (figment 0.10.19 from 2024-05-17 vs config 0.15.25 from 2026-06-26); the crates.io data was preferred.
- WebSearch on Rust float determinism — corroborated the std-source finding on transcendental functions; the std source is the citation used.

**Not reachable from this environment** (noted for transparency): `docs.rs`, `doc.rust-lang.org`, `rust-random.github.io` and `rust-lang.github.io` are blocked by the egress proxy. Every claim that would ordinarily cite them was instead verified against the corresponding **source**, which is the stronger citation.

---

## Open Questions for Later Phases

1. **`f64` vs `i64` milli-units for `expected_demand`.** Both are defensible. `f64` restricted to `+ - *` is provably bit-reproducible for a fixed binary; fixed-point removes the float domain entirely. Decide in the phase that implements the demand-expectation rule, not before.
2. **Remainder policy for dividend and pro-rata splits.** "First `r` recipients by ascending ID get one extra cent" is deterministic and conserving, but it is a small systematic transfer to low-ID households over 3,650 ticks. A rotating-offset variant is fairer and still deterministic. Worth 10 minutes in the accounting phase.
3. **Whether the tick CSV schema is truly fixed.** CSV is the right call if it is. If the goods table makes the tick series variable-width sooner than expected, switch the tick series to JSONL too — the change is mechanical and the Python loader is one line either way.
4. **Snapshot scope.** Snapshotting 50 ticks is cheap and useful; snapshotting all 3,650 makes every deliberate rule change an unreviewable 3,650-line diff. Settle the window when the first `insta` test is written.

---
*Stack research for: deterministic closed-economy agent-based macro simulation (Rust core + Python acceptance harness)*
*Researched: 2026-08-30*
