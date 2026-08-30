# Architecture Research

**Domain:** Deterministic agent-based macroeconomic simulation (Lengnick / BAM class) in Rust, with a Python analysis boundary
**Researched:** 2026-08-30
**Confidence:** HIGH — every Rust pattern below was compiled and executed locally on `rustc 1.94.1` before being written down; crate versions come from the crates.io registry API; the determinism and assertion findings were cross-checked across independent sources.

---

## Executive Answer

Six decisions carry this architecture. Everything else follows from them.

1. **Agents hold no money and no goods.** A single `Books` module owns every cent and every unit. `Household` and `Firm` are behavioural state only. This is the move that dissolves the two-mutable-borrows problem, makes conservation a local check instead of a whole-world scan, and makes "zero-sum trade" structurally true rather than hopefully true.
2. **`FirmId` is `{ slot, gen }`.** A firm's slot is a `Vec<Firm>` index; the generation counter increments on respawn. Lookup returns `Option`, so a stale ID is a typed miss, never a silently-wrong firm. The pair is also the per-agent log identity, so a firm's whole life is one `(slot, gen)` group in Python.
3. **The journal is a per-tick buffer, not an append-forever log.** Every money/goods movement appends a `Posting`. The invariant checker reads that buffer to name the offending transaction, then it is cleared. Writing it to disk is a config flag. This is what makes "name the transaction" affordable at ~10⁶ postings per decade-long run.
4. **The tick is a `const` array of named function pointers.** The array *is* the ordering, one unit test asserts the exact name sequence, and invariants and logging are phases inside it — so they cannot be accidentally skipped.
5. **`Ctx` carries the cross-cutting concerns.** `Ctx { world, rng, params, sink, tick }`. Provenance, RNG and config never appear in an individual decision function's signature.
6. **`rand_chacha::ChaCha8Rng`, never `StdRng`, and hand-rolled sampling.** Verified: `StdRng` explicitly opts out of reproducibility, and `rand`'s distribution algorithms are not value-stable across versions. Byte-identical logs cannot be built on either.

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                        RUNNER  (main.rs)                              │
│   parse CLI → load config → seed RNG → build World → drive N ticks    │
└───────────────────────────────┬──────────────────────────────────────┘
                                │  Ctx { world, rng, params, sink, tick }
┌───────────────────────────────▼──────────────────────────────────────┐
│                    TICK PIPELINE  (phases/mod.rs)                     │
│   const PHASES: [(&str, fn(&mut Ctx)); 9]  ← the array IS the order   │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌─────┐ ┌────────┐ ┌────────┐      │
│  │planning│→│ labour │→│producti│→│wages│→│ goods  │→│account │→ ...  │
│  └────────┘ └────────┘ └────────┘ └─────┘ └────────┘ └────────┘      │
│         ... →  bankruptcy  →  invariants  →  log                      │
└───────┬──────────────────────┬─────────────────────┬─────────────────┘
        │ reads/writes         │ every value move    │ every decision
┌───────▼──────────┐  ┌────────▼───────────┐  ┌──────▼───────────────┐
│      WORLD       │  │       BOOKS        │  │      LOG SINK        │
│ Vec<Household>   │  │ cash: per-account  │  │ trait Sink           │
│ Vec<Firm> (arena)│  │ stock: per-(acct,  │  │  ├ NullSink (tests)  │
│ Ownership (rel)  │  │        good)       │  │  ├ VecSink  (tests)  │
│ GoodsTable       │  │ created/destroyed  │  │  └ RunWriter (disk)  │
│ tick             │  │ journal: Vec<Post> │  └──────────┬───────────┘
└──────────────────┘  │ transfer()/settle()│             │
                      └────────┬───────────┘             │
                               │ journal                 │
                      ┌────────▼────────┐                │
                      │   INVARIANTS    │                │
                      │ check() → Result│                │
                      │ halts + names   │                │
                      │ tick/agent/txn  │                │
                      └─────────────────┘                │
                                                         ▼
                      ┌──────────────────────────────────────────────┐
                      │  runs/<run_id>/  manifest.json               │
                      │                  tick_series.csv             │
                      │                  firm_panel.csv              │
                      │                  events.jsonl                │
                      │                  decisions.jsonl             │
                      └───────────────────┬──────────────────────────┘
                       schema/schema.json │ (generated from Rust,
                       (committed)        │  committed, drift = test failure)
                      ┌───────────────────▼──────────────────────────┐
                      │  PYTHON ACCEPTANCE HARNESS  (analysis/)      │
                      └──────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Owns | Never |
|-----------|----------------|------|-------|
| `money` | `Cents(i64)` newtype, checked arithmetic | the money type | knows about agents |
| `ids` | `HouseholdId`, `FirmSlot`, `FirmId{slot,gen}`, `GoodId`, `Account` | identity | knows about balances |
| `config` | `Params` from TOML, config hash | every tunable number | contains logic |
| `rng` | `Rng` wrapper over `ChaCha8Rng` + hand-rolled samplers | the only randomness source | is cloned or forked |
| `world` | `Vec<Household>`, `Vec<Firm>` arena, `Ctx`, ID→entity resolution | behavioural agent state | holds cash or stock |
| `books` | **All** cash and **all** goods stock; `transfer`, `settle`, `produce`, `consume`; per-tick journal | value | contains economic rules |
| `goods` | `GoodsTable`, `Recipe` | the goods catalogue | is an enum |
| `ownership` | edge list + both-direction index | the ownership relation | is a field on `Firm` |
| `invariants` | four checks + journal bisection to name the offending posting | correctness | mutates anything |
| `log` | `Sink` trait, record types, `schemars` schema, run directory writer | the wire format | is called from `books` directly |
| `phases/*` | economic rules, one file per tick phase | behaviour | touches balances except via `books` |

---

## Recommended Project Structure

```
Cargo.toml
config/
  default.toml                # every parameter, none hardcoded in logic
schema/
  schema.json                 # GENERATED from Rust types, COMMITTED, read by Python
src/
  main.rs                     # CLI, run-dir creation, wiring, exit codes
  config.rs                   # Params, TOML load, blake3 config hash
  money.rs                    # Cents
  ids.rs                      # HouseholdId, FirmSlot, FirmId, GoodId, Entity, Account
  rng.rs                      # Rng over ChaCha8Rng + sample_distinct, uniform_below
  world.rs                    # World, Household, Firm, firm arena, Ctx
  books.rs                    # Books: cash + stock + journal; transfer/settle/produce/consume
  goods.rs                    # GoodsTable, Good, Recipe
  ownership.rs                # Ownership relation, owners_of / holdings_of
  invariants.rs               # check(), Violation, locate_breaking_posting()
  log/
    mod.rs                    # Sink trait, RunWriter, manifest
    schema.rs                 # every record type, #[derive(Serialize, JsonSchema)]
    writers.rs                # csv + jsonl writers
  phases/
    mod.rs                    # PHASES table + tick()
    planning.rs               # expectation, price rule, wage rule (weekly, staggered)
    labour.rs                 # decentralised matching, bounded sampling
    production.rs             # recipe → output
    wages.rs                  # contracted wage payment
    goods_market.rs           # bounded sampling, cheapest-first, stockout fallthrough
    accounting.rs             # profit, working-capital buffer, dividends
    bankruptcy.rs             # release, residual, kill slot, respawn at gen+1
tests/
  determinism.rs              # same seed → byte-identical run dir
  phase_order.rs              # asserts PHASES names in exact order
  golden/                     # committed 50-tick run, read by BOTH Rust and Python
analysis/
  acceptance.py               # section-7 criteria
  schema.py                   # loads schema/schema.json, validates
  charts.py
```

### Structure Rationale

- **`books.rs` sits beside `world.rs`, not inside it.** Two peers under one `World` owner. This keeps "value" and "behaviour" separately reviewable and makes the invariant module's dependency (`&Books`, not `&World`) narrow.
- **`phases/` is one file per tick phase.** The file list is the tick order. A new phase means a new file *and* a new line in the `PHASES` array *and* a change to the order test — three places, which is the point.
- **`schema/` is a top-level directory, not `src/`.** It is a build artifact that both languages consume; putting it under `src/` implies Rust owns it at runtime, which it does not.
- **`config/default.toml` ships in-repo.** Runs record the resolved config *and its hash* in the manifest, so a log file is self-describing.

---

## Architectural Patterns

### Pattern 1: The Ledger owns all value — agents own none

**What:** `Household` and `Firm` have no `cash` field and no `inventory` field. `Books` holds parallel arrays keyed by account and by `(account, good)`. The only public mutators are `transfer`, `settle`, `produce`, `consume`, each returning `Result`.

**When to use:** Whenever an invariant spans two collections. Here it spans `Vec<Household>` and `Vec<Firm>`, which is exactly the case that would otherwise need two simultaneous mutable borrows.

**Trade-offs:**
- **+** The two-mutable-borrows problem for money simply does not arise — both legs live in one owner.
- **+** Money conservation is a one-line check against a cached total, not a two-vector reduction.
- **+** "Zero-sum trade" becomes a property of the API rather than a property you audit.
- **+** Every value movement is funnelled through ~4 functions, which is where the journal, the provenance and the error handling live.
- **−** An extra indirection to read a balance (`books.cash(acct)` rather than `firm.cash`). At 220 agents this is free.
- **−** You must resist the temptation to "just add a cash field for convenience". Enforce with a comment and a review rule.

**Example** (compiled and run on `rustc 1.94.1`):

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Account { Household(HouseholdId), Firm(FirmId) }

#[derive(Debug)]
pub enum LedgerError {
    InsufficientFunds { account: Account, have: Cents, want: Cents },
    NegativeAmount(Cents),
    DeadFirm(FirmId),
}

pub struct Books {
    household_cash: Vec<Cents>,
    firm_cash:      Vec<Cents>,   // indexed by FirmSlot
    firm_gen:       Vec<u32>,     // generation guard, mirrors the arena
    total_money:    Cents,        // set once at t=0, never changes
    journal:        Vec<Posting>, // CLEARED each tick after the invariant check
}

impl Books {
    /// The ONLY way money moves. Both legs land or neither does.
    pub fn transfer(&mut self, tick: u32, from: Account, to: Account,
                    amount: Cents, reason: Reason) -> Result<(), LedgerError> {
        if amount.0 < 0 { return Err(LedgerError::NegativeAmount(amount)); }
        let have = self.cash(from)?;          // also validates `from` is live
        let _    = self.cash(to)?;            // also validates `to`  is live
        if have < amount {
            return Err(LedgerError::InsufficientFunds { account: from, have, want: amount });
        }
        self.add(from, Cents(-amount.0))?;
        self.add(to,   amount)?;
        self.journal.push(Posting { tick, from, to, amount, reason });
        debug_assert_eq!(self.sum_cash(), self.total_money);  // cheap dev-only tripwire
        Ok(())
    }
}
```

Note the payoff on a specific project decision: PROJECT.md says *"Bankruptcy respawn redraws when the sampled owner cannot fund a firm."* That edge case is not special-cased anywhere — it is `Err(InsufficientFunds)` from `transfer`, and the respawn loop redraws on `Err`. The ledger's error type **is** the mechanism.

`settle` does the same for a sale, atomically, in one call:

```rust
pub struct Trade {
    pub buyer: Account, pub seller: Account,
    pub good: GoodId, pub qty: u64, pub unit_price: Cents,
}

impl Books {
    pub fn settle(&mut self, tick: u32, t: Trade) -> Result<(), LedgerError> {
        let total = Cents(t.unit_price.0 * t.qty as i64);
        self.require_stock(t.seller, t.good, t.qty)?;   // check both legs BEFORE
        self.cash_at_least(t.buyer, total)?;            // mutating either one
        self.transfer(tick, t.buyer, t.seller, total, Reason::Purchase)?;
        self.move_stock(tick, t.seller, t.buyer, t.good, t.qty);
        Ok(())
    }
}
```

**Goods are not conserved — they are created and destroyed.** `production` mints units and household consumption burns them. So `Books` also holds `created: Vec<u64>` and `destroyed: Vec<u64>` per good, and the "goods conservation" invariant is really the stock–flow identity `created[g] - destroyed[g] - Σ stock[·][g] == 0`. Model consumption as an explicit `books.consume(household, good, qty)` call even if in this milestone purchase and consumption happen in the same instant — it keeps the identity uniform and survives the arrival of household food inventories.

### Pattern 2: Disjoint field borrows for "iterate firms while mutating households"

**What:** The borrow checker refuses `self.firms.iter_mut()` while `self.households[i]` is touched *through `self`*. It accepts it if you first destructure `self` into disjoint field borrows. This is the everyday answer and it needs no `unsafe`, no `RefCell`, no cloning.

**When to use:** Always, for the household↔firm case — which in this model is *most* of the tick.

**Example** (compiled and run):

```rust
impl World {
    pub fn goods_market(&mut self) {
        // One destructure at the top of the phase. The borrow checker now
        // knows these three are disjoint and lets you mutate all of them.
        let firms      = &mut self.firms;
        let households = &mut self.households;
        let books      = &mut self.books;
        let tick       = self.tick;

        for hi in 0..households.len() {
            let hid = HouseholdId(hi as u32);
            for fi in 0..firms.len() {                    // index loop, short borrows
                let f = &mut firms[fi];
                if !f.alive || books.stock(Account::Firm(f.id()), FOOD) == 0 { continue; }
                if books.settle(tick, Trade { buyer: Account::Household(hid),
                                              seller: Account::Firm(f.id()),
                                              good: FOOD, qty: 1,
                                              unit_price: f.price }).is_ok() {
                    households[hi].last_purchase_tick = tick;
                }
                break;
            }
        }
    }
}
```

The three techniques, ranked by when to reach for them:

| Technique | Use for | Notes |
|-----------|---------|-------|
| **Disjoint field borrows** | firm ↔ household (99% of this model) | Zero cost, zero ceremony. Destructure once per phase. |
| **Index loops with short borrows** | mutating one agent at a time inside a loop | `for i in 0..v.len()` then `&mut v[i]` — the borrow dies each iteration. |
| **`slice::get_disjoint_mut([a, b])`** | firm ↔ firm within one `Vec` | Stable on 1.94; returns `Result<[&mut T; N], _>`, `Err` on overlap. Arrives with input–output production chains. |

`split_at_mut` is the older idiom for the same job and appears in most search results, but `get_disjoint_mut` expresses the intent directly and does the overlap check for you — prefer it. Verified compiling:

```rust
pub fn firm_to_firm(firms: &mut Vec<Firm>, a: usize, b: usize) {
    if let Ok([x, y]) = firms.get_disjoint_mut([a, b]) {
        x.pending_out += 1;
        y.pending_in  += 1;
    }
}
```

**Intent collection is the fourth tool, and it is not the default.** Use it *only* where the read set and the write set genuinely overlap or where fairness requires a global view before any mutation:

- **Labour market** — collect `Vec<Application>` from all seekers first, then let each firm accept in a deterministic order. Without this, an early-indexed household systematically wins, which is a determinism-preserving but economically wrong bias.
- **Bankruptcy** — mark-then-sweep. Collect `Vec<FirmSlot>` to kill, *then* kill them, because killing while iterating invalidates the loop.

Do not apply intent collection globally. It roughly doubles the line count of every phase and buys nothing where disjoint field borrows already work.

### Pattern 3: Generational firm IDs over a fixed-size arena

**What:** `FirmId { slot: FirmSlot, gen: u32 }`. `slot` indexes `Vec<Firm>`; `gen` increments each time the slot is reused by a respawn. Resolution checks the generation and returns `Option`.

**When to use:** Any entity that can be removed and replaced while other entities hold references to it. Here: firms (households hold `employer: Option<FirmId>`; `Ownership` holds `owned: FirmId`).

**Why this and not the alternatives:**

| Approach | Verdict | Reason |
|----------|---------|--------|
| Raw `usize` index | **Fatal** | Bankruptcy respawn reuses index 7; household 42's `employer = 7` now silently points at a brand-new firm. This is the single worst latent bug in the design and it produces *plausible* output. |
| `swap_remove` + remap | **No** | The remap table has to be applied to households, ownership edges and every in-flight log record in the same tick. One missed site is a silent corruption, and the mapping breaks per-agent log reconstruction outright. |
| Tombstoning (`alive: bool`, never reuse) | **Partial** | Solves aliasing but grows the vector forever and makes "the 20 firms" a filter rather than a fact. Acceptable, but it loses the O(1) fixed-size arena. |
| `slotmap` crate (v1.1.1) | **Good, but not here** | It is the right answer for a dynamically-sized population, and `facorread/rust-agent-based-models` uses it exactly this way. But it brings a free list, capacity growth and version-wraparound semantics you do not need for a *constant* 20-firm arena, and it makes key-issuance order part of your determinism contract via someone else's code. Revisit if endogenous firm entry lands. |
| **Hand-rolled `{slot, gen}` over a fixed `Vec`** | **Recommended** | ~40 lines. Keeps the committed "IDs are indices" layout literally true. Slot count is constant, so no free list. Fully under your determinism control. |

**Example** (compiled and run):

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct FirmSlot(pub u32);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct FirmId { pub slot: FirmSlot, pub gen: u32 }

impl World {
    /// Stale ID → None. A dead firm is a typed miss, never a wrong firm.
    pub fn firm(&self, id: FirmId) -> Option<&Firm> {
        let f = self.firms.get(id.slot.0 as usize)?;
        (f.alive && f.gen == id.gen).then_some(f)
    }
    pub fn firm_mut(&mut self, id: FirmId) -> Option<&mut Firm> {
        let f = self.firms.get_mut(id.slot.0 as usize)?;
        (f.alive && f.gen == id.gen).then_some(f)
    }

    /// Bankruptcy: same slot, next generation. All old FirmIds now miss.
    pub fn respawn(&mut self, slot: FirmSlot, seed: Firm) -> FirmId {
        let f = &mut self.firms[slot.0 as usize];
        let gen = f.gen + 1;
        *f = Firm { gen, alive: true, ..seed };
        self.books.set_firm_gen(slot, gen);   // arena and books stay in lockstep
        FirmId { slot, gen }
    }
}
```

**Log reconstructability across a firm's death.** Every log record emits `firm_slot` and `firm_gen` as two columns. Python groups by the pair. A firm's entire life — birth, every price decision, every hire, its bankruptcy — is exactly one `(slot, gen)` group, and the successor in the same slot is a different group. No ID remapping, no gaps, no ambiguity. This is the property that raw indices and swap-remove both destroy.

**Households** need no generation in this milestone (they are never removed), so `HouseholdId(u32)` is a plain newtype index. Keep it a *distinct type* from `FirmSlot` so the two can never be crossed. If demographics ever arrive, households get the identical `{slot, gen}` treatment and nothing else changes.

`typed-index-collections` (v3.5.0) or `index_vec` (v0.1.4) would give you `TiVec<FirmSlot, Firm>` so the *container* itself refuses a wrong-typed index. Worth adopting if you find yourself writing `as usize` casts; not required, since the `world.firm(id)` accessor is already the only entry point.

### Pattern 4: The tick as a `const` array of named phases

**What:** A compile-time array pairing a phase name with a function pointer. The array is the tick order; a test asserts the exact name sequence.

**When to use:** Whenever the ordering *is* the specification — which is precisely this project's core value claim.

**Trade-offs:**
- **+** The order is one greppable literal, not spread across a 40-line function.
- **+** The name is available at runtime for per-phase tracing and per-phase timing without a second list to keep in sync.
- **+** A unit test over the names makes accidental reordering a red build, which is the whole requirement.
- **+** `invariants` and `log` are *phases*, so they cannot be skipped or reordered relative to the economics.
- **−** `fn` pointers are not inlined. At 220 agents × 3,650 ticks × 9 phases this is unmeasurable.
- **−** Phases cannot carry per-phase state. They should not; state belongs in `World`.

Rejected: **a trait-object pipeline** (`Vec<Box<dyn Phase>>`). It adds a registration step, which is the exact affordance that lets ordering drift at runtime, and it obscures the order behind a builder. Rejected: **nine bare sequential calls in `fn tick()`** — simpler, but then the phase names used for logging live in a second place and silently drift from the calls.

**Example** (compiled and run):

```rust
pub struct Ctx<'a> {
    pub w:      &'a mut World,
    pub rng:    &'a mut Rng,
    pub p:      &'a Params,
    pub sink:   &'a mut dyn Sink,
}

pub type Phase = fn(&mut Ctx);

pub const PHASES: [(&str, Phase); 9] = [
    ("firm_planning",   planning::run),
    ("labour_market",   labour::run),
    ("production",      production::run),
    ("wages",           wages::run),
    ("goods_market",    goods_market::run),
    ("firm_accounting", accounting::run),
    ("bankruptcy",      bankruptcy::run),
    ("invariants",      invariants::run),   // in the pipeline, not beside it
    ("log",             logging::run),
];

pub fn tick(w: &mut World, rng: &mut Rng, p: &Params, sink: &mut dyn Sink) {
    for (name, phase) in PHASES.iter() {
        let mut c = Ctx { w, rng, p, sink };
        phase(&mut c);
        trace_phase(*name, c.w.tick);
    }
    w.books.clear_journal();   // journal is per-tick, see Pattern 5
    w.tick += 1;
}
```

```rust
// tests/phase_order.rs — the guard against accidental reordering
#[test]
fn tick_order_is_exactly_the_brief() {
    let names: Vec<&str> = PHASES.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, vec![
        "firm_planning", "labour_market", "production", "wages",
        "goods_market", "firm_accounting", "bankruptcy", "invariants", "log",
    ]);
}
```

Each phase completes for all agents before the next begins **by construction** — a phase function is a full loop over the relevant population, and the next phase does not start until it returns. There is no per-agent `step()` anywhere in the design, which is the structural difference from a naive object-oriented ABM.

### Pattern 5: Invariants as a pipeline phase with journal bisection

**What:** `invariants::run` is phase 8. It compares four identities and, on failure, walks the current tick's journal to find the exact posting where the identity broke.

**Where they live:** A dedicated `invariants` module, called as a phase. Not a wrapper around `tick()` (a wrapper cannot see the journal at the right granularity, and it puts the check outside the thing it certifies). Not scattered `assert!`s in `books` (those catch a violation but cannot name the *tick*).

**Release-build requirement — verified.** `debug_assert!` is compiled out of optimised builds unless `-C debug-assertions` is passed; `assert!` always runs. Therefore:

- The four invariants must **not** be `debug_assert!`.
- Prefer `Result<(), Violation>` + explicit diagnostic + `process::exit(1)` over `assert!`. You want a formatted line naming tick, agent and transaction plus a nonzero exit code for the harness — not a panic backtrace.
- `debug_assert!` remains useful as a *second, finer* tripwire inside `Books::transfer` (as in Pattern 1). It fires on the offending call in dev; the phase-level check with journal bisection covers release.

**Cost:** four reductions over 220 accounts, 3,650 times = under a million integer adds for a decade. Free. Run them unconditionally, every tick, in release.

```rust
#[derive(Debug)]
pub enum Violation {
    MoneySupply  { tick: u32, expected: Cents, actual: Cents, culprit: Option<Posting> },
    GoodsIdentity{ tick: u32, good: GoodId, created: u64, destroyed: u64, in_stock: u64 },
    NegativeCash { tick: u32, account: Account, balance: Cents, culprit: Option<Posting> },
    NonZeroSum   { tick: u32, posting: Posting, debit: Cents, credit: Cents },
}

pub fn run(c: &mut Ctx) {
    if let Err(v) = check(&c.w.books, c.w.tick) {
        eprintln!("INVARIANT VIOLATION\n{v:#?}");
        c.sink.flush();                 // never lose the ticks that led here
        std::process::exit(1);          // halt immediately, per the brief
    }
}

/// Replays this tick's journal to find the first posting after which the
/// money identity no longer held. This is how a violation names a transaction.
fn locate(books: &Books, opening: Cents) -> Option<Posting> {
    let mut running = opening;
    for p in books.journal() {
        // a well-formed posting is zero-sum, so `running` must be invariant
        if running != opening { return Some(p.clone()); }
        running = books.replay_one(running, p);
    }
    None
}
```

**Why the journal is per-tick.** A decade-long run produces on the order of 10⁶ postings (≈730k purchases + ≈35k wage payments + dividends + bankruptcy transfers). Holding all of them costs a few hundred MB and buys nothing, because a violation is always located *within the tick it occurred*. So: accumulate during the tick, check, name the culprit, clear. Writing the journal to disk is `[logging] journal = true` in the config, off by default, on when you are hunting.

### Pattern 6: Determinism as a set of prohibitions

The determinism contract is mostly a list of things you must not do. Two of them are non-obvious and were verified:

1. **Do not use `StdRng`.** The `rand` documentation states that `StdRng` opts out of reproducibility guarantees — future library versions may swap the internal generator, and output can differ by architecture. Use `rand_chacha::ChaCha8Rng` (v0.10.0), which is deterministic, portable, and tested against reference vectors.
2. **Do not rely on `rand`'s distributions or `SliceRandom` for behaviour-defining draws.** The `Uniform` integer and float sampling algorithms have already been changed once, breaking value stability (`rust-random/rand` issue #786). Anything whose *exact sequence* is part of your byte-identical-log contract should be hand-rolled on top of `next_u64()`. In this model that means the bounded firm sampling in both markets — a ~15-line `sample_distinct(n, k)` in `rng.rs`, which you want to own anyway because "which firms does a household see" is an economic parameter, not an implementation detail.

Plus the standard ones: pin `rand` and `rand_chacha` with `=` in `Cargo.toml`; single-threaded; no `HashMap`/`HashSet` iteration on any path that affects behaviour (use `BTreeMap`/`BTreeSet`/`Vec` — the `Ownership` sketch below does this deliberately); no float anywhere in money, prices, wages or balances; one RNG instance, never cloned or forked; record the seed in the manifest.

---

## The Three Forward-Compatibility Mandates

### "Goods are data, not code"

**Structure:** a `GoodsTable` — a `Vec<Good>` indexed by `GoodId(u16)` — loaded from config, where each `Good` carries a `Recipe`. In this milestone the table has one row and the recipe has an empty `inputs` vector.

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct GoodId(pub u16);

#[derive(Clone, Debug)]
pub struct Recipe {
    pub labour_per_unit: u32,
    pub inputs: Vec<(GoodId, u32)>,   // EMPTY in v1 — the extension point
}

#[derive(Clone, Debug)]
pub struct Good { pub id: GoodId, pub name: String, pub recipe: Recipe }

pub struct GoodsTable { goods: Vec<Good> }
impl GoodsTable {
    pub fn get(&self, g: GoodId) -> &Good { &self.goods[g.0 as usize] }
    pub fn iter(&self) -> impl Iterator<Item = &Good> { self.goods.iter() }
}
```

**How it avoids the one-variant-enum trap.** The trap is not the enum declaration — it is that `match good { Good::Food => ... }` appears in fifteen call sites, each of which becomes a non-exhaustive match the day a second good arrives, and each of which has to be redesigned rather than extended. With a table:

- Stock is `stock[account][good_id]`, already a two-dimensional lookup. A second good is a second column, not a new code path.
- Production is `for (input, qty) in &recipe.inputs { consume(...) }` — a loop over an empty vector today, a real loop tomorrow. The *shape* of the production code is already correct.
- The goods identity invariant already iterates `goods.iter()`, so it covers good #2 for free.
- Prices are `Vec<Cents>` per firm indexed by `GoodId`, not a scalar `firm.price`.

The discipline that makes this real: **write `for g in goods.iter()` even when you know there is exactly one good.** A single hardcoded `FOOD` constant is fine as a config-resolved value in phase code; a hardcoded `if good == FOOD` branch is not.

One concession worth making explicitly: `firm.produces: GoodId` (a firm makes one good) is fine for now. Multi-product firms are a different modelling decision, not a data-layout one.

### "Ownership is a relation, not a field"

**Structure:** a dedicated `ownership` module holding an **edge list plus two indices**. Not `Firm.owner: HouseholdId`.

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Entity { Household(HouseholdId), Firm(FirmId) }   // ← firm-owns-firm is expressible NOW

#[derive(Clone, Debug)]
pub struct Share { pub owner: Entity, pub owned: FirmId, pub bps: u32 }  // basis points

#[derive(Default)]
pub struct Ownership {
    edges:    Vec<Share>,
    by_owned: BTreeMap<FirmId, Vec<usize>>,   // BTree, not Hash — iteration order is behaviour
    by_owner: BTreeMap<Entity, Vec<usize>>,
}

impl Ownership {
    pub fn owners_of(&self, f: FirmId) -> impl Iterator<Item = &Share> {
        self.by_owned.get(&f).into_iter().flatten().map(move |&i| &self.edges[i])
    }
    pub fn holdings_of(&self, e: Entity) -> impl Iterator<Item = &Share> {
        self.by_owner.get(&e).into_iter().flatten().map(move |&i| &self.edges[i])
    }
}
```

(Compiled and run.)

**Bidirectional queryability without references:** the two `BTreeMap<_, Vec<usize>>` indices are the adjacency tables; the `Vec<Share>` is the single source of truth. Both directions are O(degree). The indices are derived — rebuild them from `edges` in a `reindex()` and call it after any structural change, so they can never disagree.

**Why the field is wrong even for a one-owner-per-firm milestone:**
- It cannot express a firm owning a firm without changing the field's type (which is a schema change, a log change and a Python change).
- The reverse query "what does household 3 own?" becomes an O(n) inline scan that you will write in four different places.
- Fractional ownership needs the third field (`bps`) that a plain ID cannot carry. Include `bps` now, set every edge to `10_000`, and the dividend split code is already the general form: `dividend * share.bps / 10_000`. Watch the rounding — distribute the remainder deterministically to the lowest-ID owner, and let the ledger's zero-sum check prove you did.

**Bankruptcy interaction:** because edges hold `FirmId{slot,gen}`, a bankrupt firm's edges become unresolvable rather than misdirected. Bankruptcy explicitly retires them (`ownership.retire(firm_id)`) and the respawn inserts a fresh edge at the new generation. A leaked edge is caught by an ownership consistency assertion in the invariants phase: every live firm has exactly 10,000 bps of live owners.

### "Decisions carry provenance"

**Structure:** a `Sink` trait carried in `Ctx`. Decision functions take `&mut Ctx` and call `c.sink.emit(...)`. **No decision function's signature ever mentions provenance.**

```rust
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum Decision {
    Price { tick: u32, firm_slot: u32, firm_gen: u32,
            old: Cents, new: Cents,
            inventory: u64, buffer_lo: u64, buffer_hi: u64,
            unit_labour_cost: Cents, rule: &'static str },
    Wage  { tick: u32, firm_slot: u32, firm_gen: u32,
            old: Cents, new: Cents,
            unfilled_vacancies: u32, inventory: u64, floor: Cents, rule: &'static str },
    Hire  { tick: u32, firm_slot: u32, firm_gen: u32, household: u32,
            offered: Cents, reservation: Cents, firms_sampled: u8, rank: u8 },
}

pub trait Sink { fn emit(&mut self, d: Decision); fn flush(&mut self); }
pub struct NullSink;                       // production runs that don't need it
pub struct VecSink(pub Vec<Decision>);     // unit tests assert on decisions directly
pub struct RunWriter { /* buffered jsonl */ }
```

(Compiled and run.)

The `rule: &'static str` field is the highest-value thing here and costs nothing: it names *which branch* of the price rule fired (`"inventory_below_buffer"`, `"floored_at_unit_cost"`, `"no_change"`). When prices spiral — and PROJECT.md warns they will — a `value_counts()` on that column in Python localises the bug to a branch in one query.

**Volume — quantified, because this is the objection people raise:**

| Stream | Rows over 3,650 ticks | Reasoning |
|--------|----------------------|-----------|
| Price decisions | ≈ 10,400 | 20 firms × 3,650 / 7 (weekly cadence, staggered) |
| Wage decisions | ≈ 10,400 | same cadence |
| Hire events | ≈ 5k–20k | bounded by employment churn |
| Firm panel (per firm-tick) | 73,000 | 20 × 3,650 |
| Tick series | 3,650 | one row per tick |
| **Decision streams total** | **≈ 2–5 × 10⁴ rows, 5–15 MB JSONL** | |
| Full journal (opt-in only) | ≈ 10⁶ rows, ~50 MB CSV | ≈730k purchases + wages + dividends |

**Conclusion: JSONL and CSV are correct. Do not reach for Parquet or Arrow.** The largest always-on artifact is under 20 MB, pandas reads it in well under a second, and plain text files diff — which matters enormously because "same seed produces a byte-identical log" is a *test*, and `diff` is how that test is written. Parquet's compression and column pruning buy nothing at this size and cost you byte-comparability. Revisit only if a per-household-tick panel (730k rows) is ever switched on by default.

Two rules that keep it cheap: emit decisions to a **buffered writer** (`BufWriter`, flushed per tick, not per record), and make a household-tick panel an opt-in config flag rather than a default.

---

## Data Flow

### How money and goods move through one tick

```
 t=N  ┌─ firm_planning ─────────────────────────────────────────────┐
      │  reads   : firm.last_sales, stock(firm, food), unit cost     │
      │  writes  : firm.expected_demand, firm.price, firm.wage_offer │
      │  money   : NONE                                              │
      │  emits   : Decision::Price, Decision::Wage                   │
      └──────────────────────────────────────────────────────────────┘
      ┌─ labour_market ─────────────────────────────────────────────┐
      │  1. collect Vec<Application> from all seekers (bounded       │
      │     firm sampling, rng)      ← INTENT COLLECTION here        │
      │  2. firms accept in deterministic order                      │
      │  writes  : household.employer = Some(FirmId), .wage (fixed   │
      │            at hire), firm.headcount                          │
      │  money   : NONE (wages are paid in the wages phase)          │
      │  emits   : Decision::Hire, Event::Fire                       │
      └──────────────────────────────────────────────────────────────┘
      ┌─ production ────────────────────────────────────────────────┐
      │  for each firm: qty = headcount / recipe.labour_per_unit     │
      │  books.produce(Firm(id), food, qty)                          │
      │  goods   : created[food] += qty ; stock[firm][food] += qty   │
      │  money   : NONE                                              │
      └──────────────────────────────────────────────────────────────┘
      ┌─ wages ─────────────────────────────────────────────────────┐
      │  books.transfer(Firm → Household, household.wage, Wage)      │
      │  money   : FIRM → HOUSEHOLD          (journal += 1 per pay)  │
      │  Err(InsufficientFunds) ⇒ mark firm for bankruptcy phase     │
      └──────────────────────────────────────────────────────────────┘
      ┌─ goods_market ──────────────────────────────────────────────┐
      │  budget = f(books.cash(household))   ← never spends to zero  │
      │  sample k firms (rng), sort by price, buy cheapest-first,    │
      │  fall through to next-cheapest on stockout                   │
      │  books.settle(Trade{buyer, seller, food, qty, price})        │
      │  money   : HOUSEHOLD → FIRM                                  │
      │  goods   : stock[firm] → stock[household]                    │
      │  then books.consume(household, food, qty)                    │
      │  goods   : destroyed[food] += qty                            │
      │  writes  : firm.last_sales (feeds next planning)             │
      └──────────────────────────────────────────────────────────────┘
      ┌─ firm_accounting ───────────────────────────────────────────┐
      │  profit = revenue − wage bill (both from this tick's journal)│
      │  surplus = cash − working_capital_buffer                     │
      │  for share in ownership.owners_of(firm):                     │
      │      books.transfer(Firm → share.owner,                      │
      │                     surplus * share.bps / 10_000, Dividend)  │
      │  money   : FIRM → OWNER   ← THE LOAD-BEARING LINK            │
      └──────────────────────────────────────────────────────────────┘
      ┌─ bankruptcy ────────────────────────────────────────────────┐
      │  1. mark: collect Vec<FirmSlot> where cash < 0 or flagged    │
      │  2. sweep, per marked slot:                                  │
      │       release workers  (household.employer = None)           │
      │       books.transfer(Firm → owner, residual, Bankruptcy)     │
      │       ownership.retire(firm_id)                              │
      │       world.respawn(slot, smaller_firm) → gen + 1            │
      │       draw random household as owner; books.transfer(        │
      │           Household → Firm, seed_capital)                    │
      │           Err(InsufficientFunds) ⇒ redraw                    │
      │       ownership.insert(Share{owner, owned: new_id, 10_000})  │
      │  money   : FIRM → OWNER, then HOUSEHOLD → FIRM (conserved)   │
      └──────────────────────────────────────────────────────────────┘
      ┌─ invariants ────────────────────────────────────────────────┐
      │  Σ cash                    == total_money                    │
      │  created − destroyed − Σ stock == 0   (per good)             │
      │  ∀ accounts: cash ≥ 0                                        │
      │  ∀ postings in journal: debit == credit                      │
      │  + ownership: every live firm has 10,000 bps of live owners  │
      │  on failure ⇒ bisect journal, print tick/agent/posting, exit │
      └──────────────────────────────────────────────────────────────┘
      ┌─ log ───────────────────────────────────────────────────────┐
      │  tick_series row; firm_panel rows; flush sink buffers        │
      └──────────────────────────────────────────────────────────────┘
      books.clear_journal();  tick += 1
```

### Key invariant of the flow

**Every single arrow labelled "money" is a `Books::transfer` or `Books::settle` call.** There is no other way. That is the entire correctness argument, and it is enforced by the type system: `Household` and `Firm` have no cash field to touch.

---

## The Sim/Analysis Boundary

### The seam: a run directory + a generated, committed schema

```
runs/<run_id>/
  manifest.json      seed, resolved config, config_hash, schema_version,
                     git_sha, rustc_version, n_ticks, started_at, exit_status
  tick_series.csv    3,650 rows × ~20 cols — aggregates
  firm_panel.csv     73,000 rows — per firm-tick state (slot, gen, ...)
  events.jsonl       bankruptcy | hire | fire | cash_out
  decisions.jsonl    price | wage | hire provenance
  journal.csv        OPT-IN, ~10⁶ rows
schema/schema.json   ← the contract
```

**Where the schema lives, so both sides agree:** in `schema/schema.json` at the repo root, **generated from the Rust types and committed**.

- Rust is the single source of truth. Every record type in `log/schema.rs` derives `Serialize` + `JsonSchema` (`schemars` v1.2.2, which is explicitly serde-compatible — the generated schema matches what `serde_json` actually writes).
- The binary gets a `--dump-schema` subcommand.
- A Rust test regenerates the schema and asserts it equals the committed file. **Schema drift is a red build**, which is the property that makes this a real contract rather than documentation.
- Python's `analysis/schema.py` loads the same `schema/schema.json`, validates the run directory against it, and refuses a `schema_version` major mismatch with a clear error.

**Add a golden run.** `tests/golden/` holds a committed 50-tick run directory. Rust asserts it reproduces byte-identically; Python asserts it can parse and compute every acceptance metric on it. This is the cheapest possible end-to-end contract test — it catches a Rust-side format change and a Python-side parser assumption in the same commit, and it runs in milliseconds.

**Direction of dependency is one-way: Rust → schema → Python.** Python never writes anything the sim reads. Nothing about plotting or statistics enters the Rust binary; the sim's only output obligation is well-formed records.

**Why CSV for panels and JSONL for events:** panels are rectangular and column-typed (CSV, with the header as the human-readable schema); events and decisions are heterogeneous tagged unions (JSONL, with `#[serde(tag = "kind")]`). Both are text, both diff, both are read by one pandas call.

---

## Suggested Build Order

The single most important observation for the roadmap: **tick order is not build order.** `firm_planning` runs first in the tick but is built seventh, because a reaction rule cannot be tuned before there is something to react to.

The second most important: **invariants and logging are built before any economics**, so every economic phase is born under the check rather than retrofitted into it.

```
┌── S0 PRIMITIVES ────────────────────────────────────────── (no deps)
│   money.rs (Cents)  ids.rs  config.rs (Params + hash)  rng.rs (ChaCha8)
│   Gate: property test — Cents arithmetic never silently wraps;
│         same seed → identical u64 stream.
└─────────────────────────┬───────────────────────────────────────────
┌── S1 BOOKS + INVARIANTS ▼ ───────────────────────── (needs S0)
│   books.rs (cash, stock, journal, transfer/settle/produce/consume)
│   invariants.rs (four checks + journal bisection)
│   BUILT AS ONE UNIT — the ledger is what the invariants check.
│   Gate: unit tests prove transfer is atomic, refuses overdraft,
│         and that a deliberately corrupted books FAILS each of the
│         four checks with the right agent and posting named.
└─────────────────────────┬───────────────────────────────────────────
┌── S2 WORLD + PIPELINE + LOG ▼ ───────────────────── (needs S1)
│   world.rs (arena, FirmId{slot,gen}, Ctx)
│   phases/mod.rs (PHASES table with all 9 phases as no-ops)
│   log/ (Sink trait, schema.rs, RunWriter, manifest, --dump-schema)
│   Gate: 3,650 EMPTY ticks run, invariants pass trivially, a run
│         directory is produced, two runs at the same seed diff clean.
│         ⇒ The harness is real before any economics exist.
└─────────────────────────┬───────────────────────────────────────────
┌── S3 GOODS + PRODUCTION ▼ ───────────────────────── (needs S2)
│   goods.rs (GoodsTable, Recipe) ; phases/production.rs
│   First thing that moves the goods identity.
│   Gate: created − destroyed − Σstock == 0 over 3,650 ticks.
└─────────────────────────┬───────────────────────────────────────────
┌── S4 LABOUR + WAGES ▼ ───────────────────────────── (needs S3)
│   phases/labour.rs (intent collection, bounded sampling,
│                     reservation wages) ; phases/wages.rs
│   First thing that moves MONEY. Wages contracted at hire.
│   Gate: money conservation holds across 3,650 ticks of hiring
│         and paying; unemployment is not 0% and not 100%.
└─────────────────────────┬───────────────────────────────────────────
┌── S5 GOODS MARKET ▼ ─────────────────────────────── (needs S3+S4)
│   phases/goods_market.rs (bounded sampling, cheapest-first,
│                           stockout fallthrough, wealth-dependent
│                           budget) ; firm.last_sales starts filling
│   Gate: money flows household→firm; a full circular flow exists;
│         both invariants still hold. Prices/wages are STILL STATIC.
└─────────────────────────┬───────────────────────────────────────────
┌── S6 OWNERSHIP + ACCOUNTING + DIVIDENDS ▼ ──── (needs S5)
│   ownership.rs (relation + both-direction index)
│   phases/accounting.rs (profit, buffer, dividend split)
│   ⚠ DO NOT DEFER. PROJECT.md names the missing dividend link as
│     the #1 way this build dies. Without it, cash pools in firms and
│     the economy stalls within a few simulated years — and you will
│     misdiagnose it as a price-rule bug.
│   Gate: a 3,650-tick run does NOT stall; household cash share is
│         stable; the split is exactly zero-sum after rounding.
└─────────────────────────┬───────────────────────────────────────────
┌── S7 FIRM PLANNING ▼ ────────────────────────────── (needs S5+S6)
│   phases/planning.rs (adaptive expectation, price rule floored at
│   unit labour cost, wage rule, weekly cadence STAGGERED across the
│   week) — built LAST of the economics because it needs sales
│   history and a non-stalling economy to tune against.
│   Gate: prices move and do not spiral; output autocorrelates;
│         staggering verified (no week-boundary sawtooth).
└─────────────────────────┬───────────────────────────────────────────
┌── S8 BANKRUPTCY + RESPAWN ▼ ─────────────────────── (needs S2+S4+S6)
│   phases/bankruptcy.rs (mark-then-sweep; release, residual to
│   owner, retire edges, respawn at gen+1, seed capital with redraw)
│   Deferred to last among economics because it needs the generational
│   arena (S2), worker release (S4) and the ownership relation (S6).
│   Gate: firms die and respawn over a decade with money conserved,
│         no stale FirmId ever resolves, firm count stays 20, and
│         per-(slot,gen) log groups are complete and non-overlapping.
└─────────────────────────┬───────────────────────────────────────────
┌── S9 PYTHON ACCEPTANCE HARNESS ▼ ────────────────── (needs S2 for
│   the schema; S8 for a complete economy — but START AT S2)          │
│   analysis/schema.py, acceptance.py, charts.py; golden run test
│   Write the conservation-audit and seed-diff checks against the
│   EMPTY-tick run from S2, then grow the harness alongside S3–S8.
│   Gate: full section-7 criteria on a 3,650-tick run, first 250
│         ticks discarded as burn-in.
└─────────────────────────────────────────────────────────────────────
```

### Dependency edges, stated plainly

| This | must exist before | Because |
|------|-------------------|---------|
| `Cents`, `ids`, `rng`, `config` | everything | they are the vocabulary |
| `Books` | any phase that moves value | it is the *only* thing that moves value |
| `invariants` | any economics | so every rule is born under the check |
| `FirmId{slot,gen}` arena | bankruptcy | stale IDs are the hazard bankruptcy creates |
| log schema + `Sink` | the Python harness | the harness reads the schema, not the code |
| `production` | `goods_market` | you cannot sell what does not exist |
| `labour` + `wages` | `goods_market` | households need income before they have a budget |
| `goods_market` | `planning` | the price rule needs `last_sales` to react to |
| `ownership` | `accounting` (dividends) | dividends are a query over the relation |
| `ownership` + `labour` + arena | `bankruptcy` | it retires edges, releases workers, bumps generation |
| non-stalling economy (S6) | tuning the price rule (S7) | otherwise you tune against a dying economy |

### Cross-cutting, wired in early, never a phase of its own

- **Provenance.** The `Sink` is in `Ctx` from S2. Every rule emits as it is written, in S4–S8. Retrofitting provenance is exactly the thing PROJECT.md forbids.
- **Config.** Every number introduced in S3–S8 lands in `config/default.toml` in the same commit that uses it. Never `let lambda = 0.25;`.
- **Determinism test.** `tests/determinism.rs` exists from S2 and runs on every commit thereafter. It is cheap and it fails loudly the first time someone iterates a `HashMap`.

---

## Scaling Considerations

Recast for simulation size rather than users — this build explicitly excludes scaling beyond 200 agents.

| Scale | Adjustments |
|-------|-------------|
| **220 agents (this build)** | Everything above. `Vec<Household>` / `Vec<Firm>` AoS is correct. A decade runs in seconds; invariants every tick are free; JSONL/CSV logs are small. **Do not optimise anything.** |
| **~2,000 agents** | Still AoS. First real cost is the journal (~10⁷ postings/decade) — keep it per-tick-buffered, which the design already does. Bounded market sampling means market cost is O(n·k), not O(n²), so this scales naturally. |
| **~20,000+ agents** | Now consider struct-of-arrays for the hot fields (cash, price, inventory) while keeping cold fields in AoS. `Books` is already SoA, so the migration is mostly `Firm` field extraction. `slotmap` replaces the hand-rolled arena if the population becomes dynamically sized. Determinism survives all of this; parallelism does not, so do not. |

### AoS vs SoA vs ECS at this scale — the verdict

**`Vec<Household>` / `Vec<Firm>` with index IDs is unambiguously right, and it is right for correctness reasons, not just simplicity.**

- 220 agents × ~10 fields fits in L2 several times over. Cache-layout arguments — the entire case for SoA and ECS — are irrelevant here. The published Rust ABM that does use SoA (`facorread/rust-agent-based-models`) justifies it explicitly on memory-performance grounds, which is a benefit you do not need and cannot measure.
- An ECS (Bevy, `hecs`, `legion`) would import a scheduler, and a scheduler is a *reordering machine*. This project's core value claim is a fixed, provable tick order. Adopting a framework whose main feature is deciding execution order for you is directly adversarial to the requirement. The published ECS-for-ABM literature motivates it by parallel performance — a thing this project has explicitly ruled out.
- Debuggability wins: `dbg!(&world.firms[7])` shows you a whole firm. In SoA you assemble it by hand from ten vectors, every time, for a decade of debugging sessions.

**What would change the call:** agent count crossing ~10⁴ *and* a profile showing memory-bound phases; or a decision to parallelise (which would first require abandoning byte-identical determinism). Neither is in scope. `Books` is already SoA for the two fields where it matters, which is the right hybrid.

---

## Anti-Patterns

### 1. A `cash` field on `Household` and `Firm`

**What people do:** the obvious thing — `struct Firm { cash: Cents, ... }`.
**Why it's wrong:** it splits the money supply across two vectors, so (a) every transfer needs two simultaneous mutable borrows and you end up reaching for `RefCell`, (b) conservation becomes a whole-world two-vector scan instead of a compare against a cached total, (c) there is no single place to hang the journal, so provenance and error handling scatter.
**Do this instead:** `Books` owns all cash. Agents have none.

### 2. `Rc<RefCell<Firm>>` or `&Firm` inside a `Household`

**What people do:** reach for it the moment they want `household.employer.offered_wage`.
**Why it's wrong:** PROJECT.md names this as the signal the design went wrong. It also breaks determinism-by-inspection (aliasing makes "who mutated this" unanswerable) and makes the whole `World` unserialisable.
**Do this instead:** `ctx.w.firm(household.employer?)?.offered_wage`. The `Option` chain is the correct amount of friction — it is asking you to handle the dead-firm case, which is a real case.

### 3. Raw `usize` firm IDs

**What people do:** `struct Household { employer: Option<usize> }`.
**Why it's wrong:** this is the single worst latent bug in the design. Bankruptcy respawn reuses slot 7; household 42's `employer = Some(7)` now silently points at a brand-new firm; wages are paid by the wrong firm; the economy still runs and the output still looks plausible. You will lose days.
**Do this instead:** `FirmId { slot, gen }` and an `Option`-returning accessor.

### 4. `HashMap` iteration on a behaviour path

**What people do:** `for (id, x) in &self.some_map`.
**Why it's wrong:** Rust's default hasher is randomly seeded per process. Iteration order changes between runs, so the same seed produces different logs. PROJECT.md forbids it explicitly, and it is the failure mode most likely to be introduced accidentally six weeks in.
**Do this instead:** `BTreeMap` / `BTreeSet` / `Vec`. The `Ownership` sketch above uses `BTreeMap` for exactly this reason.

### 5. `StdRng`

**What people do:** `StdRng::seed_from_u64(seed)` — the example in most tutorials.
**Why it's wrong:** verified against the `rand` documentation — `StdRng` explicitly opts out of reproducibility. A `cargo update` can silently change every trajectory in the project.
**Do this instead:** `ChaCha8Rng::seed_from_u64(seed)`, with `rand` and `rand_chacha` pinned by exact version. And hand-roll behaviour-defining sampling rather than trusting `Uniform`/`SliceRandom`, whose algorithms have already changed once.

### 6. `debug_assert!` for the four invariants

**What people do:** reach for `debug_assert!` because invariant checks "sound expensive".
**Why it's wrong:** verified — `debug_assert!` is compiled out of optimised builds unless `-C debug-assertions` is passed. Your decade-long release runs would carry no checks at all, which is precisely the scenario the requirement exists to prevent. And the checks are not expensive: four reductions over 220 accounts.
**Do this instead:** a real `invariants` phase returning `Result`, with a formatted diagnostic and `exit(1)`. Keep `debug_assert!` only as a *finer* dev-only tripwire inside `Books`.

### 7. `enum Good { Food }`

**What people do:** a one-variant enum, reasoning that it is type-safe and can be extended later.
**Why it's wrong:** the enum is not the problem; the fifteen `match` sites are. Each becomes a redesign rather than an extension, and prices/stock become scalars that have to be widened to vectors everywhere at once.
**Do this instead:** a `GoodsTable` with `GoodId(u16)`, `stock[account][good]`, and `for g in goods.iter()` even when there is one good.

### 8. `Firm { owner: HouseholdId }`

**What people do:** the field, because there is exactly one owner today.
**Why it's wrong:** it cannot express firm-owns-firm without a type change that ripples into the schema and into Python; the reverse query becomes an O(n) scan written inline in four places; and fractional ownership has nowhere to live.
**Do this instead:** the `Ownership` edge list with `Entity` (already `Household | Firm`) and `bps` (already there, always 10,000).

### 9. Provenance as an extra return value

**What people do:** `fn price_rule(...) -> (Cents, PriceProvenance)`, threaded up through three call layers.
**Why it's wrong:** every rule's signature churns, every caller has to plumb it, and the first time someone is in a hurry they drop it — which is exactly the early-history gap PROJECT.md warns retroactive provenance can never fill.
**Do this instead:** `&mut dyn Sink` inside `Ctx`. Rules call `c.sink.emit(...)` and return only their result.

### 10. Deferring dividends to "polish"

**What people do:** build planning, labour, production and the goods market, see the economy stall, and start debugging the price rule.
**Why it's wrong:** PROJECT.md names this as the single most common way a first build of this model dies. Without profits flowing to owning households, cash accumulates inside firms, drains out of households, and the economy deflates within a few simulated years — and the symptom (falling prices, falling output) looks exactly like a price-rule bug.
**Do this instead:** S6 before S7. Ship ownership and dividends before you tune a single behavioural parameter, and make "a 3,650-tick run does not stall" the gate on S6.

### 11. Intent collection everywhere

**What people do:** having learned the pattern, apply it to all nine phases.
**Why it's wrong:** it roughly doubles the code of every phase and buys nothing where disjoint field borrows already work. It also introduces a second place where ordering can drift.
**Do this instead:** intent collection only in `labour` (fairness requires a global view of applications) and `bankruptcy` (mark-then-sweep, because killing while iterating invalidates the loop).

---

## Integration Points

### External Dependencies

| Dependency | Current version | Integration pattern | Gotchas |
|------------|-----------------|---------------------|---------|
| `rand` | 0.10.2 | trait surface only | pin with `=`; **never** `StdRng`; distributions are not value-stable |
| `rand_chacha` | 0.10.0 | `ChaCha8Rng` behind a project `Rng` wrapper | the wrapper is where hand-rolled samplers live |
| `serde` / `serde_json` | 1.0.151 | derive on `log/schema.rs` types only | tagged enums for events/decisions |
| `schemars` | 1.2.2 | derive alongside `Serialize`; `--dump-schema` | explicitly serde-compatible; committed output + drift test |
| `csv` | 1.x | panel writers | `BufWriter`, flush per tick |
| `toml` | 1.1.4 | `Params` deserialisation | reject unknown keys (`deny_unknown_fields`) so typos are errors |
| `typed-index-collections` | 3.5.0 | *optional*, `TiVec<FirmSlot, Firm>` | adopt only if `as usize` casts proliferate |
| `slotmap` | 1.1.1 | *not now* | the answer if the firm population ever becomes dynamically sized |

### Internal Boundaries

| Boundary | Communication | Rule |
|----------|---------------|------|
| `phases/*` → `books` | `transfer` / `settle` / `produce` / `consume`, all returning `Result` | the **only** way value moves; no phase touches a balance directly |
| `phases/*` → `world` | `Ctx.w`, disjoint field borrows destructured once per phase | no `RefCell`, no `Rc`, no inter-agent references |
| `phases/*` → `log` | `Ctx.sink.emit(...)` | provenance never appears in a rule's signature |
| `invariants` → `books` | `&Books` + `&[Posting]`, read-only | the checker never mutates |
| `world` → `books` | generation counters mirrored on respawn | the arena and the books must agree on which slots are live |
| `ownership` → `world` | holds `FirmId`, never `&Firm` | edges retired on bankruptcy; consistency asserted every tick |
| Rust → Python | run directory + `schema/schema.json` | one-way; schema generated from Rust, committed, drift is a test failure |

---

## Confidence and Gaps

| Claim | Confidence | Basis |
|-------|------------|-------|
| Split-borrow / `get_disjoint_mut` / index-loop patterns | **HIGH** | compiled and executed on `rustc 1.94.1` |
| Central-ledger transfer pattern | **HIGH** | compiled and executed, including the conservation check |
| Generational `FirmId` arena | **HIGH** | compiled and executed |
| `const PHASES` table + `Ctx` + `dyn Sink` | **HIGH** | compiled and executed |
| Goods table, ownership relation, provenance sink | **HIGH** | compiled and executed |
| `assert!` runs in release, `debug_assert!` does not | **HIGH** | std docs, cross-checked across two independent sources |
| `StdRng` is not value-stable; use `rand_chacha` | **MEDIUM-HIGH** | `rand` docs + the rust-random book's portability chapter, cross-checked |
| `rand` distributions are not value-stable | **MEDIUM** | rust-random/rand issue #786 |
| Crate versions | **HIGH** | crates.io registry API, queried 2026-08-30 |
| AoS is right at 220 agents | **MEDIUM-HIGH** | reasoning from cache size + the SoA literature's own stated (performance) motivation, which does not apply here |
| Log volume estimates | **MEDIUM** | derived arithmetic from the brief's parameters, not measured |

### Open questions for phase-level research

1. **Is purchased food consumed immediately, or do households hold inventory?** The brief's tick order has no consumption phase. This determines whether the goods identity has a household-stock term. Recommendation: model `consume` explicitly either way so the identity keeps one shape; resolve during the goods-market phase discussion.
2. **Dividend rounding under fractional ownership.** With one owner at 10,000 bps it never bites. Fix the deterministic remainder rule (lowest owner ID takes the remainder) before multi-owner arrives.
3. **Exact staggering scheme for weekly planning.** `firm_slot % 7` is deterministic and even, but it correlates cadence with slot, which respawn then perturbs. Worth a phase-level decision.
4. **Whether the `firm_panel` should carry `books`-derived columns** (cash, stock) or only behavioural state. Carrying both is redundant but makes Python's job trivial; recommend carrying both and letting the invariants prove consistency.

---

## Sources

- [Splitting Borrows — The Rustonomicon](https://doc.rust-lang.org/nomicon/borrow-splitting.html) — `split_at_mut`, `split_first_mut`, why the borrow checker cannot prove index disjointness
- [`slotmap` — docs.rs](https://docs.rs/slotmap/latest/slotmap/) and [orlp/slotmap](https://github.com/orlp/slotmap) — generational keys, `new_key_type!`, `SecondaryMap`, invalidation-on-removal semantics
- [`facorread/rust-agent-based-models`](https://github.com/facorread/rust-agent-based-models) — a published Rust ABM using SoA + `slotmap` for dying agents; the stated rationale is memory performance
- [Seeding RNGs — The Rust Rand Book](https://rust-random.github.io/book/guide-seeding.html) and [`rand::rngs` — docs.rs](https://docs.rs/rand/latest/rand/rngs/) — `StdRng` opts out of reproducibility; use a named algorithm
- [`rand_chacha` — docs.rs](https://docs.rs/rand_chacha) — deterministic, portable, tested against reference vectors
- [rust-random/rand issue #786 — Value stability of distributions](https://github.com/rust-random/rand/issues/786) — `Uniform` integer/float algorithms have changed, breaking value stability
- [`std::debug_assert` — doc.rust-lang.org](https://doc.rust-lang.org/std/macro.debug_assert.html) and [Rust's Two Kinds of 'Assert' Make for Better Code — Laurence Tratt](https://tratt.net/laurie/blog/2023/rusts_two_kinds_of_assert_make_for_better_code.html) — release-build behaviour
- [`typed-index-collections` — docs.rs](https://docs.rs/typed-index-collections/latest/typed_index_collections/) and [`index_vec` — docs.rs](https://docs.rs/index_vec) — typed index newtypes over `Vec`
- [`schemars` — GitHub](https://github.com/GREsau/schemars) — JSON Schema generation from Rust types, serde-compatible
- [Lengnick, *Agent-based macroeconomics: A baseline model* (JEBO 86, 2013)](https://legacy.econ.tuwien.ac.at/lva/compeco.se/artikel/jebo_2013_agent_based_macroeconomics_a_baseline_model.pdf) — the discrete-day event structure this tick order follows
- [newwayland/baseline-economy](https://github.com/newwayland/baseline-economy) — a Mesa replication of Lengnick; useful as a behavioural cross-check, not as a layout model
- [Caiani et al., *Agent based-stock flow consistent macroeconomics: Towards a benchmark model* (JEDC 69, 2016)](https://faculty.sites.iastate.edu/tesfatsi/archive/tesfatsi/ABMSFCMacroModelBenchmark.CainiEtAl2016.pdf) — double-entry bookkeeping as the accounting discipline behind the invariants
- crates.io registry API — version and freshness data for every crate named above, queried 2026-08-30

---
*Architecture research for: deterministic agent-based macroeconomic simulation in Rust*
*Researched: 2026-08-30*
