# Phase 3: World, Tick Pipeline and Log Seam - Research

**Researched:** 2026-08-31
**Domain:** Deterministic file-format seam (CSV/JSONL/JSON), fixed-order pipeline table, agent world types
**Confidence:** HIGH — every recommendation below was compiled and executed on the pinned
`rustc 1.94.1 (e408947bf 2026-03-25)` against a **copy of this repository** with the real
`books`, `invariants`, `ids`, `rng` and `config` modules, including the designs that failed.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Classified infrastructure by the smart-discuss test: every success criterion is technical
(a test asserts a name sequence, two runs diff byte-identically, a binary exits non-zero),
and no user-facing behaviour is described. No grey area was escalated for a user decision.

**Locked by prior decisions — not reopened here**

- **Tick order is not build order.** Firm planning runs first in the tick but is built ninth.
  This phase builds the *table*, with all nine phases present as no-ops.
- **CSV for the tick series, JSONL for events, JSON for run metadata.** Parquet was rejected
  because the determinism proof *is* a diff, and an opaque binary cannot be diffed or grepped.
- **Money is logged as integer `*_cents` columns.** A decimal string makes pandas infer
  `float64` and silently degrades the Phase 4 conservation audit from exact `int64` equality
  to a tolerance check — the precise failure the brief exists to prevent.
- **`run_meta.json` is excluded from the determinism diff** and is the ONLY place a wall clock
  may appear. `ticks.csv` and `events.jsonl` carry no timestamp, hostname, path or PID.
- **Provenance is a joinable flat table** (tick, agent, decision type, inputs, outcome) — never
  free text. Empty at this phase but present and schema-validated, because provenance added
  retroactively never covers the early history.
- **The liveness gate ships `false`** so criterion 1's empty run passes. Criterion 6 exercises
  the same binary with it overridden to `true`. Phase 6 owns flipping the shipped value.

**The wire-shape stake this phase carries**

`Posting` already has a serialisation shape, decided in Phase 2 wave 2 and rated **costly to
change from Phase 3 onward** — precisely because this is the phase that snapshots it into
`events.jsonl`. Address fields render through `serialize_with` as `"household:12"` /
`"firm:3:0"`. Whatever `schema/schema.json` freezes here is what Phases 5-11 append to.
Get it right rather than quickly.

### Claude's Discretion

Module layout for `world`/`log`, the `Sink` trait's shape (`NullSink`/`VecSink`/`RunWriter`),
the run-directory naming convention, and the two questions the project research left open for
this phase: the `insta` snapshot window size (50 ticks proposed — all 3,650 would make a
deliberate rule change unreviewable), and whether a per-firm panel carries books-derived
columns redundantly alongside behavioural state.

### Deferred Ideas (OUT OF SCOPE)

- Any economic behaviour. Production arrives in Phase 5; nothing here decides a price, wage or
  hire.
- The Python harness itself — Phase 4 — though the schema it consumes is frozen here.
- Flipping `invariants.liveness_enabled` to `true` in the shipped config: Phase 6 criterion 7.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TICK-01 | Fixed `const PHASES` table, brief's phases in order, each completing for all agents before the next | § Pattern 1 — `PHASES` built on Phase 2's `ALL_CHECKS` construction; exhaustive-match order test compiled and mutation-proved |
| TICK-02 | Log schema generated and committed as `schema/schema.json`; drift is a test failure | § Pattern 4 + § Pitfall 2 — **schemars rejected on measured evidence**; hand-rolled emitter derived from the bytes actually written, drift test mutation-proved twice |
| TICK-03 | Per-tick series to `ticks.csv`, money as integer `*_cents` columns | § Pattern 3 + § Pitfall 3/4/5 — csv 1.4.0 terminator, header, `Option`, nesting and float behaviour all measured; pandas 3.0.5 read back at `int64` |
| TICK-04 | Per-event stream to `events.jsonl` covering bankruptcy/hire/fire/dividend, sufficient to reconstruct history | § Pattern 3 + § Pitfall 8 — externally-tagged enum verified; **endowment events required**, see Pitfall 8 |
| TICK-05 | `run_meta.json` carries seed, config hash, toolchain; held separate from diffed logs | § Pattern 5 — layout written and run; toolchain string via `build.rs`, verified |
| TICK-06 | Diffed logs contain no wall-clock, hostname, path or PID | § Pattern 5 + § Pitfall 10 — enforcement test written and run; halt-message check extended from guard 7h |
| TICK-07 | Decision provenance as a joinable flat table, never free text | § Pattern 3 — `ProvenanceRow`; **header must be written eagerly**, see Pitfall 4 |
| TICK-08 | 3,650 empty ticks execute and two runs diff byte-identically | § Measured — 3,650 ticks in **10 ms release / 526 ms debug**, `ticks.csv` 202,974 bytes |
| TICK-09 | Same seed → byte-identical logs, in-process and cross-process | § Pattern 6 — `assert_cmd` cross-process test written and passing; **debug and release bytes also identical** |
| TICK-10 | Different seed → different logs | § Pitfall 1 — **the prescribed design fails this**; measured counterexample and the fix |
</phase_requirements>

---

## Summary

Phase 3 has no economics, so nothing in it can be proved by the model behaving sensibly. Every
criterion is a claim about *bytes on disk* and about *tests that would fail if the claim were
false*. This research therefore did not read documentation about `csv` and `serde_json` — it
built the phase, ran it, and diffed the output, then broke each guarantee in turn to check that
something noticed.

Three findings change the plan rather than merely informing it.

**First, the design the ROADMAP prescribes for criterion 3 does not work.** An activation-order
shuffle plus a per-tick draw-count column was built and run over 3,650 ticks at seeds 42 and 43.
The two runs produced **byte-identical `ticks.csv`**. Consuming the RNG is not sufficient: the
draw count is a *constant* (218 per tick here), so it proves draws happened and says nothing
about the seed. What TICK-10 needs is for a *seed-sensitive value* to reach the log. Adding one
integer column carrying a digest of the tick's activation permutation flipped the same test from
identical to differing at tick 0 — measured both ways.

**Second, `schemars` must not be added.** It is the obvious choice and the project research
names it, but it derives a *second, independent* description of the types and it cannot see
`#[serde(serialize_with)]`. Compiled against a replica of this repo's `Posting`, `serde_json`
writes `"debit":"household:12"` while `schemars` 1.2.2 declares that field an externally-tagged
object `{"Household": integer}`. The generated schema contradicts the bytes for exactly the field
CONTEXT.md calls "the wire-shape stake this phase carries", and a generated-vs-committed drift
test passes forever because the two wrong things agree. A ~90-line emitter that reads the CSV
header `csv::Writer` itself emits and the field order out of the JSON text `serde_json` itself
writes cannot drift from reality, adds no dependency, and reports that field as a string.

**Third, an empty pipeline produces empty artifacts, and empty artifacts break both a downstream
reader and the determinism proof itself.** With the naive design, `provenance.csv` and
`events.jsonl` are both zero bytes. pandas 3.0.5 raises `EmptyDataError` on the former (measured),
and a cross-process hash comparison over two empty files returns `e3b0c442…` twice and passes
vacuously (measured). Both are closed by construction: write the CSV header eagerly (which
requires `has_headers(false)`, because the obvious spelling emits the header **twice** —
measured), and emit the opening endowment as events read from the books' accessors, which is
also exactly the origin row HARN-02's conservation *replay* needs.

**Primary recommendation:** add exactly four crates (`csv`, `serde_json`, and dev-only
`assert_cmd`, `tempfile`); reject `schemars`, `insta` and `predicates`; make the tick log a flat
integer-only CSV with one seed-sensitive digest column; generate the schema from the writers
rather than from a second derive; and write every guard as a mutation-proved negative test in the
Phase 2 style.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tick ordering (`PHASES`) | `src/phases.rs` (pipeline) | — | The array *is* the specification; nothing else may know the order |
| Agent identity + behavioural state | `src/world.rs` | `src/ids.rs` (identities) | `World` owns `Vec<Household>` / `FirmArena<Firm>`; identities already exist |
| Every cent and unit | `src/books.rs` (ledger, Phase 2) | — | LEDG-01: agents hold **no** balance; Phase 3 must not reopen this |
| Invariant checking | `src/invariants.rs` (Phase 2) | `src/phases.rs` (calls it as step 8) | The check is a pipeline phase, not a wrapper |
| Wire format + schema | `src/log.rs` | — | Only module that names `csv` or `serde_json` |
| Disk layout / run directory | `src/log.rs` (`RunWriter`) | `src/main.rs` (path from `--out`) | A path is an operator input, never config-derived (T-1-04) |
| Run metadata / wall clock | `src/main.rs` → `run_meta.json` | `build.rs` (toolchain string) | The single quarantined non-deterministic surface |
| RNG consumption | `src/rng.rs` (Phase 1) | `src/phases.rs` (activation shuffle) | `Purpose::ActivationOrder{Households,Firms}` already exist at discriminants 10/11 |
| CLI surface | `src/main.rs` | — | CORE-08: no simulation logic here |

---

## Project Constraints (from CLAUDE.md)

All verified as still binding, and all satisfied by the recommendations below.

| Directive | How Phase 3 satisfies it | Verified |
|-----------|--------------------------|----------|
| Integer cents everywhere | Every `ticks.csv` column is an integer; `Money` is `#[serde(transparent)]` over `i64` and serialises as a bare integer | Ran: `Money::from_cents(-105)` → JSON `-105` |
| IDs never references; no `Rc<RefCell<…>>` | `World` holds `Vec<Household>` and `FirmArena<Firm>`; `Ctx` holds `&mut World` | Built; `tests/lints.sh` guards 7b/7c stayed green |
| No `HashMap`/`HashSet` on a behaviour path | The log module uses `BTreeMap` for the dtype lookup only | `cargo clippy --all-targets --all-features` clean |
| `#![forbid(unsafe_code)]` | Unchanged in `src/lib.rs` | Built |
| Every parameter in config, **no serde defaults** | Phase 3 adds **no new config leaf** — see § Open Question 1 | — |
| No `-C target-cpu=native`, no `rayon`, no `getrandom` on the behaviour path | `csv`/`serde_json` pull only `itoa`, `ryu`, `zmij`, `memchr`, `csv-core` | Ran `tests/toolchain.sh` after adding them: **OK** |
| `tracing`/`log` never as the data log | `serde_json` writing an owned `Event` enum | Design |
| Single-threaded | No thread, no `rayon` | `cargo tree --edges normal` grep for rayon: no match |
| Do not use `fastrand` | **`insta` pulls `fastrand` and `getrandom`** — one of several reasons to reject it | Ran `cargo add --dev insta`: 17 transitive packages incl. both |

---

## Standard Stack

### Core — add to `[dependencies]`

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `csv` | **1.4.0** | `ticks.csv`, `provenance.csv` | Terminator is hard-coded `\n` with no `cfg(windows)`; integers via `itoa`; header comes from the serde derive in declaration order. All four verified. `[VERIFIED: crates.io API, published 2025-10-17, MSRV 1.73]` |
| `serde_json` | **1.0.151** | `events.jsonl`, the schema emitter's type probe | Struct field order is declaration order; `Value::Object` is a `BTreeMap` unless `preserve_order`. Both verified from source and by running. `[VERIFIED: crates.io API, published 2026-07-20, MSRV 1.71]` |

### Supporting — add to `[dev-dependencies]`

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `assert_cmd` | **2.2.2** | Criteria 2 (cross-process) and 6 (process-level halt) | `Command::cargo_bin("sim")` + `.assert().failure().code(1)`. `[VERIFIED: crates.io API, 2026-05-11, MSRV 1.85 — satisfied by 1.94.1]` |
| `tempfile` | **3.27.0** | Isolated run directories and the one-leaf config override | Already in the lockfile via `proptest`; adding it as a direct dev-dep introduces **no new package**. `[VERIFIED: cargo add, 0 new packages beyond assert_cmd's own]` |

**Exact manifest diff, as produced by `cargo add` and then run:** `[VERIFIED: diff of the built tree]`

```toml
[dependencies]
csv        = "1.4.0"
serde_json = "1.0.151"

[dev-dependencies]
assert_cmd = "2.2.2"
tempfile   = "3.27.0"
```

Adds **7 normal-edge packages** (`csv`, `csv-core`, `itoa`, `memchr`, `ryu`, `serde_json`,
`zmij`) and **8 dev-edge packages**. `tests/toolchain.sh` passes unchanged afterwards, and
`cargo clippy --all-targets --all-features` is warning-free. `[VERIFIED: ran both]`

### Rejected — with the measurement that rejects each

| Proposed | Verdict | Evidence |
|----------|---------|----------|
| `schemars` 1.2.2 | **REJECT** | Compiled a replica of `Posting`: `serde_json` writes `"debit":"household:12"`, schemars declares `{"$ref": "#/$defs/Account"}` → `oneOf [{Household: int}, {Firm: object}]`. It **cannot see `#[serde(serialize_with)]`**, and this project uses it on both address fields. It also emits `properties` **alphabetically**, so it does not record CSV column order — which is the contract for `ticks.csv`. `[VERIFIED: compiled and ran both serialisers side by side]` |
| `insta` 1.48.0 | **REJECT** | `cargo add --dev insta --features json` pulls **17 packages**, including `fastrand` (which `CLAUDE.md` § What NOT to Use names by name) and `getrandom`. The alternative — a committed golden file plus `assert_eq!` — is 8 lines and 0 packages, and matches the repo's existing generated-artifact pattern (`clippy.toml`). `[VERIFIED: ran cargo add and read the package list]` |
| `predicates` 3.1.4 | **REJECT** | `assert_cmd` 2.2.2 depends on it but does **not** re-export it — `predicates::str::contains` fails to compile with `E0433` unless it is a direct dev-dependency. Use `.assert().failure().code(1)` then a plain `assert!(stderr.contains(…))`, which also produces a better message. `[VERIFIED: compile error reproduced; read assert_cmd-2.2.2/src/lib.rs:115-119]` |
| `serde_json` feature `preserve_order` | **DO NOT ENABLE** | Swaps `Map` from `BTreeMap` to `IndexMap`. Features are unified across the graph, so this is a byte-shape change any future dependency could impose. `[VERIFIED: serde_json-1.0.151/src/map.rs:1-36, 66 `preserve_order` cfg sites]` |

---

## Package Legitimacy Audit

`gsd-tools query package-legitimacy check --ecosystem crates …` `[VERIFIED: ran the seam]`

| Package | Registry | First published | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----------------|-----------|-------------|---------|-------------|
| `csv` | crates.io | 2014-11-21 | 3,490,786/wk | github.com/BurntSushi/rust-csv | OK | Approved |
| `serde_json` | crates.io | 2015-08-07 | 22,522,340/wk | github.com/serde-rs/json | OK | Approved |
| `assert_cmd` | crates.io | 2018-05-28 | 1,217,085/wk | github.com/assert-rs/assert_cmd | OK | Approved (dev) |
| `tempfile` | crates.io | 2015-04-14 | 13,362,731/wk | github.com/Stebalien/tempfile | OK | Approved (dev) |
| `predicates` | crates.io | 2017-06-02 | 3,007,756/wk | github.com/assert-rs/predicates-rs | OK | Legitimate, **not adopted** (unnecessary) |
| `insta` | crates.io | 2019-01-13 | 1,972,642/wk | github.com/mitsuhiko/insta | OK | Legitimate, **not adopted** (dep weight, `fastrand`) |
| `schemars` | crates.io | 2019-08-08 | 11,815,846/wk | github.com/GREsau/schemars | OK | Legitimate, **not adopted** (wrong output — see above) |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

Cargo has no `postinstall` equivalent; all four adopted crates are already present in the
lockfile's resolution graph after `cargo add`, and all four are byte-fetched from
`static.crates.io` through the pinned lockfile. `[VERIFIED: cargo fetch succeeded, cargo tree read]`

---

## Architecture Patterns

### System Architecture Diagram

```
   config/baseline.toml ──► config::load ──► (Params, config_sha256)
   --seed <u64> ─────────────────────────────────► effective_seed
   --out <dir> ──────────────────────────────────► run directory path
                                 │
                                 ▼
                       Books::new(&params)
                    (endows, then CLEARS the
                     endowment postings so
                     tick 0's journal is empty)
                                 │
              ┌──────────────────┴──────────────────┐
              ▼                                     ▼
      World { households, firms,          books.accounts() + cash_of/stock_of
              household_order,                      │
              firm_order,                           ▼
              draws_this_tick,             Event::Endowment × 220
              activation_digest }          ──► events.jsonl  (the replay origin
                                                              Phase 4 HARN-02 needs)
                                 │
                    ┌────────────┴────────────┐
                    ▼                         │
        ┌─── per tick, 3650× ─────────────────┴──────────────────────┐
        │  shuffle_activation:                                       │
        │    Stream(tick, 0, ActivationOrderHouseholds) ─► 199 draws │
        │    Stream(tick, 0, ActivationOrderFirms)      ─►  19 draws │
        │    activation_digest = sha256(order)[..8]  ◄── SEED-       │
        │                                                SENSITIVE  │
        │  for (id, name, phase) in PHASES  (9 entries, in order):   │
        │    0 firm_planning   ─┐                                    │
        │    1 labour_market    │                                    │
        │    2 production       │ no-ops in this phase               │
        │    3 wages            │                                    │
        │    4 goods_market     │                                    │
        │    5 firm_accounting  │                                    │
        │    6 bankruptcy      ─┘                                    │
        │    7 invariants ──► CheckSet::run(&books, tick) ─Err─► HALT│
        │    8 log        ──► sink.tick_row(TickRow{..})             │
        │  books.end_of_tick(); tick += 1                            │
        └────────────────────────────────────────────────────────────┘
                    │                                   │
                    ▼                                   ▼
          sink.finish() (flush BEFORE       eprintln!("INVARIANT VIOLATION: {v}")
          inspecting the Result, or the      exit(1)   ── stderr names tick 0
          ticks leading to the halt are
          lost in the BufWriter)
                    │
        ┌───────────┴────────────┬────────────────┬──────────────────┐
        ▼                        ▼                ▼                  ▼
   runs/<id>/ticks.csv    events.jsonl    provenance.csv     run_meta.json
   9 int64 columns        220+ rows       header + 0 rows    seed, config
   202,974 bytes          18,560 bytes    49 bytes           hash, rustc
        │                        │                │                  │
        └── DIFFED ──────────────┴────────────────┘        EXCLUDED FROM DIFF
                    │                                       (only wall-clock-
                    ▼                                        capable file)
        schema/schema.json  ◄── generated by `sim --dump-schema`,
                                COMMITTED, drift is a test failure
                    │
                    ▼
        Phase 4 Python harness (reads the schema, never the Rust)
```

### Recommended Project Structure

```
src/
├── books.rs          # Phase 2 — unchanged
├── config.rs         # Phase 1 — unchanged
├── ids.rs            # Phase 1 — unchanged
├── invariants.rs     # Phase 2 — unchanged
├── lib.rs            # + pub mod log; pub mod phases; pub mod world;  (alphabetical)
├── log.rs            # Sink, TickRow, Event, ProvenanceRow, RunWriter, schema_json()
├── main.rs           # REWRITTEN — the real CLI (see Pitfall 11)
├── money.rs          # Phase 1 — unchanged
├── numeric.rs        # Phase 1 — unchanged
├── phases.rs         # Ctx, PhaseId, PHASES, tick(), run()
├── rng.rs            # Phase 1 — unchanged
└── world.rs          # World, Household, Firm  (NO balance fields)
build.rs              # emits SIM_RUSTC_VERSION for run_meta.json
schema/
└── schema.json       # GENERATED and COMMITTED
tests/
├── golden/           # 50-tick committed run directory
├── phase_order.rs    # (or keep the order test in-module, as invariants.rs does)
├── determinism.rs    # in-process + cross-process + different-seed
├── log_schema.rs     # drift test
└── lints.sh          # + guard 7f-agents (see § Pattern 7)
```

Measured sizes of the working probe: `world.rs` 40 lines, `phases.rs` 162 lines, `log.rs` 270
lines. `[VERIFIED: wc -l on the built tree]`

**`log.rs` is a single file, not a `log/` directory.** The project research sketched
`log/{mod,schema}.rs`, but 270 lines does not need a directory and the flat alphabetical
`src/` ordering CONTEXT.md names is the existing convention. Revisit if a per-firm panel is
added.

---

### Pattern 1: `const PHASES` — the same construction as `ALL_CHECKS`

**What:** A `const` array of `(PhaseId, &str, PhaseFn)` triples, with a companion
`PhaseId::ALL` constant and an **exhaustive-match** position function in the test module.

**When to use:** Whenever the ordering *is* the specification.

CONTEXT.md asks whether Phase 2's `ALL_CHECKS` construction applies here. **It does, exactly** —
`src/invariants.rs:403-469` is the template and it should be copied structurally, not merely
imitated. Four claims come free from it, and they are four *different* claims:

1. the table runs the documented sequence (`PHASES` vs `PhaseId::ALL`, element for element);
2. an identifier cannot exist without a table entry (`documented_position` is an exhaustive
   `match` — a tenth phase stops the test module compiling until it is placed);
3. the names spell their identifiers (derived by a `snake_case` helper, never a second
   hand-written list);
4. the derived `Ord` on `PhaseId` agrees with the run order.

```rust
// src/phases.rs — compiled and run
pub type PhaseFn = fn(&mut Ctx) -> Result<(), Violation>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhaseId {
    FirmPlanning, LabourMarket, Production, Wages, GoodsMarket,
    FirmAccounting, Bankruptcy, Invariants, Log,
}

impl PhaseId {
    /// Declared in run order, so the derived `Ord` agrees with the table.
    pub const ALL: [PhaseId; 9] = [ /* … same nine, same order … */ ];
}

pub const PHASES: [(PhaseId, &str, PhaseFn); 9] = [
    (PhaseId::FirmPlanning,   "firm_planning",   noop),
    (PhaseId::LabourMarket,   "labour_market",   noop),
    (PhaseId::Production,     "production",      noop),
    (PhaseId::Wages,          "wages",           noop),
    (PhaseId::GoodsMarket,    "goods_market",    noop),
    (PhaseId::FirmAccounting, "firm_accounting", noop),
    (PhaseId::Bankruptcy,     "bankruptcy",      noop),
    (PhaseId::Invariants,     "invariants",      run_invariants),
    (PhaseId::Log,            "log",             run_log),
];
```

**Two deviations from the ARCHITECTURE.md sketch, both deliberate.**

- The sketch's `PhaseFn` is `fn(&mut Ctx)` with no return. It must be
  `fn(&mut Ctx) -> Result<(), Violation>`: LEDG-10 requires the invariant phase to *return*
  a `Result`, and a phase that calls `std::process::exit(1)` from inside the library makes the
  halt untestable in-process. `[VERIFIED: built both; the Result form is what tests/invariant_halt.rs
  can already reach]`
- The sketch's `Ctx` carries `&mut Rng`. Phase 1's `Rngs::stream(&self, …)` takes `&self`, so
  `Ctx` carries `&Rngs`. `[VERIFIED: src/rng.rs:223]`

**Each phase completes for all agents before the next begins by construction** — a phase
function is a full loop and the next does not start until it returns. There is no per-agent
`step()` anywhere. Nothing extra is needed to satisfy that clause of TICK-01; it is worth a
sentence in the module doc so a later reader does not add one.

### Anti-pattern: a `Vec<Box<dyn Phase>>` pipeline

Adds a registration step, which is the exact affordance that lets ordering drift at *runtime*
and defeats the order test. Rejected in ARCHITECTURE.md and still right.

---

### Pattern 2: Activation order that actually witnesses the seed

**The design CONTEXT.md and the ROADMAP prescribe does not satisfy TICK-10.** See Pitfall 1 for
the measurement. The correct shape:

```rust
/// Activation order: the one RNG consumption an empty tick makes (criterion 3).
fn shuffle_activation(w: &mut World, rngs: &Rngs) {
    w.draws_this_tick = 0;

    // Pools are rebuilt per draw site, never shared: src/rng.rs:317-334 makes
    // pool aliasing a caller obligation, and a reused buffer couples two
    // purposes through state even though their keystreams are independent.
    w.household_order.clear();
    w.household_order.extend(0..w.households.len() as u32);
    let mut s = rngs.stream(w.tick, 0, Purpose::ActivationOrderHouseholds);
    s.shuffle_in_place(&mut w.household_order);
    w.draws_this_tick += s.draws();

    w.firm_order.clear();
    w.firm_order.extend(0..w.firms.len() as u32);
    let mut s = rngs.stream(w.tick, 0, Purpose::ActivationOrderFirms);
    s.shuffle_in_place(&mut w.firm_order);
    w.draws_this_tick += s.draws();

    // The order's OWN VALUE must reach the log, not just the draw count.
    w.activation_digest = order_digest(&w.household_order, &w.firm_order);
}

/// A 63-bit digest of the tick's activation order, using `sha2` — ALREADY a
/// dependency (src/config.rs:485). Shifted right one bit so the CSV column
/// parses as a positive int64 in pandas rather than overflowing to object.
fn order_digest(households: &[u32], firms: &[u32]) -> i64 {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for x in households { h.update(x.to_le_bytes()); }
    h.update(b"|");
    for x in firms { h.update(x.to_le_bytes()); }
    let d = h.finalize();
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[..8]);
    (u64::from_le_bytes(b) >> 1) as i64
}
```

`Purpose::ActivationOrderHouseholds = 10` and `ActivationOrderFirms = 11` **already exist**
`[VERIFIED: src/rng.rs:67,69 — "Household activation order for the tick (Phase 3)" /
"Firm activation order for the tick (Phase 3)"]`, so no `Purpose` variant is appended and
`ALL_PURPOSES` does not change. That matters: the discriminants are append-only and renumbering
one re-keys every sub-stream after it.

**Draw counts, measured:** 199 (households, 200−1) + 19 (firms, 20−1) = **218 per tick**,
constant. `Stream::shuffle_in_place` takes exactly `pool.len() - 1` draws `[VERIFIED: src/rng.rs:355-366
and the measured column]`.

**Why a digest rather than "the first activated household".** A permutation that differs only in
its tail is exactly the divergence a first-element column misses. A digest over the whole
permutation costs one sha256 of 880 bytes per tick — the same crate the config hash already
uses — and 3,650 ticks completed in 10 ms release including it. `[VERIFIED: timed]`

**Alternative worth considering:** log two columns, `activation_digest` **and**
`first_household_activated`. The digest is the sensitive detector; the first element is the
human-readable localiser when the digest tells you a tick diverged. Cheap; the planner's call.

---

### Pattern 3: The `Sink` trait and `RunWriter`

```rust
pub trait Sink {
    fn tick_row(&mut self, row: TickRow);
    fn event(&mut self, e: Event);
    fn provenance(&mut self, p: ProvenanceRow);
    fn finish(&mut self) -> std::io::Result<()>;
}
pub struct NullSink;                 // in-process runs that write nothing
#[derive(Default)] pub struct VecSink { … }  // unit tests assert on records directly
pub struct RunWriter { … }           // the disk writer
```

`finish()` rather than `flush()`: it returns `io::Result` and is called **once**, before the
tick loop's `Result` is inspected. See Pitfall 9.

**`TickRow` — flat, integer-only, no `Option`:**

```rust
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TickRow {
    pub tick: u32,
    pub total_money_cents: i64,
    pub firm_cash_cents: i64,
    pub stock_units: i64,
    pub headcount: u64,
    pub transactions: u32,
    pub rng_draws: u32,
    pub activation_digest: i64,
    pub postings: u32,
}
```

Read back with pandas 3.0.5: 3,650 rows × 9 columns, **every dtype `int64`**,
`total_money_cents.nunique() == 1` at value 2,000,000. `[VERIFIED: ran it]`

`firm_cash_cents / total_money_cents` is OWN-07's deflationary-stall signal; both columns are
present from this phase's first commit so the ratio is computable for the whole history.

**`Event` — externally tagged, snake_case, append-only:**

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Hire { tick: u32, firm: String, household: u32, wage_cents: i64 },
    Fire { tick: u32, firm: String, household: u32 },
    Dividend { tick: u32, firm: String, household: u32, amount_cents: i64 },
    Bankruptcy { tick: u32, firm: String, residual_cents: i64 },
    /// The opening endowment, one row per account. Read from the books'
    /// ACCESSORS at setup, never from an endowment posting: `Books::new`
    /// clears those before tick 0 so liveness cannot pass on them.
    /// Phase 4's conservation audit REPLAYS FROM THIS ROW (HARN-02).
    Endowment { tick: u32, account: String, cash_cents: i64, units: i64 },
}
```

The tag is emitted **first**, then fields in declaration order:
`{"event":"hire","tick":0,"firm":"firm:3:0","household":12,"wage_cents":6300}`.
`[VERIFIED: ran it]`

**`ProvenanceRow` — flat, and its header is written eagerly:**

```rust
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProvenanceRow {
    pub tick: u32,
    pub agent: &'static str,      // rendered address: "firm:3:0"
    pub decision: &'static str,   // "price" | "wage" | "hire"
    pub input_a: i64,
    pub input_b: i64,
    pub outcome: i64,
    pub rule: &'static str,       // WHICH BRANCH fired
}
```

The `rule` column is the highest-value field and costs nothing: when prices spiral in Phase 9, a
`value_counts()` on it localises the bug to a branch in one query. It is a fixed enumeration of
`&'static str`, which is what keeps TICK-07's "never free text" true by construction.

---

### Pattern 4: Schema generated from the writers, not from a second derive

**The generator reads what the writers actually emit.** Column names come from the header line
`csv::Writer` produces; JSON field order comes from the text `serde_json` produces; dtypes come
from reparsing that same text into a `Value`. There is no second description of the types
anywhere, so the schema cannot disagree with the file — which is the property `schemars` fails.

```rust
fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        Value::String(_) => "string",
        Value::Bool(_)   => "bool",
        Value::Number(n) if n.is_i64() || n.is_u64() => "int64",
        Value::Number(_) => "float64",
        Value::Null      => "null",
        _ => "UNSUPPORTED",
    }
}

/// Ordered `(field, dtype)` read off the bytes serde_json actually writes.
/// Key order from the TEXT (declaration order); dtype from the parsed value —
/// so a `serialize_with` rendering an address as "household:12" is reported
/// as a string, which is what it is.
fn json_fields<T: Serialize>(v: &T) -> Vec<(String, &'static str)> { … }

/// Ordered `(column, dtype)` for a CSV row type: names from the header
/// `csv::Writer` ITSELF emits, dtypes from the same serde impl.
fn csv_columns<T: Serialize>(exemplar: &T) -> Vec<(String, &'static str)> { … }

pub fn schema_json() -> String { … }   // deterministic string, byte-stable
```

Measured output (abridged) — note `posting.debit` / `posting.credit` correctly typed `string`:

```json
{
  "schema_version": "v1",
  "ticks.csv": [
      { "name": "tick", "dtype": "int64" },
      { "name": "total_money_cents", "dtype": "int64" },
      … 7 more …
  ],
  "provenance.csv": [ { "name": "tick", "dtype": "int64" }, … ],
  "events.jsonl": [
    { "event": "hire", "fields": [ … ] },
    { "event": "posted", "fields": [
      { "name": "posting.debit", "dtype": "string" },
      { "name": "posting.credit", "dtype": "string" }, … ] }
  ]
}
```

**The drift test, and how it avoids comparing a file to itself:**

```rust
#[test]
fn schema_matches_the_committed_file() {
    let generated = sim::log::schema_json();
    let committed = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/schema/schema.json")
    ).expect("schema/schema.json is committed");
    assert_eq!(generated, committed,
        "schema drift: run `sim --dump-schema > schema/schema.json`");
}
```

The test **never writes**. Regeneration is a separate operator action (`sim --dump-schema`), so
the committed bytes are an independent artifact of a prior decision, exactly as `clippy.toml` is.
A test that regenerated-then-compared would be comparing the generator to itself.

**Mutation-proved twice** `[VERIFIED: both mutations run, both failed, revert passed]`:

| Mutation | Result |
|----------|--------|
| Swap two column entries in the committed `schema/schema.json` | **FAILED** as required |
| Rename `rng_draws` → `rng_draws_per_tick` in the Rust type only | **FAILED** as required |
| Revert | passed |

**One improvement the planner should make:** the raw `assert_eq!` prints a 2.7 KB single-line
escaped blob, which is unreadable. Add a helper that reports the first differing line number and
those two lines. No dependency needed.

---

### Pattern 5: The run directory and `run_meta.json`

```
runs/<id>/
  ticks.csv        202,974 bytes  (3,650 rows × 9 int64 columns)   ── DIFFED
  events.jsonl      18,560 bytes  (220 endowment rows at this phase) ── DIFFED
  provenance.csv        49 bytes  (header + 0 rows)                ── DIFFED
  run_meta.json        223 bytes                                   ── EXCLUDED
schema/schema.json                                                 ── committed, drift-tested
```
`[VERIFIED: all four byte counts measured from a real run]`

```json
{
  "schema_version": "v1",
  "seed": 42,
  "config_sha256": "b3530ae4d072b7ad4c4070e3f200cceb5e3fab9c6ab616fe51eab36f49dbd4b8",
  "rustc": "rustc 1.94.1 (e408947bf 2026-03-25)",
  "ticks_completed": 3650,
  "exit": "ok"
}
```

`ticks_completed` and `exit` are the two fields that make a halted run self-describing: the
liveness-halt run recorded `"ticks_completed": 0, "exit": "violation"` `[VERIFIED: ran it]`.

**The toolchain string comes from a `build.rs`,** because `rustc --version` at runtime would be
a process spawn on the behaviour path:

```rust
// build.rs — verified
fn main() {
    let v = std::process::Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .arg("--version").output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=SIM_RUSTC_VERSION={v}");
    println!("cargo:rerun-if-changed=build.rs");
}
```

Note: adding `build.rs` changes nothing `tests/toolchain.sh` checks — it passed unchanged.
`[VERIFIED: ran it]`

**No wall clock is present at all in the probe's `run_meta.json`.** CONTEXT.md says it is the
only file that *may* carry one; it does not say it must. If a `started_at` is wanted, add it here
and nowhere else. **Do not** put a `duration_ms` in it: a run's own record then differs between
two identical runs, which invites someone to "fix" the determinism test by widening it.

**Enforcement of the exclusion, rather than documentation of it** — this test was written and
run, and its four clauses are four different claims:

```rust
const EXCLUDED_FROM_DIFF: [&str; 1] = ["run_meta.json"];

#[test]
fn the_exclusion_is_enforced_not_documented() {
    // two runs at one seed, into two temp directories …
    let files = entries(&a);                        // BTreeSet, never HashSet
    assert_eq!(files, entries(&b), "the two runs produced different file sets");

    // 1. The excluded file must EXIST. Excluding a file that was never written
    //    is a vacuous exclusion and the rule enforces nothing.
    for x in EXCLUDED_FROM_DIFF {
        assert!(files.contains(x), "{x} is excluded from the diff but was never written");
    }
    // 2. Every OTHER file is diffed — ENUMERATED FROM THE DIRECTORY, never from
    //    a hand-written list that could fall behind what the run writes.
    let mut diffed = 0;
    for f in &files {
        if EXCLUDED_FROM_DIFF.contains(&f.as_str()) { continue }
        // 3. A diffed file must be NON-EMPTY: two empty files hash equal and the
        //    comparison certifies nothing.
        assert!(std::fs::metadata(a.join(f)).unwrap().len() > 0,
                "{f} is empty — comparing it to another empty file proves nothing");
        assert_eq!(sha(&a.join(f)), sha(&b.join(f)), "{f} differs between two runs at one seed");
        diffed += 1;
    }
    assert_eq!(diffed, files.len() - EXCLUDED_FROM_DIFF.len());
    assert!(diffed >= 3);
    // 4. TICK-06: no path, hostname or PID in any diffed file.
}
```

This test **failed first** against the naive design (clause 3 caught the two empty files) and
passes against the corrected one, reporting `diffed 3 of 4 files; excluded ["run_meta.json"]`.
`[VERIFIED: both runs]`

---

### Pattern 6: Determinism tests — three different claims, three tests

| Test | What it proves | Measured |
|------|----------------|----------|
| `same_seed_identical_in_process` | Reproducibility in one process | Two `VecSink`/`RunWriter` runs, byte-equal |
| `same_seed_identical_across_processes` | No global state, env leakage or allocator-order effect | `assert_cmd`, two invocations, sha256 pairs equal `[VERIFIED]` |
| `different_seed_differs` | **The seed is actually wired in** | `ticks.csv` differs from tick 0 `[VERIFIED]` |

**And a fourth that came for free and is worth keeping:** debug and release builds produce
**byte-identical** `ticks.csv` `[VERIFIED: cmp of a 3,650-tick debug run against the release run]`.
That is the artefact-level statement that no `debug_assertions`-gated code (the RNG re-entry
guard at `src/rng.rs:196-215`) leaks into the log.

**Scope warning for `different_seed_differs`.** At this phase `events.jsonl` contains only
endowments, which are **seed-independent**, so the two seeds produce identical event streams
`[VERIFIED: cmp returned equal]`. Write the assertion as "`ticks.csv` differs", not "every diffed
file differs" — the latter would be a red build for a correct sim.

### Pattern 7: Guard 7f extended — criterion 7's inherited obligation

Guard 7f at `tests/lints.sh:628-662` currently searches non-ledger `src/` files for
`\b(household_cash|firm_cash|household_stock|firm_stock|firm_headcount)\b` and for
`fn[[:space:]]+set_cash`. **Neither half catches the defect criterion 7 is about**: a field
spelled `cash` on `struct Household` matches neither pattern. The guard is silent on the new
types not because they are clean but because it cannot see them.

The extension below was written, run against the clean tree, and mutation-proved on **three**
hazard shapes. It follows the file's own `assert_fires` / `assert_ignores` / `assert_absent`
discipline, and it carries the `assert_ignores` clause that the first draft failed — the naive
money-type pattern fired on a legitimate `let held: Money = books.cash_of(a).unwrap();`.

```bash
# 7f-agents. Household and Firm hold no balance (ROADMAP Phase 3 criterion 7).
mapfile -t AGENT_SRC < <(git ls-files -- 'src/world.rs')
if [ "${#AGENT_SRC[@]}" -eq 0 ]; then
    fail "guard 7f-agents: src/world.rs is not tracked — the guard would pass trivially"
fi

# (a) An agent type may not declare a money-TYPED field. Anchored to struct-field
#     syntax (trailing comma, line-anchored) so a legitimate local binding of type
#     Money is left alone — a pattern without that anchor fires on
#     `let held: Money = books.cash_of(a).unwrap();`, which was reproduced.
MONEY_FIELD='^[[:space:]]*(pub[[:space:]]+)?[a-z_]+[[:space:]]*:[[:space:]]*(crate::money::)?(Money|Cents)[[:space:]]*,[[:space:]]*$'
assert_fires 7f-agents "$MONEY_FIELD" 4 '    pub cash: Money,
    balance: Money,
    pub wallet : Cents,
    pub hoard: crate::money::Money,'
assert_ignores 7f-agents "$MONEY_FIELD" '    pub price_cents: i64,
    pub id: HouseholdId,
    fn cash_of(&self, a: Account) -> Option<Money> {
    let held: Money = books.cash_of(a).unwrap();
    let m: Money = Money::ZERO;'
assert_absent "guard 7f-agents: an agent type in src/world.rs declares a money-typed field. The books own every cent (LEDG-01, ROADMAP Phase 3 criterion 7)" \
    -nE "$MONEY_FIELD" -- "${AGENT_SRC[@]}"

# (b) A balance-SHAPED field name, whatever its type. An i64 named `cash` evades
#     (a) entirely and is the same defect.
BALANCE_NAME='^[[:space:]]*(pub[[:space:]]+)?(cash|balance|funds|wallet|savings|deposits|inventory|stock|units_held|headcount|employees)[[:space:]]*:'
assert_fires 7f-agents-name "$BALANCE_NAME" 4 '    pub cash: i64,
    balance: u64,
    pub inventory: i64,
    headcount: u32,'
assert_ignores 7f-agents-name "$BALANCE_NAME" '    pub price_cents: i64,
    pub id: HouseholdId,
    let cash: Money = books.cash_of(a).unwrap();
    fn cash(&self) -> Money {'
assert_absent "guard 7f-agents: an agent type in src/world.rs declares a balance-shaped field. The books own the quantity" \
    -nE "$BALANCE_NAME" -- "${AGENT_SRC[@]}"

# (c) The types this criterion is about must actually EXIST here, or (a) and (b)
#     are guards over nothing — the same "pin the probe to a declaration count"
#     discipline guard 7j already uses.
for t in Household Firm; do
    grep -qE "^(pub )?struct $t\b" "${AGENT_SRC[@]}" \
      || fail "guard 7f-agents: struct $t is not declared in src/world.rs — the guard polices a set that does not contain the types criterion 7 names"
done
```

**Mutation results** `[VERIFIED: all four runs]`

| Mutation to `src/world.rs` | Guard | Result |
|---|---|---|
| `pub cash: crate::money::Money,` on `Household` | 7f-agents (a) | **FAILED** — "declares a money-typed field" |
| `pub cash: i64,` on `Firm` | 7f-agents (b) | **FAILED** — "declares a balance-shaped field" |
| `struct Household` renamed to `struct Consumer` | 7f-agents (c) | **FAILED** — "the guard polices a set that does not contain the types criterion 7 names" |
| clean tree | all three | passes |

The existing 7f `set_cash` half stays as it is; it now polices a set that genuinely can contain
the thing it forbids. `tests/lints.sh` in full passed unchanged against the built Phase 3 tree
(all 7 checks, 60 method bans, 10 source guards). `[VERIFIED: ran it]`

---

### Pattern 8: Golden run instead of `insta`

**Recommendation: skip `insta`; commit a 50-tick golden run directory and compare it with
`assert_eq!` / a sha256 pair.** Reasons, in the order that decides it:

1. `insta` adds **17 packages** including `fastrand`, which `CLAUDE.md` names in its "What NOT
   to Use" table, and `getrandom`. They arrive as dev edges so `tests/toolchain.sh` stays green
   — but a reader who greps the tree finds a banned crate and has to reconstruct why.
   `[VERIFIED: ran cargo add --dev insta --features json]`
2. The repo already has the review workflow `cargo insta review` provides, in a different form:
   generated artifact → committed → drift test (`clippy.toml`, and now `schema/schema.json`).
   Adding a second, tool-specific spelling of the same idea costs consistency.
3. The comparison is 8 lines with no dependency.

**On the 50-tick window — the project research's stated reason is wrong, but its number is
right.** The research says all 3,650 ticks "would make a deliberate rule change unreviewable".
Measured: a **localised** change (one value on one tick) diffs to the identical 4-line hunk at
50 rows and at 3,650 rows — `diff` prints only the hunk. The real numbers are for a
**trajectory-wide** change:

| Window | File size | `diff` output on a seed change |
|--------|-----------|-------------------------------|
| 50 ticks | 2,795 bytes | 102 lines |
| 3,650 ticks | 202,974 bytes | 7,302 lines |

`[VERIFIED: measured all four]`

So keep 50, and record the honest reason: a rule change moves the whole trajectory, and 102 lines
is reviewable while 7,302 is not — plus 203 KB rewritten in the repo on every deliberate change.

**Add one constraint the research did not state:** the window must span at least two planning
cycles, or a cadence change is invisible in it. The cadence is a **21-day month**
(`month_days = 21`), so the window must be **≥ 42**. Fifty satisfies this with margin, which is
the strongest argument for the number. Do not shrink it to 20.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSV quoting, escaping, terminators | A `join(",")` writer | `csv` 1.4.0 | Handles embedded commas, quotes and newlines with `QuoteStyle::Necessary`; terminator is `\n` with no platform branch `[VERIFIED: csv-core-0.1.13/src/writer.rs:24]` |
| Shortest round-trip number formatting | `format!("{}", x)` | `csv` (itoa/ryu), `serde_json` (itoa/zmij) | Both are shortest-round-trip and deterministic; a hand-rolled float format is where byte-stability dies |
| JSON escaping | `format!("{{\"a\":{}}}", …)` | `serde_json::to_writer` | One unescaped `"` in an address string silently corrupts the stream |
| Invoking the built binary in a test | `std::process::Command::new("./target/debug/sim")` | `assert_cmd::Command::cargo_bin("sim")` | Resolves the correct profile's binary; a hard-coded path silently tests a stale build |
| Temp directories | `std::env::temp_dir().join(format!("…{}", process::id()))` | `tempfile::tempdir()` | The existing `tests/tracer_end_to_end.rs:21` does the hand-rolled version and leaks the directory on a failing assert |
| JSON Schema from Rust types | `schemars` derive | The ~90-line emitter in Pattern 4 | schemars is a **second** description and cannot see `serialize_with` — measured wrong on this repo's own `Posting` |
| Hashing bytes | A new hash crate | `sim::config::config_hash` (`sha2`, already a dependency) | `src/config.rs:485`; already used for the config hash and reusable for file digests |
| A permutation checksum | `wrapping_mul` rolling hash | `sha2` over the order bytes | Same crate, no wrapping arithmetic to argue about under `overflow-checks = true` |

**Key insight:** every hand-rolled candidate in this phase fails in the same direction — it
produces *plausible* bytes that differ from what a second reader expects. The whole phase exists
to make the bytes a contract, so a plausible-but-different byte is the defect class, not a
performance question.

---

## Common Pitfalls

### Pitfall 1: The vacuous-reproducibility trap — the prescribed design falls into it

**What goes wrong:** `different_seed_differs` passes trivially… by *failing*. Two runs at
different seeds produce byte-identical logs, so TICK-10 is red — or, worse, someone deletes the
test.

**Measured, first-hand.** I built exactly what CONTEXT.md and ROADMAP criterion 3 describe — an
activation-order shuffle (218 draws per tick) plus a `rng_draws` column — ran 3,650 ticks at
seed 42 and seed 43, and `cmp` returned **0: byte-identical**.

**Why it happens:** the RNG *is* consumed, but nothing observable depends on it. `rng_draws` is
218 on every tick of every run at every seed — a constant column proves draws occurred and says
nothing about which draws. The activation order affects nothing because every phase is a no-op.

**How to avoid:** log a seed-sensitive **value**, not a seed-independent **count**. One extra
`activation_digest` column (Pattern 2) flipped the identical run to differing at tick 0.

**Warning signs:** any determinism column whose `nunique()` over a run is 1. A useful standing
check for Phase 4: `assert df.activation_digest.nunique() > 1`.

**Keep the draw-count column anyway** — it is the divergence *localiser* (`src/rng.rs:261-262`
says so), and its constancy is itself an assertion worth making: a tick whose draw count moved is
a fixed-draw-sampling violation (CORE-05).

---

### Pitfall 2: `schemars` silently describes a shape the writer does not produce

**What goes wrong:** `schema/schema.json` is generated, committed, drift-tested and **wrong**.
Phase 4's validator rejects every real posting, and the drift test never fires because the
generator and the file agree with each other.

**Measured.** With `Posting`'s `#[serde(serialize_with = "serialize_account")]`
(`src/books.rs:215`):

```
ACTUAL serde_json  -> {"seq":0,"debit":"household:12","debit_cents":100}
SCHEMARS SCHEMA    -> "debit": { "$ref": "#/$defs/Account" }
                      "Account": { "oneOf": [ {"Household": integer}, {"Firm": object} ] }
```

schemars also emits `properties` **alphabetically** (`debit`, `debit_cents`, `seq`) while
`required` is in declaration order — so it does not record the CSV column order either, and
column order is the `ticks.csv` contract.

**Why it happens:** `#[derive(JsonSchema)]` and `#[derive(Serialize)]` are two independent
macros. schemars offers `#[schemars(with = "String")]` as a manual override, but that is a
promise a human must remember on every `serialize_with` field, forever.

**How to avoid:** derive the schema from the serialiser's output (Pattern 4). Then the override
problem cannot exist.

**Warning signs:** any schema field whose declared type is an object while the file shows a
string.

---

### Pitfall 3: `csv::Writer` writes the header **twice** if you write it yourself

**What goes wrong:** `ticks.csv` opens with two identical header lines and pandas reads the
second one as a data row of strings, turning every column to `object`.

**Measured:**

```rust
let mut w = csv::Writer::from_writer(vec![]);
w.write_record(["tick", "a"]).unwrap();      // your header
w.serialize(Row { tick: 0, a: 1 }).unwrap(); // csv writes ITS header too
// → "tick,a\ntick,a\n0,1\n"
```

**How to avoid:** `csv::WriterBuilder::new().has_headers(false)` and then write the header
yourself. Verified output: `"tick,a\n0,1\n"`, and with zero rows: `"tick,a\n"`.

**Why you need the eager header at all:** see Pitfall 4.

---

### Pitfall 4: A zero-row CSV is a **zero-byte** file, and pandas refuses it

**Measured, both halves:**

- `csv::Writer` emits its header on the **first `serialize`**, so a run with no provenance rows
  leaves `provenance.csv` at **0 bytes**.
- `pandas.read_csv` on that file raises `EmptyDataError: No columns to parse from file`
  `[VERIFIED: pandas 3.0.5]`.

Phase 3 writes **zero** provenance rows by definition, so this is not hypothetical — it is the
default outcome.

**How to avoid:** the eager header from Pitfall 3. A header-only `provenance.csv` reads back as
`(0, 7)` with the correct column names `[VERIFIED]`.

**One consequence to hand to Phase 4:** a header-only CSV gives every column dtype **`object`**,
not `int64` `[VERIFIED]`. HARN-02's dtype assertion must therefore be conditional on a non-empty
frame, or read the dtype from `schema/schema.json` (which is why the schema carries dtypes).

`events.jsonl` behaves differently and is safe: `pd.read_json(path, lines=True)` on a zero-byte
file returns an empty `(0, 0)` DataFrame with **no exception** `[VERIFIED]`.

---

### Pitfall 5: `csv` refuses nested structs and sequences — at runtime, not compile time

**Measured:**

| Row shape | Result |
|-----------|--------|
| nested struct field | `ERR CSV write error: cannot serialize Inner container inside struct when writing headers from structs` |
| `Vec<i64>` field | `ERR … cannot serialize sequence container inside struct …` |
| unit-variant enum field | **OK** — writes the snake_case name |
| `Option<i64>` field | **OK** — `None` writes an **empty field** |

Two consequences.

- `TickRow` and `ProvenanceRow` must be **flat**. The failure is a runtime `Err`, so a nested
  field compiles and fails only when the row is first written — put the schema drift test in the
  suite and it fires at build time instead.
- **Never use `Option` in a CSV row type.** An empty field makes pandas infer `float64` (NaN) for
  an otherwise-integer column, which is exactly the degradation the `*_cents` decision exists to
  prevent. If a value can be absent, use a sentinel integer and document it in the schema.

`Posting` itself **does** serialise to CSV successfully — all its fields are scalars once the two
`serialize_with` addresses render to strings `[VERIFIED]`. Useful if the opt-in journal dump
lands later.

---

### Pitfall 6: `serde_json` serialises a `HashMap` in the map's own iteration order

**What goes wrong:** an event or metadata object carrying a `HashMap` field produces different
bytes on **every process**, and the determinism test fails intermittently and unreproducibly.

**Measured — five consecutive runs of one binary:**

```
{"zeta":0,"alpha":1,"beta":3,"mid":2,"kilo":7,"delta":5,"omega":6,"yankee":4}
{"alpha":1,"zeta":0,"yankee":4,"omega":6,"mid":2,"beta":3,"kilo":7,"delta":5}
{"beta":3,"yankee":4,"delta":5,"zeta":0,"kilo":7,"mid":2,"alpha":1,"omega":6}
{"alpha":1,"delta":5,"beta":3,"yankee":4,"omega":6,"mid":2,"kilo":7,"zeta":0}
{"kilo":7,"omega":6,"beta":3,"alpha":1,"zeta":0,"mid":2,"yankee":4,"delta":5}
```

**This corrects a claim in `CLAUDE.md` and `research/STACK.md`,** which state that serde_json's
"map keys [are] `BTreeMap`-sorted ⇒ byte-identical output". That is true **only** for
`serde_json::Value::Object`, whose backing type is a `BTreeMap` unless the `preserve_order`
feature is on `[VERIFIED: serde_json-1.0.151/src/map.rs:33-36]`. A `HashMap` **field** goes
through `serialize_map` and keeps its iteration order. Verified both: a `Value` round-trip of
`{"zeta","alpha","mid","beta"}` came back sorted; the `HashMap` did not.

**How to avoid:** the existing `clippy.toml` `disallowed-types` ban on `std::collections::HashMap`
is what actually closes this, plus `tests/lints.sh` check 4a (which catches a type **alias** the
lint cannot see). Nothing new is needed — but the reason it matters is now measured rather than
assumed, and the doc claim should be corrected.

---

### Pitfall 7: A float in the log is a silent corruption vector in **both** formats

**Measured.** `f64::NAN`, `f64::INFINITY`, `-0.0`, `0.1+0.2` and a subnormal `1e-320`:

| | CSV (`ryu`) | JSON (`zmij`) | pandas 3.0.5 read-back |
|---|---|---|---|
| `NaN` | `NaN` | **`null`** | `NaN` |
| `inf` | `inf` | **`null`** | `inf` |
| `-0.0` | `-0.0` | `-0.0` | `-0.0` |
| `0.1+0.2` | `0.30000000000000004` | `0.30000000000000004` | `0.3` |
| `1e-320` | `1e-320` | `1e-320` | **`9.999889e-321`** |

Three findings, all first-hand:

1. **JSON maps both `NaN` and `±Infinity` to `null`.** A runaway price and a NaN price are
   indistinguishable in `events.jsonl`, irreversibly. This is the strongest argument for keeping
   `expected_demand` out of the event stream when Phase 9 arrives.
2. pandas loses precision on a **subnormal** CSV float, so a CSV float does not round-trip
   through the Python side in general.
3. Both writers are otherwise deterministic — a float does not *break* byte-identity; it breaks
   *meaning*.

**How to avoid:** `ticks.csv` and `provenance.csv` carry integers only, as the flat integer row
types above enforce. When Phase 9 must log `expected_demand`, log it as an integer in milli-units
(`expected_demand_milli`), which `src/numeric.rs`'s `MILLI_SCALE` already exists for.

---

### Pitfall 8: The empty pipeline produces empty artifacts, and empty artifacts pass every test

**What goes wrong:** the cross-process determinism test hashes `events.jsonl` from two runs, gets
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` twice, and passes. That is
the sha256 of the **empty string** — the test certified nothing about the event stream.

**Measured:** the naive build produced exactly that. `[VERIFIED: printed the hash pairs]`

**How to avoid, two clauses:**

1. Assert every diffed file is **non-empty** before comparing it (Pattern 5, clause 3). This test
   failed against the naive build and passes against the corrected one.
2. Give `events.jsonl` genuine content at this phase: emit the **opening endowment** as one
   `Event::Endowment` per account, read from `books.accounts()` / `cash_of` / `stock_of` at
   setup.

Clause 2 is not a workaround; it is a requirement arriving early. HARN-02 defines the Phase 4
conservation audit as *"an event replay from the initial endowment"*, and without these rows there
is nothing to replay from. Measured: 220 endowment rows summing to exactly **2,000,000 cents**,
equal to `total_money_cents` `[VERIFIED: pandas `e.cash_cents.sum()`]`.

**The subtlety that makes this non-obvious.** `Books::new` deliberately **clears** the endowment
postings before tick 0, so the liveness check cannot pass on them (`src/books.rs:608` doc, step
5). So Phase 3 must synthesise these events from the **accessors**, exactly as that doc instructs
— `books.rs` says in as many words: *"Phase 3 therefore reads opening balances from the accessors
below rather than from an endowment event."*

---

### Pitfall 9: A `BufWriter` swallows the ticks that led to the halt

**What goes wrong:** the run halts at tick 1,847, and `ticks.csv` ends at tick 1,791 — the last
buffer boundary. The diagnostic evidence for the halt is the part that was dropped.

**Two separate causes, both real:**

- `csv::Writer` **does** flush on `Drop`, but **ignores the error** — verified: a writer dropped
  without `flush()` still produced a complete 14-byte file. So a write failure (full disk) is
  silent.
- `std::process::exit(1)` **does not run destructors**, so a plain `BufWriter` around
  `events.jsonl` loses whatever is still buffered.

**How to avoid:** the tick loop returns `Result`; `main` calls `sink.finish()?` **before**
inspecting that `Result`, and only then prints and exits. Measured: after the liveness halt at
tick 0, `run_meta.json` was complete and recorded `ticks_completed: 0`.

**One design consequence worth a decision.** Because `invariants` is phase 8 and `log` is phase 9,
the tick that violates is **never logged** — the halted run's `ticks.csv` was **0 bytes**
`[VERIFIED]`, which reintroduces Pitfall 4 for any halted run. Recommendation: write the
`ticks.csv` header eagerly (Pattern 3) so a halted run is still openable, and consider having the
halt path emit the offending tick's row before exiting. Do **not** solve it by moving `log` before
`invariants` — the check must run before the tick is declared good.

---

### Pitfall 10: The float-confinement guard fires on a semantic version **string**

**Measured, and it cost a build.** `tests/numeric_det.rs::confinement_of_the_float_domain` failed
with:

```
src/log.rs:182 contains a floating-point literal; only ["numeric.rs", "config.rs"] may
  (an inferred f64 needs no type name and calls no banned method, so it is invisible to
   both other guards): pub const SCHEMA_VERSION: &str = "1.0.0";
```

The matcher is deliberately comment-blind **and** string-blind — Phase 1 recorded that
*"a heuristic that skips comments is one someone later widens to skip a string"* and reworded
`src/rng.rs` rather than loosening the test.

**How to avoid:** follow that precedent. Do **not** add `log.rs` to `FLOAT_ALLOWLIST`
(`tests/numeric_det.rs:91`). Spell the schema version without a decimal literal in the source:
`"v1"` works and was what the probe used. If a semver string is genuinely wanted in the emitted
JSON, build it from integer consts and concatenate.

**Scope, measured:** this was the **only** collision in a 472-line Phase 3 implementation. The
string literals `"int64"` and `"float64"` do **not** trip either matcher (the `f64` in `float64`
has an identifier character before it, and the type matcher requires identifier boundaries)
`[VERIFIED: re-ran the guard clean after the one fix]`.

---

### Pitfall 11: `tests/tracer_end_to_end.rs` breaks the moment `main.rs` is rewritten

**Measured:** with the Phase 3 `main.rs` in place, `cargo test` reports **2 failures** in
`tests/tracer_end_to_end.rs` — `runs_end_to_end` and `different_seed_changes_the_draw`. Both
assert the Phase 1 tracer's single stdout line
(`tracer effective_seed=… config_sha256=… draw=… money_cents=…`), which Phase 3 replaces.

**How to avoid:** plan for it. Those two tests are not obsolete in *intent* — `different_seed_changes_the_draw`
is the ancestor of TICK-10 — so the honest move is to **port** them into the new determinism
test file rather than delete them, in the same commit that rewrites `main.rs`. The remaining
tests in that file (the overflow assertions at lines ~137-227) are about the release profile and
must survive untouched.

`[VERIFIED: ran the full suite against the Phase 3 tree — 244 tests, all green except these two]`

> **Count correction (2026-08-31, plan 03-01).** The 244 above is this prototype tree's count —
> it already carried the world and pipeline modules and their tests. The **repository's**
> pre-phase count is **242**, measured before and after plan 03-01, which changed only
> `Cargo.toml` and `Cargo.lock`. Plan 03-01's task 2 copied 244 into a `fails_when` and it has
> been corrected there. Later plans must expect 242 plus whatever they add, not 244 — a reader
> starting from 244 would go looking for two deleted tests that never existed here.

---

### Pitfall 12: A five-part config agreement, if any config leaf is touched

Phase 2 lost twelve library tests by missing the fifth part. If Phase 3 adds **any** config leaf,
all five must move together: `config/baseline.toml` with its two-line `# GRADE:` block, the
`src/config.rs` schema struct, `config/PROVENANCE.md`, the schema-leaf agreement, **and** the
hand-written `FULL` TOML fixture at `src/config.rs:503`.

**Recommendation: Phase 3 adds no config leaf.** See Open Question 1. `--out` is a CLI flag and
affects no behaviour; the schema version is a `const`; the run-directory name is derived. Keeping
the count at 41 avoids the whole agreement.

---

## Code Examples

### Overriding exactly one config leaf, with no env var (criterion 6)

The project forbids env-var config overrides as an invisible input. Textual substitution over the
shipped file preserves the `# GRADE:` comments — which `tests/provenance.rs` makes load-bearing —
and asserts that exactly one leaf moved.

```rust
// tests/determinism.rs — compiled and run, exits 1, stderr names tick 0
#[test]
fn the_binary_halts_on_a_liveness_violation_at_tick_zero() {
    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml")).unwrap();

    // Exactly one leaf overridden, ASSERTED — so a reworded config fails loudly
    // rather than silently running with liveness still off, which would make
    // this test pass for the wrong reason (it would just be criterion 1 again).
    let hits = base.matches("\nliveness_enabled = false\n").count();
    assert_eq!(hits, 1, "expected exactly one liveness_enabled leaf to override");
    let cfg = dir.path().join("liveness_on.toml");
    std::fs::write(&cfg, base.replace("\nliveness_enabled = false\n",
                                      "\nliveness_enabled = true\n")).unwrap();

    let assert = assert_cmd::Command::cargo_bin("sim").unwrap()
        .args(["--config", cfg.to_str().unwrap(),
               "--out", dir.path().join("run").to_str().unwrap()])
        .assert()
        .failure()
        .code(1);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(stderr.contains("tick 0"),   "stderr does not name tick 0: {stderr}");
    assert!(stderr.contains("liveness"), "stderr does not name the check: {stderr}");
    // TICK-06 at the message level, the runtime half of guard 7h.
    assert!(!stderr.contains(dir.path().to_str().unwrap()), "stderr carries a path");
}
```

**Measured stderr:**
```
INVARIANT VIOLATION: tick 0: liveness — 0 transactions recorded, at least 1 required; no posting, which is the violation
```
Exit code **1**. `[VERIFIED: ran it]`

**Why tick 0 and not a later tick:** `Books::new` clears the endowment postings, so tick 0's
journal is empty, `transactions_this_tick() == 0` (`src/books.rs:1345`) and `check_liveness`
fires on the first tick. The endowment cannot satisfy it — which is precisely the subtlety
CONTEXT.md flags, and it is closed by Phase 2's construction, not by anything Phase 3 does.
`[VERIFIED: reproduced end to end]`

**Alternative considered and rejected:** a `toml::to_string(&Params)` round-trip. `Params` derives
`Serialize`, so it works — but it strips every comment, and the comments carry the `CORE-11`
source grades. Textual substitution is the honest one-leaf override.

### The cross-process determinism test

```rust
#[test]
fn two_processes_at_one_seed_write_identical_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");
    let mut hashes = Vec::new();
    for name in ["a", "b"] {
        let out = dir.path().join(name);
        assert_cmd::Command::cargo_bin("sim").unwrap()
            .args(["--config", cfg, "--out", out.to_str().unwrap()])
            .assert().success();
        for f in ["ticks.csv", "events.jsonl"] {
            let bytes = std::fs::read(out.join(f)).unwrap();
            assert!(!bytes.is_empty(), "{f} is empty — this comparison would be vacuous");
            hashes.push(sim::config::config_hash(&bytes));   // sha2, already a dep
        }
    }
    assert_eq!(&hashes[0..2], &hashes[2..4]);
}
```
`[VERIFIED: passes; and the `is_empty` clause caught the real defect in Pitfall 8]`

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `serde_json` numbers via `ryu` | via **`zmij`** 1.0.23 | serde_json 1.x recent | Both shortest-round-trip; `csv` still uses `ryu`. No behaviour difference measured. |
| `schemars` 0.8 `schema_for!` returning `RootSchema` | 1.2.2 returns `Schema` with draft 2020-12 `$defs` | schemars 1.0, 2025-07 | Irrelevant here — rejected on the `serialize_with` finding, not the API |
| `csv` 1.3 | 1.4.0 (2025-10-17) | — | No change to the writer defaults verified here |
| `assert_cmd` 2.0 | 2.2.2, MSRV **1.85** | 2026-05-11 | Requires edition-2024-era toolchain; 1.94.1 satisfies it |

**Deprecated/outdated in the project's own documents, corrected here:**
- `research/STACK.md` and `CLAUDE.md`: *"map keys `BTreeMap`-sorted ⇒ byte-identical output"* —
  true for `Value::Object`, **false for a `HashMap` field**. See Pitfall 6.
- `research/ARCHITECTURE.md` line 903 and the roadmap's Phase 3 "Uses" list name `schemars` and
  `insta`. Both are rejected here on measured grounds.
- `research/ARCHITECTURE.md` Pattern 4's `PHASES` sketch uses `fn(&mut Ctx)` with no return and
  `&mut Rng`; both must change (Pattern 1).
- ROADMAP Phase 3 criterion 3's stated mechanism (shuffle + draw-count column) is **not
  sufficient**. See Pitfall 1. The criterion's *goal* is right; its stated mechanism needs one
  more column.

---

## Runtime State Inventory

Not applicable — Phase 3 is greenfield within an existing crate. It adds three modules and
rewrites `src/main.rs`; it renames nothing, migrates no stored data and registers nothing with
the OS. The one piece of *existing* state it invalidates is `tests/tracer_end_to_end.rs`'s
expectation of the Phase 1 tracer stdout (Pitfall 11), which is a code edit and not a data
migration.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `rustc` / `cargo` | everything | ✓ | 1.94.1 (e408947bf 2026-03-25) | — |
| `clippy`, `rustfmt` | lint gate | ✓ | via `rust-toolchain.toml` components | — |
| crates.io (index + download) | `cargo add csv serde_json assert_cmd tempfile` | ✓ | `cargo fetch` succeeded for all | — |
| `bash`, `grep`, `git ls-files` | `tests/lints.sh`, `tests/toolchain.sh` | ✓ | both scripts ran green | — |
| `python3` + `pandas` | **not required by Phase 3** | ✓ (3.0.5, installed here to verify read-back) | 3.0.5 | Phase 4 owns the Python environment |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

**One egress note for the planner:** `static.crates.io` returns **403** to a bare `curl` through
this session's proxy, but `cargo` fetches from it successfully. Do not conclude from a failed
`curl` that a crate is unavailable. `[VERIFIED: curl 403, cargo fetch 0]`

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `libtest` (`cargo test`), rustc 1.94.1; `proptest` 1.11.0 for properties; **new:** `assert_cmd` 2.2.2 + `tempfile` 3.27.0 for process-level tests |
| Config file | `Cargo.toml` `[dev-dependencies]`; `.proptest-regressions/` committed |
| Quick run command | `cargo test --locked --lib phases world log` |
| Full suite command | `cargo test --locked --all-targets && cargo test --locked --release --all-targets && bash tests/lints.sh && bash tests/toolchain.sh && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` |
| Measured runtime (this research pass, on the built Phase 3 tree) | debug tests **2.9 s** · release tests **18.3 s** (cold; includes the release rebuild) · `lints.sh` **4.6 s** · `toolchain.sh` **0.08 s** · clippy **0.26 s** warm |

**What Phase 3 adds beyond Phase 1-2's infrastructure — the whole list:**

1. **Two dev-dependencies** (`assert_cmd`, `tempfile`). This is the first phase that needs
   them; Phase 2's VALIDATION.md correctly notes Phase 2 added none. CI runs `--locked`, so the
   lockfile update is part of the same commit.
2. **A committed golden run directory** at `tests/golden/` (50 ticks — Pattern 8).
3. **A committed generated artifact** at `schema/schema.json`, in the `clippy.toml` mould.
4. **One new guard in `tests/lints.sh`** (`7f-agents`, three clauses — Pattern 7). The script's
   own summary line and its check-count prose must be updated in the same commit, since it
   currently says "ten source guards".
5. **No new test *runner*, no new framework, no new CI step.**

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| TICK-01 | `PHASES` runs the exact 9-name sequence; a tenth phase cannot be added without placing it | unit (in-module, `ALL_CHECKS` pattern) | `cargo test --locked --lib phases::order` | ❌ Wave 0 |
| TICK-01 | Each phase completes for all agents before the next | unit | `cargo test --locked --lib phases::order::each_phase_is_a_full_loop` (structural — assert `PhaseFn` takes `&mut Ctx` and no per-agent step exists) | ❌ Wave 0 |
| TICK-02 | Generated schema equals the committed file | integration | `cargo test --locked --test log_schema schema_matches_the_committed_file` | ❌ Wave 0 |
| TICK-02 | Drift test **fires** — mutation | integration (negative) | `bash tests/schema_drift_negative.sh` (perturb, expect fail, revert under `trap`) | ❌ Wave 0 |
| TICK-03 | `ticks.csv` header, column order and integer-only dtypes | integration | `cargo test --locked --test log_schema ticks_csv_is_flat_and_integer_only` | ❌ Wave 0 |
| TICK-03 | 3,650 rows, `\n` terminator, no CRLF, no empty field | integration | `cargo test --locked --test determinism the_run_directory_is_well_formed` | ❌ Wave 0 |
| TICK-04 | Every `Event` variant round-trips and appears in the schema | unit | `cargo test --locked --lib log::events` | ❌ Wave 0 |
| TICK-04 | Endowment events sum to `total_money_cents` | integration | `cargo test --locked --test determinism endowment_events_sum_to_the_money_stock` | ❌ Wave 0 |
| TICK-05 | `run_meta.json` carries seed, config hash, rustc | integration | `cargo test --locked --test determinism run_meta_carries_the_three_fields` | ❌ Wave 0 |
| TICK-06 | No path, hostname, PID or timestamp in any diffed file | integration | `cargo test --locked --test determinism the_exclusion_is_enforced_not_documented` | ❌ Wave 0 |
| TICK-06 | Halt message carries no environment (source half) | shell guard | `bash tests/lints.sh` (guard 7h, already exists; extend its file set to `src/log.rs`, `src/phases.rs`, `src/world.rs`) | ⚠️ extend |
| TICK-07 | `provenance.csv` exists with the exact 7-column header and 0 rows | integration | `cargo test --locked --test log_schema provenance_has_a_header_even_with_no_rows` | ❌ Wave 0 |
| TICK-08 | 3,650 empty ticks execute; invariants pass; run directory complete | integration | `cargo test --locked --release --test determinism the_empty_decade_runs` | ❌ Wave 0 |
| TICK-09 | Same seed → identical, in-process | integration | `cargo test --locked --test determinism same_seed_identical_in_process` | ❌ Wave 0 |
| TICK-09 | Same seed → identical, cross-process | integration (`assert_cmd`) | `cargo test --locked --test determinism two_processes_at_one_seed_write_identical_bytes` | ❌ Wave 0 |
| TICK-09 | Debug and release bytes agree | integration | `cargo test --locked --release --test determinism debug_and_release_agree` (or a CI step) | ❌ Wave 0 |
| TICK-10 | **Different seed → different `ticks.csv`** | integration | `cargo test --locked --test determinism different_seed_differs` | ❌ Wave 0 |
| TICK-10 | The counter-check has teeth — mutation | manual-then-recorded | Blank the `activation_digest` column, confirm `different_seed_differs` **fails**, revert. **Already performed in this research pass** — record the result in the plan's SUMMARY as Phase 2 did. | n/a |
| Criterion 6 | Binary exits non-zero, stderr names tick 0 | integration (`assert_cmd`) | `cargo test --locked --test determinism the_binary_halts_on_a_liveness_violation_at_tick_zero` | ❌ Wave 0 |
| Criterion 7 | `Household` / `Firm` carry no balance, expose no `set_cash` | shell guard (3 clauses) | `bash tests/lints.sh` (guard `7f-agents`) | ❌ Wave 0 |
| Criterion 7 | The guard **fires** on all three hazard shapes | shell guard fixtures | built into the guard via `assert_fires` / `assert_ignores` — **already mutation-proved in this research pass** | ❌ Wave 0 |
| Golden | 50-tick run reproduces byte-identically | integration | `cargo test --locked --test determinism the_golden_run_reproduces` | ❌ Wave 0 |

**Which are unit- vs property- vs integration-provable — the honest split.**

- **Unit** (`--lib`, no filesystem): the `PHASES` order triple, `Event`/`TickRow` wire shapes via
  `VecSink`, `schema_json()` self-consistency, `order_digest` sensitivity.
- **Integration** (`tests/`, writes to a `tempfile::tempdir`): everything about *files* —
  TICK-03, -05, -06, -08, -09, -10, criterion 6. These need a real run directory and, for
  TICK-09's cross-process half and criterion 6, a real process.
- **Property (`proptest`)**: **only one thing here genuinely earns a property test** —
  `order_digest` should map distinct permutations to distinct digests over a generated space of
  permutations. Everything else in this phase is a fixed table or a fixed file shape, where a
  property test would generate inputs the model never produces. **Do not add proptest cases for
  the log types**; the phase's risk is in the file bytes, not in an input domain. This is a
  deliberate deviation from "more property tests is better" and should be stated in the plan so
  the plan-checker does not read it as a gap.
- **Shell guard** (`tests/lints.sh`): criterion 7, and the extension of guard 7h's file set.

### Sampling Rate

- **Per task commit:** `cargo test --locked --lib phases world log` — sub-second.
- **Per wave merge:** `cargo test --locked --all-targets && cargo test --locked --release --all-targets`
  — the release run is not a duplicate: TICK-08's decade and the `debug_and_release_agree` claim
  are both about the release binary.
- **Phase gate:** the full six-step suite, matching CI exactly, before `/gsd-verify-work`.
- **Max feedback latency:** ~5 s warm.

### Wave 0 Gaps

- [ ] `Cargo.toml` — add `csv`, `serde_json`, `assert_cmd`, `tempfile`; commit the `Cargo.lock` update
- [ ] `src/world.rs` — `World`, `Household`, `Firm` (covers criterion 7's subject)
- [ ] `src/phases.rs` — `Ctx`, `PhaseId`, `PHASES`, `tick`, `run` + the in-module `order` tests (TICK-01)
- [ ] `src/log.rs` — `Sink`, `TickRow`, `Event`, `ProvenanceRow`, `RunWriter`, `schema_json` (TICK-02..07)
- [ ] `build.rs` — `SIM_RUSTC_VERSION` (TICK-05)
- [ ] `src/main.rs` — rewritten CLI; `--dump-schema` (TICK-05, criterion 6)
- [ ] `schema/schema.json` — generated and committed (TICK-02)
- [ ] `tests/determinism.rs` — TICK-06, -08, -09, -10, criterion 6, golden
- [ ] `tests/log_schema.rs` — TICK-02, -03, -07 drift and shape
- [ ] `tests/golden/` — committed 50-tick run directory
- [ ] `tests/lints.sh` — guard `7f-agents` (criterion 7) + guard 7h file-set extension + the check-count prose
- [ ] `tests/tracer_end_to_end.rs` — port `runs_end_to_end` and `different_seed_changes_the_draw`; keep the overflow tests

---

## Security Domain

`security_enforcement: true` in `.planning/config.json`. Phase 2's register is the model: this is
an in-process, single-threaded computation with **one new externally-visible surface** — the run
directory it writes.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No principal exists |
| V3 Session Management | no | No session |
| V4 Access Control | no | No subject to authorise |
| V5 Input Validation | **yes** | `--config` path and `--out` path are the two operator inputs; config content is already validated by `config::load` (`deny_unknown_fields`, `Params::validate`) |
| V6 Cryptography | **partial** | `sha2` is used for a *digest*, not a security control — the config hash and the activation digest are integrity/identity labels, not authentication. Do not describe them as tamper-proof |
| V12 File and Resources | **yes** | The run directory is created from an operator path joined only with fixed file names |
| V14 Configuration | **yes** | No env-var override anywhere; the config file plus `--seed` is the whole input |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via `--out` | Tampering | `--out` is joined only with fixed literal names (`ticks.csv`, …), never with config-derived content. This continues T-1-04's disposition from Phase 1, which `src/main.rs` already documents |
| Path traversal via config content into a filename | Tampering | No config value reaches a path. Assert it: `--out` is the only path source |
| Environment disclosure in a diffed log | Information Disclosure | TICK-06; enforced by the exclusion test (Pattern 5 clause 4) and by guard 7h's source half, extended to the three new files |
| Environment disclosure in a halt message | Information Disclosure | Guard 7h already covers `src/books.rs` and `src/invariants.rs`; **extend its file set** — a `Violation` rendered through a new `Display` in `log.rs` would be unguarded |
| A determinism test that passes vacuously | Tampering (of the evidence) | Non-empty assertion on every diffed file; the different-seed counter-check with a seed-sensitive column. Both measured to fail before they pass |
| A schema that describes bytes the writer does not produce | Tampering | Generate from the writer, not from a second derive (Pitfall 2) |
| Silent write failure (full disk) truncating a log | Denial of Service / Tampering | `sink.finish()` returns `io::Result` and is propagated; never rely on `csv::Writer`'s error-swallowing `Drop` (Pitfall 9) |
| A liveness gate left off permanently | Tampering | Already carried by T-02-02's three records; criterion 6 adds the process-level proof that turning it on halts |

**New threat IDs the planner should register (suggested, continuing the `T-03-xx` series):**
path handling for `--out`, environment in a diffed file, vacuous determinism evidence, schema/writer
divergence, silent truncation on write failure.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A `tests/golden/` directory of 50 ticks is the right substitute for `insta`, and the plan-checker will not read the absence of `insta` as a gap | Pattern 8 | Low — the golden file provides the same regression signal; only the review UX differs |
| A2 | The suggested `T-03-xx` threat IDs continue Phase 2's numbering convention | Security Domain | Low — cosmetic; the security agent authors the register |
| A3 | `schema_version` is a `const` in `src/log.rs` rather than a config leaf | Pitfall 12, Open Q1 | Low — but if the planner routes it to config, all five parts of the agreement move |
| A4 | pandas `EmptyDataError` on a zero-byte CSV is stable across pandas 3.x | Pitfall 4 | Low — measured on 3.0.5, which is the version `CLAUDE.md` pins for Phase 4 |
| A5 | `csv-core`'s `\n` default terminator will not change in a 0.1.x patch | Standard Stack | Low — read from source and pinned by `Cargo.lock`; a bump is a reviewed change |
| A6 | The run-directory naming convention (`runs/<id>/`) is `--out`-supplied, with no default beyond `runs/latest` | Pattern 5 | Low — Claude's discretion per CONTEXT.md; the probe used `runs/latest` |

---

## Open Questions

1. **Does the schema version belong in `src/log.rs` as a `const`, or in the config?**
   - What we know: CORE-10 requires every *simulation or economic* parameter in the config with
     no serde defaults, and carves out non-economic numerical constants as documented `const`
     items. A schema version is neither economic nor numerical-method — it is a wire-format
     label. Phase 1's carve-out precedent (`POW_FRAC_BITS`, `PPM_SCALE`, `MILLI_SCALE`) is the
     closest analogue.
   - What's unclear: whether it warrants a `GRADE: PROJECT` row in `config/PROVENANCE.md` the way
     those constants did.
   - Recommendation: `const` in `src/log.rs`, **plus** a `GRADE: PROJECT` provenance row stating
     why it is not configuration — matching the carve-out precedent exactly, and keeping the
     config leaf count at 41 so the five-part agreement is untouched.

2. **Does the per-firm panel carry books-derived columns redundantly?** (CONTEXT.md leaves this
   to discretion.)
   - What we know: a per-firm-tick panel at 20 firms × 3,650 ticks is 73,000 rows. At the ~55
     bytes/row measured for the 9-column tick series, a 10-column firm panel is roughly
     **4 MB** — well inside the "under 20 MB always-on" budget ARCHITECTURE.md sets.
   - What's unclear: whether Phase 3 should ship the panel at all, since it has no behavioural
     state to put in it (`Firm { price_cents }` is the whole struct).
   - Recommendation: **do not ship the firm panel in Phase 3.** The schema is the thing that must
     be right, and a panel with one meaningful column freezes a shape before there is anything to
     shape it around. Add it in Phase 9, when `expected_demand`, `price`, `wage_offer` and
     `last_sales` all exist — and log books-derived columns (`cash_cents`, `stock_units`)
     **redundantly** alongside them at that point, because the alternative is a join in every
     Python query and the redundancy is 4 MB.

3. **Should the halt path emit the offending tick's row before exiting?**
   - What we know: with `invariants` at position 8 and `log` at 9, the violating tick is never
     logged, and a halted run's `ticks.csv` was measured at 0 bytes.
   - What's unclear: whether an extra row from a tick that failed its check is a useful diagnostic
     or a corrupt row in an otherwise-clean series.
   - Recommendation: write the header eagerly regardless (so the file is always openable), and
     emit the offending row **into `events.jsonl` as a `Violation` event**, not into `ticks.csv`.
     That keeps the tick series a series of *passed* ticks while preserving the evidence. Worth a
     planner decision; either is defensible.

4. **Is `Event::Posted` (a nested `Posting`) wanted at all in Phase 3?**
   - What we know: the probe's one-level flatten produced `posting.*` fields in **alphabetical**
     order, because the nested object goes through the `BTreeMap`-backed `Value` rather than the
     text scanner. So a nested event loses declaration order in the schema, and
     `pd.read_json(lines=True)` gives a dict-valued column.
   - Recommendation: **do not nest.** Either omit `Posted` from Phase 3 entirely (the journal is
     per-tick and cleared; ARCHITECTURE.md makes a full journal dump an opt-in config flag), or
     flatten the posting's fields into the variant. Freezing a nested shape into
     `schema/schema.json` is exactly the "costly to change from Phase 3 onward" decision
     CONTEXT.md warns about.

---

## Sources

### Primary (HIGH confidence) — compiled and executed this session on rustc 1.94.1

- A **full working Phase 3 implementation** built against a copy of this repository:
  `src/world.rs` (40 lines), `src/phases.rs` (162), `src/log.rs` (270), `build.rs`, rewritten
  `src/main.rs`. 3,650 ticks run in debug and release; run directories produced, diffed, hashed
  and read back with pandas 3.0.5.
- **Failing designs, run deliberately**: shuffle-without-digest (byte-identical across seeds);
  `schemars` against a `serialize_with` field; eager header without `has_headers(false)` (double
  header); zero-byte artifacts passing a hash comparison; a money-typed-field guard pattern firing
  on a legitimate local binding.
- **Mutation proofs**: 2 on the schema drift test, 3 on guard `7f-agents`, 1 on the different-seed
  counter-check. Each broke the thing the check exists to catch, confirmed the check fails, and
  reverted.
- **Crate sources, extracted and read**: `csv-core-0.1.13/src/writer.rs:24` (terminator default),
  `csv-1.4.0/src/writer.rs:293-322`, `serde_json-1.0.151/src/map.rs:1-36`,
  `assert_cmd-2.2.2/src/lib.rs:115-119` and its `Cargo.toml`.
- **In-repo sources read this session**: `src/books.rs` (1-420, 475-620, 1109-1400),
  `src/invariants.rs` (380-540, 1500-1610), `src/ids.rs` (all), `src/rng.rs` (1-100, 180-380),
  `src/config.rs` (416-560), `src/lib.rs`, `src/main.rs`, `src/money.rs:50-60`,
  `tests/lints.sh` (628-700 and structure), `tests/toolchain.sh` (all),
  `tests/numeric_det.rs:91, 180-215`, `config/baseline.toml` (tail).
- **crates.io API** (queried 2026-08-31) for every version number and publication date.
- **`gsd-tools query package-legitimacy check --ecosystem crates`** for all seven candidate
  crates.

### Secondary (MEDIUM confidence)

- `.planning/research/ARCHITECTURE.md` Patterns 4-6 and § The Sim/Analysis Boundary — the design
  this phase implements; **three of its specifics are corrected above** on measured grounds.
- `.planning/research/SUMMARY.md` § Correctness Constraints — carried forward; constraint 5's
  per-tick draw-count series is necessary but **not sufficient** (Pitfall 1).
- `.planning/phases/02-books-journal-and-invariants/02-RESEARCH.md` and `02-VALIDATION.md` — the
  evidentiary and validation-section shape this document matches.

### Tertiary (LOW confidence)

None. Nothing in this document rests on a web search; `docs.rs` and `doc.rust-lang.org` were not
consulted and were not needed.

**Could not verify:**
- Nothing material. The one thing not exercised is a *write failure* (full disk) confirming that
  `csv::Writer`'s `Drop` silently swallows it — I verified the `Drop` **does** flush on a healthy
  filesystem and read that its error is discarded, but did not force an ENOSPC. Pitfall 9's
  recommendation (`finish()` returning `io::Result`) is correct either way. `[ASSUMED: the
  error-swallowing half]`

---

## Metadata

**Confidence breakdown:**

| Area | Level | Reason |
|------|-------|--------|
| Standard stack | HIGH | All four crates added, fetched, compiled, run; `toolchain.sh` and clippy green afterwards; versions from the crates.io API |
| Architecture | HIGH | The whole phase was built and run end to end, including the halt path and the cross-process test |
| Byte-identical output | HIGH | Terminators, header, field order, number formatting, map ordering and float rendering all measured, and read back through pandas 3.0.5 |
| Vacuous-reproducibility counter-check | HIGH | The prescribed design was built and **observed to fail**; the fix was built and observed to pass |
| Schema generation | HIGH | schemars' divergence reproduced on this repo's own wire shape; the alternative built and mutation-proved twice |
| Guard 7f extension | HIGH | Written, run clean, mutation-proved on three hazard shapes, with the `assert_ignores` false positive found and fixed |
| Golden vs `insta` | MEDIUM-HIGH | Dependency weight and diff sizes measured; the review-workflow preference is a judgement |
| Firm-panel question | MEDIUM | Size estimate derived from the measured tick-row byte cost, not from a built panel |

**Research date:** 2026-08-31
**Valid until:** 2026-09-30 — the crate versions are pinned by `Cargo.lock` and the toolchain by
`rust-toolchain.toml`, so the findings do not decay; the 30-day figure is for the crates.io
version numbers only.
