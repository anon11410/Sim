<!-- GSD:project-start source:PROJECT.md -->

## Project

**Sim — Minimal Closed Economy**

An agent-based macroeconomic simulation. 200 households and 20 firms trade a single good
("food") in a closed economy where money is a fixed pile that only ever changes hands. One
tick is one day; the target run is 10 simulated years.

This first build is a correctness foundation, not a demo. Every later capability — a second
good, capital, banks, government, demographics, a stock market — is built on top of this daily
loop, so the loop has to be right before it is interesting.

**Core Value:** **The daily tick loop must be provably correct and demonstrably alive.** Money conserved to
the cent every tick, runs byte-identically reproducible from a seed, and an economy that
fluctuates rather than pinning or spiralling. If this is wrong, nothing built on it can be
right.

### Constraints

- **Tech stack**: Rust for the simulation — not for speed at this scale, but because porting a
  tuned agent-based model later is brutal: passing tests is not enough, emergent behaviour has
  to be reproduced, and small numeric or ordering differences change the entire trajectory
- **Tech stack**: Python for analysis and charts, reading the sim's log files — nothing about
  plotting or statistics belongs in the Rust binary
- **Numeric**: integer cents everywhere in money — float money drifts, and drift over thousands
  of ticks silently destroys conservation
- **Architecture**: IDs never references — reaching for `Rc<RefCell<...>>` is the signal the
  design went wrong
- **Determinism**: single-threaded, single seeded RNG; byte-identical logs for a given seed are
  a test, not an aspiration
- **Performance**: a 200-agent decade completes in seconds — this is what makes debugging
  possible, and is the reason not to build for scale yet
- **Configuration**: no parameter hardcoded in logic; all expected to need tuning

<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->

## Technology Stack

## Toolchain Baseline

| Item | Value | Note |
|------|-------|------|
| Rust toolchain | **1.94.1** (2026-03-25) — verified present on this machine | Pin it. See §9. |
| Edition | **2024** | Required by `rand` 0.10 (MSRV 1.85). |
| Python | **3.13** | `pandas` 3.0.5 needs ≥3.11, `numpy` 2.5.2 needs ≥3.12. 3.13 satisfies everything. |

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

## 1. Seeded RNG

### Recommendation

### The reproducibility guarantee, precisely

### The crate-selection landscape as of 2026-08

| Crate | Current | Verdict |
|-------|---------|---------|
| `rand` 0.10.2 + `chacha` feature | 2026-07-02 | **Use this.** |
| `chacha20` 0.10.2 | 2026-08-27 | The actual implementation. Depending on it directly also works and is one fewer layer, but you lose `random_range`/`shuffle`/`choose`, which you want. |
| `rand_chacha` 0.10.0 | 2026-02-02 | Still published and correct, but its own README now says it *"was formerly the implementation behind `rand::rngs::StdRng`"*. **rand 0.10.0 replaced its `rand_chacha` dependency with `chacha20`** (changelog #1642: *"This changes the implementation behind `StdRng`, but the output remains the same"*). Using `rand_chacha` in a new 2026 build is swimming against the ecosystem. |
| `rand_pcg` 0.10.2 | 2026-04-11 | Portable and value-stable, tiny state. Legitimate but weaker statistically than ChaCha8 and buys nothing at 200 agents. |
| `fastrand` 2.5.0 | 2026-07-19 | **Do not use.** Its whole selling point is a small fast Wyrand-family generator with no cross-version output-stability contract. It is designed for "I need a random number", not "I need this number again in 2028". |

### rand 0.10 API breakage you will hit

| Old (0.8/0.9) | New (0.10) |
|---|---|
| `Rng` (extension trait) | **`RngExt`** |
| `RngCore` | **`Rng`** (moved up from `rand_core`) |
| `rng.gen()` / `gen_range()` | `rng.random()` / `rng.random_range()` |
| `OsRng`, `rand::thread_rng()` | `SysRng`, `rand::rng()` — *neither exists under our feature set, by design* |
| `choose_multiple` / `choose_multiple_weighted` | `sample` / `sample_weighted` |
| `SmallRng`, `ReseedingRng`, feature `small_rng` | Removed / not portable |

### Determinism rules for RNG use

## 2. Determinism Hazards in the Rust Standard Library

### `HashMap` / `HashSet` — the primary hazard

| Situation | Use | Why |
|-----------|-----|-----|
| Agent state keyed by dense integer ID (the 95% case here) | **`Vec<Household>` / `Vec<Firm>`, ID = index** | Already mandated by the brief. Iteration order is index order. No hashing at all. |
| Sparse relation, e.g. firm → owning household | **`Vec<HouseholdId>` indexed by `FirmId`**, or `BTreeMap<FirmId, HouseholdId>` | The brief requires ownership to be a *relation*; a `Vec<(FirmId, HouseholdId)>` edge list sorted by key is the most future-proof and the most deterministic. |
| Genuinely need a map with `Ord` keys | **`BTreeMap` / `BTreeSet`** | Iteration is sorted key order. Deterministic by construction, no extra dependency. |
| Need a map with insertion-order iteration, or non-`Ord` keys | **`indexmap` 2.14.1** (`IndexMap`, `IndexSet`) | Deterministic insertion order. Note: `IndexMap::swap_remove` changes order — use `shift_remove` if order matters after removal. |
| Must use `HashMap` for lookup performance only | `HashMap<K, V, BuildHasherDefault<DefaultHasher>>` **and never iterate it** | Fixed hasher removes run-to-run variance but order is still arbitrary and may change with a std upgrade. Lookup-only is safe; iteration is not. |
| Need to iterate any hash map | Collect keys into a `Vec`, `sort_unstable()`, iterate that | The explicit-sort escape hatch. |
#![deny(clippy::disallowed_types)]

# clippy.toml

### Sorting — the subtle hazard

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

## 3. Structured Logging to Disk

### Recommendation: two files, two formats

| Stream | Format | Crate | File |
|--------|--------|-------|------|
| Per-tick time series (~10 numeric fields × 3,650 rows) | **CSV** | `csv` 1.4.0 + `serde` | `runs/<id>/ticks.csv` |
| Per-event stream (bankruptcy, hire, fire, dividend, + provenance) | **JSONL** (newline-delimited JSON) | `serde_json` 1.0.151 | `runs/<id>/events.jsonl` |
| Run metadata (seed, config hash, versions, wall-clock) | JSON | `serde_json` | `runs/<id>/run_meta.json` — **excluded from the determinism diff** |

### Why these, on the four axes asked

| | **CSV (`csv` 1.4.0)** | **JSONL (`serde_json`)** | **Parquet/Arrow (59.2.0)** |
|---|---|---|---|
| **Python-side read** | `pd.read_csv(p)` — one line, correct dtypes, `int64` for cents | `pd.read_json(p, lines=True)` — one line; heterogeneous fields become NaN-padded columns, which is exactly right for an event stream | `pd.read_parquet(p)` — one line, but requires `pyarrow` (a ~100 MB wheel) for a 3,650-row file |
| **Byte-identical across runs** | **Yes.** Numbers via `itoa`/`ryu`, fixed field order from serde derive, `\n` terminator (not platform CRLF) | **Yes.** Numbers via `itoa`/`zmij`, struct field order from derive, map keys `BTreeMap`-sorted | **Fragile.** Embeds a `created_by` writer-version string, and page/dictionary layout plus compression output are writer-version dependent. Byte-stable only if you freeze `parquet` exactly — and you cannot `diff` the result to find *where* it broke |
| **Append performance** | Trivial. `csv::Writer` over a `BufWriter`; 3,650 rows is microseconds | Trivial. `serde_json::to_writer` + `b"\n"` per event | Poor fit. Parquet is columnar and batch-oriented; appending means buffering row groups in memory and finalising at close |
| **Schema evolution** | Poor — adding a column changes every row and breaks column-position readers. **Acceptable here because the tick schema is fixed and small** | **Excellent** — add a field, old readers ignore it; `#[serde(default)]` on the read side handles missing fields; different event variants coexist in one file | Good, but at a cost you have no reason to pay |
| **Diffability / grep** | `diff a/ticks.csv b/ticks.csv` points at the exact tick | `grep '"firm_id":7' events.jsonl` reconstructs one agent's history from a shell | Opaque binary. **This alone disqualifies it**, given the brief says logs are diffed to prove determinism |

### Event stream shape

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]

### Determinism rules for logging

### What NOT to use for logging

| Avoid | Why | Instead |
|-------|-----|---------|
| `tracing` + `tracing-subscriber` JSON layer **for the data log** | Its JSON output carries timestamps, thread ids, span ids and level — **all nondeterministic or noise**. It also reorders fields via a visitor. It cannot produce a byte-identical file. | `serde_json` writing your own `Event` enum. `tracing` is fine on **stderr for human diagnostics only** — and even then, disable the timestamp for clean test output. |
| `log` + `env_logger` for the data log | Same problem, plus it is string-formatted, not machine-readable | Same |
| `bincode` / `postcard` / any binary serde format | Compact, but not diffable, not greppable, and needs a Rust decoder to inspect — the Python side would need a schema port | CSV + JSONL |
| One combined log file | Mixing a fixed 10-column series with variable-shape events forces the tick series into JSON, tripling its size and making `pd.read_csv` unusable | Two files |

## 4. Configuration

### Recommendation: `serde` 1.0.229 + `toml` 1.1.4 — nothing more

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]      // typo in a parameter name = hard error

### The three rules that make this the right choice

### Defaults plus overrides

- `--config <path>` (required) — which parameter file
- `--seed <u64>` (optional) — overrides `config.seed`, recorded in `run_meta.json` as the effective seed
- `--out <dir>` (optional) — where logs go; affects no behaviour

### Alternatives and why not

| Option | Current | Verdict |
|--------|---------|---------|
| `serde` + `toml` | 1.0.229 / 1.1.4 | **Use this.** Zero layering, zero surprise, config file is the whole input. |
| `config` | 0.15.25 (2026-06-26, actively maintained) | Competent and current, but it exists to **layer** file + env + defaults + remote sources. Every layer is an invisible input that can differ between your machine and CI, which is exactly the class of bug that makes a run irreproducible. Its `Value` coercion will also happily hand you a float where you declared an int. **Reject on determinism grounds, not quality grounds.** |
| `figment` | 0.10.19 — **last released 2024-05-17**, roughly two years stale, and a community fork `figment2` exists | Its headline feature (provenance-tracked config errors) is genuinely nice, but the maintenance signal is poor and the layering objection above applies equally. **Do not use.** |
| Env-var overrides (12-factor style) | — | **Actively harmful here.** An env var is an input that is not in the committed config and not in the log; a run configured that way cannot be reproduced from the repository. |
| JSON / YAML config | — | JSON has no comments, and you want comments explaining every parameter's provenance ("λ = 0.25, Lengnick 2013 §4"). YAML adds a parser dependency and the Norway problem. TOML is Cargo's own format and reads well for flat parameter tables. |

## 5. Integer Money

### Recommendation: a newtype over `i64`. No crate.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
#[serde(transparent)]

### Overflow strategy: belt **and** braces — both, not either

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

## 6. Fixed-Point / Float Boundary

### Direct answer: `expected_demand` as `f64` does **not** threaten byte-identical reproducibility, provided you restrict which operations touch it.

### The 31 functions that would break it

### The float/money boundary — the design that keeps them apart

#[inline]

# clippy.toml — machine-enforce the banned list

## 7. Testing

### Property-based testing: `proptest` 1.11.0 — not `quickcheck`

| | `proptest` 1.11.0 (2026-03-24) | `quickcheck` 1.1.0 (2026-02-10) |
|---|---|---|
| Shrinking | **Integrated value-tree shrinking** — shrinks respect the generator's preconditions, so a shrunk counterexample is still a *valid* economy | Type-directed shrinking that routinely produces invalid inputs and then reports a false failure |
| Regression persistence | **`.proptest-regressions` file, committed to git.** A counterexample found once is replayed on every future run, forever | None. A rare failure found in CI is lost |
| Composing generators | `prop_compose!`, `Strategy` combinators — natural for "generate a *plausible* firm" | `Arbitrary` impls only; awkward for constrained domains |
| Activity | Actively developed | Effectively in maintenance |

- `Money::split(n)` — the parts always sum exactly back to the whole, for all `(amount, n)`.
- `Money` `Add`/`Sub` — never wrap; `a - b + b == a`.
- Price rule — output is never below unit labour cost, for any inventory/buffer/cost triple.
- Goods matching — never allocates more units than a firm's inventory; buyer spend never exceeds budget.
- Labour matching — a household is employed by at most one firm; a firm's headcount equals its employee list length.

### Snapshot testing: `insta` 1.48.0

- `assert_snapshot!` on the first 50 rows of `ticks.csv` — a text snapshot, reviewed with `cargo insta review`.
- `assert_json_snapshot!` on the first 50 events.
- Purpose: an accidental change to the wage rule shows up as a **reviewable diff of the economy**, not as a silent trajectory shift. This is the closest thing to a regression test for emergent behaviour.

| Test | What it proves | Crate |
|------|----------------|-------|
| `determinism_same_seed_byte_identical` — run 3,650 ticks twice **into two directories**, assert `sha256` of `ticks.csv` and `events.jsonl` match | Reproducibility | `sha2`, std |
| `determinism_across_process_boundary` — invoke the built binary twice, compare files | Reproducibility at the artefact level (catches global state, env leakage, allocator-order effects) | `assert_cmd` 2.2.2 |
| `different_seed_differs` — a different seed produces a *different* log | That the seed is actually wired in (guards against an accidentally-constant RNG) | std |
| `insta` snapshots | Behaviour has not drifted | `insta` |
| `proptest` properties | Invariants hold on adversarial inputs | `proptest` |

### The conservation invariant, checked cheaply every tick

## 8. Python Analysis Stack

### Recommendation

# analysis/pyproject.toml

### pandas 3.0.5, not polars 1.44.1

- The workload is **3,650 rows and maybe a few thousand events**. Every operation is instantaneous in either. There is no performance argument, so the decision reduces to friction.
- **`statsmodels` speaks pandas natively.** `acf`, `adfuller`, `hpfilter` take and return pandas `Series` with labelled output. With polars you insert `.to_numpy()` at every boundary and lose the labels — pure friction for zero gain.
- `pandas` 3.0.5 requires only `numpy` + `python-dateutil`; **`pyarrow` is an optional extra, not a hard dependency** (verified on PyPI). The install is small.
- The brief says *"acceptance harness, not a reusable toolkit."* Choose the boring, maximally-familiar tool.

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

| Tool | Estimator | Verdict |
|------|-----------|---------|
| `statsmodels.tsa.stattools.acf(y, nlags=k)` | Biased (divides by `n`), with Bartlett confidence intervals | **Use this.** It is the convention in the macro-ABM literature, gives all lags plus significance bands in one call, and the band is what makes "output is persistent" a falsifiable claim rather than a number. |
| `pandas.Series.autocorr(lag=k)` | Pearson correlation of `y[:-k]` vs `y[k:]` — effectively the unbiased variant | Fine for a quick single lag-1 number. It will **not equal** `acf[k]`. If you report both without saying which, you will confuse yourself. |
| `numpy.correlate` / `np.corrcoef` | Whatever you hand-roll | You must demean, normalise, and choose the bias convention yourself. Easy to get subtly wrong; no upside. |

### Harness shape

### What NOT to use on the Python side

| Avoid | Why |
|-------|-----|
| `polars` here | See above. Right library, wrong scale — pure friction against `statsmodels`. |
| `pyarrow` / `duckdb` | Only needed if you chose Parquet. You did not. |
| `seaborn`, `plotly` | The brief explicitly excludes a reusable plotting toolkit. matplotlib does six diagnostic charts fine. |
| `scipy` | Nothing here needs it directly; `statsmodels` pulls it transitively anyway. Do not add it as a direct dependency for one function. |
| `jupyter` notebooks as the harness | Non-reproducible cell order, diffs badly, cannot gate CI. Notebooks are fine for exploration; the acceptance criteria must be `pytest`. |
| `maturin` / `PyO3` bindings | Would couple the Rust and Python halves and violate the brief's "Python reads the sim's log files" constraint. The disk boundary is a feature — it forces the log to be complete. |

## 9. Project Layout

### Why single crate, not a workspace

- A workspace exists to manage **multiple crates**. You have one binary. A workspace adds a virtual manifest, a second `Cargo.toml`, and `-p` flags on every command, in exchange for nothing.
- The Python side is not a Cargo member and never will be — `analysis/` is just a directory with its own `pyproject.toml` and `uv.lock`. Two toolchains coexisting in one repo is completely ordinary and needs no Cargo involvement.
- Split into a workspace *later*, if and when the roadmap's step-3 chart toolkit or a second binary genuinely needs it. Splitting a crate is a mechanical refactor; you lose nothing by deferring.

### Why `lib.rs` + thin `main.rs` — the one layout decision that matters

- `tests/determinism.rs` does `use sim::*` and runs the economy twice in-process.
- `proptest` reaches the price rule and `Money::split` directly.
- `main.rs` stays small enough that "did the CLI break?" is answered by reading 40 lines.

### `rust-toolchain.toml` is load-bearing here

### Output directory convention

## Installation

# Rust

# only if a non-Vec map is genuinely required:

# cargo add indexmap

# Python

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

## Sources

- **crates.io API** (`/api/v1/crates/…`, queried 2026-08-30) — every Rust version number, publication date and dependency requirement in this document.
- **Crate source tarballs from `static.crates.io`**, extracted and read directly:
- **Local `rustc 1.94.1` std/core source** (via `rustup component add rust-src`):
- **First-hand compile-and-run experiments** on rustc/cargo 1.94.1:
- **PyPI JSON API** (queried 2026-08-30) — all Python versions, `requires_python`, and pandas 3.0.5's dependency list confirming `pyarrow` is an optional extra.
- WebSearch on the Rust config-crate landscape (`figment` vs `config`) — its qualitative conclusion favouring figment was **contradicted by crates.io release dates** (figment 0.10.19 from 2024-05-17 vs config 0.15.25 from 2026-06-26); the crates.io data was preferred.
- WebSearch on Rust float determinism — corroborated the std-source finding on transcendental functions; the std source is the citation used.

## Open Questions for Later Phases

<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
