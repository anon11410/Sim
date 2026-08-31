# Phase 2: Books, Journal and Invariants - Research

**Researched:** 2026-08-31
**Domain:** Deterministic double-entry ledger in Rust, per-tick invariant pipeline, adversarial negative testing
**Confidence:** HIGH (every architectural claim below was compiled and executed on the pinned `rustc 1.94.1` this session; the four MEDIUM/LOW items are named individually in the Assumptions Log)

## Summary

The architecture for this phase is already settled and is **not** re-litigated here (see `## User Constraints`). What research adds is the implementation-level detail the design leaves open — and in four places it materially changes what the plan should say.

The four findings that change the plan:

1. **Bisection to the offending posting is unsound.** A binary search for "the first posting whose residual is non-zero" assumes the residual has a monotone onset. It does not: a dropped cent followed later by an equal over-credit returns the residual to zero, and the search then names a *later, unrelated* posting. Measured this session: a journal broken at posting #50 and healed at #120 caused the bisection to report **#200** while a linear scan correctly reported **#50**. A linear scan over one tick's journal costs **80 ns/tick** — cheaper than the conservation recompute it accompanies. **Recommend the linear first-non-zero-residual scan, and satisfy LEDG-09's word "bisected" by localisation quality, not by algorithm.**

2. **`&mut self` alone does not make LEDG-02 true.** A `&mut Books` method that takes a *callback* hands out a mid-transaction `&Books`, and it compiles. Demonstrated this session: a hook observed a total of 50 against an opening 100. LEDG-02 needs four legs, not one — borrow discipline, a no-callback signature rule, no interior mutability, and panic-atomicity via compute-then-commit.

3. **LEDG-02 is *provable by an executable test*, not only by "a test is impossible to write".** A non-atomic `transfer` that writes one leg then panics leaves the books at **-400** against an opening **100**, observable through `catch_unwind` + `AssertUnwindSafe`. The compute-then-commit version leaves them at **100**. That is a positive, writeable, passing test — far stronger evidence than an unfalsifiable claim about what cannot be written.

4. **The negative test needs no cargo feature and no production hole.** `#[cfg(test)]` methods on `Books` are reachable from unit tests in `src/books.rs` (private fields included) and are a **compile error `E0599`** from an integration test in `tests/`. Verified both directions this session. The corruption vocabulary therefore lives in `src/books.rs` under `#[cfg(test)]`, is invisible to every consumer including the crate's own integration tests, and needs no `[features]` entry, no `--features` flag in CI, and no toolchain assertion that a fault-injection feature stayed off.

Supporting measurements: the money-conservation recompute over 220 accounts costs **175 ns/tick — 0.64 ms for the whole 3,650-tick decade**, so LEDG-04's "every tick, in release, always" is free. And `debug_assert!`'s body is **not evaluated** under this repo's exact release profile (`overflow-checks = true`, `debug-assertions` left at its default `false`) — verified by running it — which is the hard evidence behind LEDG-10.

**Primary recommendation:** Build `books.rs` around a compute-then-commit `transfer` returning the amount actually moved; keep the journal as a `Vec<Posting>` where each posting carries the two running residuals *after* it is applied, so localisation is a linear scan and not a replay; build the check set once from config into an ordered `Vec` so the liveness gate is one branch at construction and none per tick; and put the four corruption routines under `#[cfg(test)]` in `src/books.rs` where the compiler guarantees they cannot escape.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Cash balances, goods stock | `books` (state) | — | LEDG-01: one owner, both legs of a transfer inside it |
| Value mutation (`transfer`, `produce`, `consume`) | `books` (state) | — | LEDG-02: a single `&mut self` entry point, no callbacks |
| Per-tick journal buffer | `books` (state) | — | Written only by `books`' own posting recorder; nobody else can push |
| Invariant evaluation | `invariants` (pure read) | `books` (read-only accessors) | Reads `&Books`, mutates nothing, returns `Result` |
| Offending-posting localisation | `invariants` (pure read) | `books` (journal slice accessor) | The journal is data; the search is a pure function over it |
| Check-set selection from config | `invariants` (construction) | `config` (params) | One read of the gate, at construction; the per-tick path has no conditional |
| Halting the run | Phase 3's pipeline / `main.rs` (CLI) | `invariants` (returns the `Err`) | Phase 2 produces the typed error; Phase 3's `const PHASES` propagates it |
| Economic decisions of any kind | **none — out of scope** | — | CONTEXT: this phase contains no economic behaviour |

## User Constraints (from CONTEXT.md)

### Locked Decisions

*(Copied verbatim from `02-CONTEXT.md` § Locked by prior decisions — not reopened here.)*

- **Agents own no value.** `Books` holds every balance; no `set_cash` exists
  anywhere. (Research SUMMARY, Architecture — the load-bearing move.)
- **`transfer()` is the only cash-mutation point, and is atomic.** The books are
  never observable mid-transaction, which is what makes zero-sum trade a
  property of the API rather than a thing to test for.
- **Invariants are a pipeline phase returning `Result`, never `debug_assert!`.**
  `debug_assert!` is compiled out of release, and an invariant absent from the
  binary that produced the run is worth nothing. Cost is ~220 `i64` adds per
  tick — run it in release, every tick, always.
- **The journal is a per-tick buffer, not an append-forever log.** A decade
  produces ~10⁶ postings; a violation is always locatable inside the tick it
  occurred. Accumulate, check, bisect to name the offending posting, clear.
  Disk write is a config flag.
- **Liveness is config-gated off for Phase 3's pre-economics empty run** and on
  by default from Phase 6. Recorded as a cross-phase constraint by the
  roadmapper: LEDG-08 would otherwise fail TICK-08's 3,650 empty ticks.
- **Money is already checked.** `Money` panics on overflow in every profile and
  `Money::split` conserves the remainder — both delivered and tested in Phase 1,
  so LEDG-03's obligation here is on the *callers*: subtract what was actually
  transferred, never the intended amount.

### Claude's Discretion

*(Copied verbatim from `02-CONTEXT.md`.)*

All remaining implementation choices — module layout inside `books`, the
journal's internal representation, the exact `Result` error type carried out of
the invariant phase, and how the bisect-to-offending-posting search is written.
Guided by ROADMAP success criteria and the conventions Phase 1 established.

### Deferred Ideas (OUT OF SCOPE)

*(Copied verbatim from `02-CONTEXT.md`.)*

- Writing the journal to disk each tick — a config flag exists in the design but
  the per-tick buffer is what Phase 2 needs; disk persistence belongs with the
  log seam in Phase 3.
- Any economic behaviour whatsoever. Nothing in this phase decides a price, a
  wage or a hire.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LEDG-01 | A central `Books` module owns every cent and every goods unit; `Household` and `Firm` hold no balance fields and expose no `set_cash` | Pattern 1 (module shape); Pitfall 4 (`Household`/`Firm` do not exist yet — the absence must be asserted, not assumed); Validation row LEDG-01 |
| LEDG-02 | `transfer()` is the only cash-mutation point and is atomic — the books are never observable mid-transaction | **Pattern 2 (the four legs)**; Code Example 2 (compute-then-commit); Code Example 3 (the executable atomicity test); Pitfall 1 (the callback hole, reproduced) |
| LEDG-03 | `Money::split` distributes any remainder deterministically, and callers subtract the amount actually transferred | Pattern 3 (`transfer` returns `Money`); Pitfall 6 (LEDG-01 dissolves most of this, but a derived accumulator reopens it) |
| LEDG-04 | Money conservation is checked every tick in release builds against the initial money stock, exactly | Pattern 5 (residual-carrying journal); measured cost 175 ns/tick; Pitfall 5 (the `opening_stock` constant must not be recomputed from the same balances it checks) |
| LEDG-05 | Goods conservation is checked every tick: produced minus consumed equals inventory | **Pattern 6 (the one-shape identity)** — per-`Account` inventory, so Phase 7's open question changes no formula |
| LEDG-06 | Non-negativity is checked every tick across cash, inventory and headcount | Pattern 7; Pitfall 7 (headcount has no owner in Phase 2 — the check must be present and provably reachable, not silently empty) |
| LEDG-07 | Zero-sum trade is checked: every sale moves units one way and equal cash the other | Pattern 4 (a `Posting` carries both legs, so zero-sum is checkable per posting rather than per aggregate) |
| LEDG-08 | A liveness invariant asserts transactions-per-tick is greater than zero | **Pattern 8 (check-set at construction)**; config shape in `## Configuration Surface`; Code Example 5 (verified) |
| LEDG-09 | On violation the sim halts immediately and prints the tick, the agent and the offending transaction, bisected from a per-tick journal buffer | **Pattern 5 + Pitfall 2 (bisection is unsound — measured)**; Code Example 4 (localisation); error type in `## Error Type Design` |
| LEDG-10 | Invariants are a real pipeline phase returning `Result`, never `debug_assert!`, and a negative test proves a deliberately seeded leak actually halts the run | **Pattern 9 (`#[cfg(test)]` corruption vocabulary — verified invisible to `tests/`)**; verified `debug_assert!` fact; `## Negative Test Design`; `## Validation Architecture` |

## Project Constraints (from CLAUDE.md)

These are as binding as the locked decisions. Each is stated as the concrete obligation it imposes on Phase 2 code.

| Directive | Obligation on this phase |
|-----------|--------------------------|
| Integer cents everywhere in money | `Money` only; `books.rs` and `invariants.rs` must name **no** float type at all — see Pitfall 3, which is enforced by an existing test |
| IDs never references; no `Rc<RefCell<…>>` | Balances are `Vec<Money>` indexed by `HouseholdId.0` / `FirmSlot.0`; postings key on `ids::Account` |
| No `HashMap`/`HashSet` on a behaviour path | Ledger storage is `Vec` indexed by dense ID. If a sparse map is ever needed, `BTreeMap`. `clippy.toml` `disallowed-types` already denies both |
| Single-threaded, single seeded RNG | The invariant phase consumes **no** RNG draws. Introducing one would shift every downstream sub-stream (CORE-04) |
| `#![forbid(unsafe_code)]` (already in `lib.rs`) | Rules out raw-pointer aliasing as an LEDG-02 escape hatch — this is load-bearing for Pattern 2 leg 3 |
| No parameter hardcoded in logic; all in TOML | The liveness gate is a config key, not a `const`, not an env var, not a CLI flag |
| `[profile.release] overflow-checks = true`; `Money` checked in every profile | Already true. Verified this session that this does **not** enable `debug_assertions` |
| `pub mod` surface in `lib.rs` flat and alphabetical | `pub mod books;` sorts between `config` and `ids`… — actually before `config`; `pub mod invariants;` between `ids` and `money` |
| Module `//!` docs carry requirement and decision IDs | `books.rs` cites LEDG-01/02/03/07; `invariants.rs` cites LEDG-04…10. The validation audit greps for these |
| Guards are adversarial, not declarative | Every gate in this phase must be *observed to fire*, following the `tests/lints.sh` discipline |
| Constants that would drift are generated, never typed | The `ALL_CHECKS` order is the source of truth; the order test reads it, never a second hand-written list |
| `main.rs` stays thin; `anyhow` only there, `thiserror` in the lib | `Violation` is a `thiserror` enum in `src/invariants.rs` |

## Standard Stack

**This phase adds no new dependency.** Everything it needs is already in the committed `Cargo.lock`.

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `thiserror` | **2.0.20** | The `Violation` and `PostError` enums | Already the project's typed-error crate; `#[error("…")]` interpolates `Display` of nested fields, verified this session with a `Posting` field rendering inside a `Violation` variant `[VERIFIED: Cargo.lock, and compiled+run this session]` |
| `std` only | rustc 1.94.1 | `Vec<Money>` balances, `Vec<Posting>` journal | No collection crate is warranted: every ledger key is a dense integer index `[VERIFIED: src/ids.rs:31-41 — `HouseholdId(pub u32)`, `FirmSlot(pub u16)`]` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `proptest` | **1.11.0** (dev) | Conservation under random posting sequences; `transfer` return-vs-delta agreement | Already a dev-dependency with a committed `.proptest-regressions/` `[VERIFIED: Cargo.lock; .proptest-regressions/ exists]` |
| `serde` | **1.0.229** | `Serialize` on `Posting` / `PostingKind`, so Phase 3's `events.jsonl` needs no adapter type | Derive only. Add it now; retrofitting a wire shape after Phase 3 snapshots exist is a trajectory-visible change |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `#[cfg(test)]` corruption methods | A `fault-injection` cargo feature | The feature is reachable from `tests/`, which is its only advantage; it costs a `[features]` entry, a second CI invocation, and a new toolchain assertion that the feature stayed out of the default set. `#[cfg(test)]` gets the same power with a compiler-enforced boundary. **Rejected.** |
| `#[cfg(test)]` corruption methods | A `trybuild` compile-fail harness for LEDG-02 | `trybuild` is a new dependency whose expected-output files are toolchain-version-sensitive — a hazard for a repo whose entire premise is a pinned compiler. The `tests/lints.sh` idiom (inject a probe, assert the build fails, restore under a `trap`) already exists here and is toolchain-agnostic. **Rejected.** |
| Linear residual scan | True binary search over the journal | Measured unsound on cancelling residuals (Pitfall 2). Same asymptotic cost at this journal size. **Rejected.** |
| Journal as `Vec<Posting>` | A fixed-capacity ring buffer of the last N postings | The research SUMMARY mentions a ring buffer of the last N transfers. A per-tick `Vec` cleared each tick is bounded by the tick's own posting count (~10² – 10³), so the ring adds a wrap-index and a "did we lose the offending posting?" failure mode for no memory saving. **Rejected — but `Vec::clear()` must be used, not reallocation, so the capacity is reused.** |
| `Vec<(CheckId, &str, CheckFn)>` built at construction | `const CHECKS` + a runtime `enabled: [bool; 5]` mask | The mask puts a branch back on the per-tick path — the exact "scattered conditional" the CONTEXT asks to avoid. **Rejected.** |

**Installation:** none. `cargo add` is not run in this phase.

## Package Legitimacy Audit

> This phase installs **no** external packages. The table records the two crates whose *usage* expands, both already locked.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `thiserror` 2.0.20 | crates.io | already locked | — (not queried; no new install) | dtolnay/thiserror | OK (pre-existing) | Approved — already a direct dependency `[VERIFIED: Cargo.toml:19, Cargo.lock]` |
| `proptest` 1.11.0 | crates.io | already locked | — (not queried; no new install) | proptest-rs/proptest | OK (pre-existing) | Approved — already a dev-dependency `[VERIFIED: Cargo.toml, Cargo.lock]` |

**Packages removed due to [SLOP] verdict:** none — no package was proposed.
**Packages flagged as suspicious [SUS]:** none.

A `cargo add` in this phase would be a **defect**, not a choice: it would change `Cargo.lock`, and the lockfile is part of the reproducibility contract that CI enforces with `--locked` `[VERIFIED: .github/workflows/*.yml — "cargo build --locked"]`. If the planner finds itself wanting a crate, that is the signal to re-read this section.

## Architecture Patterns

### System Architecture Diagram

```
                       config/baseline.toml
                                │
                                │  [invariants] liveness_enabled
                                ▼
                       ┌──────────────────┐
                       │ CheckSet::from_  │   ← the ONE place the gate is read
                       │   params()       │      (once per run, at construction)
                       └────────┬─────────┘
                                │ Vec<(CheckId, &str, CheckFn)>
                                │
   caller (Phase 5+ economics)  │
        │                       │
        │ &mut Books            │
        ▼                       │
 ┌───────────────────────┐      │
 │ Books::transfer       │      │
 │  1 compute (fallible) │      │
 │  2 commit  (infallible)      │
 │  3 record  → journal  │      │
 └──────────┬────────────┘      │
            │                   │
   ┌────────▼─────────┐         │
   │ balances         │         │
   │  hh_cash  Vec    │         │
   │  firm_cash Vec   │         │
   │  hh_stock  Vec   │         │
   │  firm_stock Vec  │         │
   │  produced/consumed         │
   └────────┬─────────┘         │
            │                   │
   ┌────────▼──────────────┐    │
   │ journal: Vec<Posting> │    │   each Posting carries the two
   │  (per tick, cleared)  │    │   RUNNING RESIDUALS after it applied
   └────────┬──────────────┘    │
            │                   │
            │  &Books (shared, read-only)
            ▼                   ▼
      ┌─────────────────────────────────┐
      │ invariants: CheckSet::run(&Books, tick)
      │   money → goods → non-neg → zero-sum → liveness
      │   on failure: linear scan of journal for the
      │   FIRST non-zero residual → the offending Posting
      └───────────────┬─────────────────┘
                      │
        Ok(())        │        Err(Violation { tick, account, posting, … })
           │          │                     │
           ▼          │                     ▼
   Books::end_of_tick()                Phase 3 pipeline aborts the run;
   → journal.clear(),                  main.rs maps it to a non-zero exit
     tx_this_tick = 0                  and prints Display to stderr
```

### Recommended Project Structure

```
src/
├── books.rs        # LEDG-01/02/03/07. Balances, stock, journal, transfer/
│                   # produce/consume, PostError, and the #[cfg(test)]
│                   # corruption vocabulary (private, unreachable from tests/)
├── invariants.rs   # LEDG-04…10. Violation, CheckId, ALL_CHECKS, CheckSet,
│                   # the localisation scan. Reads &Books, mutates nothing.
└── lib.rs          # + pub mod books;  + pub mod invariants;

tests/
├── ledger_props.rs      # proptest: conservation under random posting sequences
├── ledger_atomicity.rs  # the catch_unwind atomicity test (LEDG-02, positive)
├── invariant_halt.rs    # library-level: a tick loop aborts at the right tick
└── lints.sh             # EXTENDED: the LEDG-02 borrow probe + the grep guards
```

Whether `books` is a file or a directory is discretion; **a single file is recommended**, because the `#[cfg(test)]` corruption methods must sit in the same module as the private balance fields, and splitting `books/` into submodules forces those fields to `pub(crate)` — which is a wider surface than LEDG-01 wants.

### Pattern 1: The books own the value; agents do not exist yet

`Household` and `Firm` structs are **not** introduced in this phase — Phase 3 owns `world.rs`. LEDG-01's "hold no balance fields and expose no `set_cash`" is therefore, in Phase 2, a claim about a type that does not exist. That is a trap: a grep for `set_cash` trivially passes over an empty set.

**What to do instead:** assert the positive property that will still be checkable in Phase 3 and beyond — *the only writable path to a balance is inside `books.rs`*. Concretely, a source-level guard asserting that no file under `src/` other than `books.rs` names a balance field, plus the fact that the fields are private with no `pub` setter. Record in the module docs that Phase 3 inherits the obligation.

### Pattern 2: LEDG-02 has four legs, not one

`&mut self` is necessary and **not sufficient**. All four must hold:

| Leg | Mechanism | How it is evidenced |
|-----|-----------|---------------------|
| **1. Exclusive borrow** | `transfer(&mut self, …)` — a shared `&Books` cannot coexist with it | Compile-fail probe: `error[E0502]: cannot borrow 'b' as mutable because it is also borrowed as immutable` `[VERIFIED: compiled this session, rustc 1.94.1]` |
| **2. No re-entry** | No `&mut Books` method takes a closure, `impl Fn`, `dyn Trait`, or a callback of any kind; `Books` holds no `dyn Sink` field | A grep guard over `src/books.rs`. **Necessary because a `&mut self` method with a hook DOES leak a mid-transaction view — reproduced this session, the hook observed 50 against an opening 100** |
| **3. No interior mutability** | `books.rs` names no `Cell`, `RefCell`, `Rc`, `Arc`, `Mutex`, `RwLock`, `UnsafeCell` | Add them to `clippy.toml` `disallowed-types` (cheap — see Pitfall 8) **and** a grep guard. `#![forbid(unsafe_code)]` in `lib.rs` closes the raw-pointer route `[VERIFIED: src/lib.rs:7]` |
| **4. Panic-atomicity** | Compute-then-commit: every fallible step runs before any write; the commit phase contains only infallible assignments | A **positive, executable** `catch_unwind` test — see Code Example 3 |

Leg 4 is the one worth the most attention. Measured this session: a naive `transfer` that decrements then panics leaves the books at **-400** against an opening **100**, and `catch_unwind(AssertUnwindSafe(…))` observes it. The compute-then-commit version leaves them at **100** and returns `Err` instead of panicking.

**When to use:** every `&mut Books` method, not only `transfer`. `produce` and `consume` follow the same shape.

### Pattern 3: `transfer` returns the amount actually moved

```rust
pub fn transfer(&mut self, from: Account, to: Account, amount: Money)
    -> Result<Money, PostError>
```

Returning `Money` rather than `()` is what makes LEDG-03's caller obligation expressible: a caller writes `let moved = books.transfer(…)?;` and subtracts `moved`. For the whole-amount case `moved == amount`, so the value looks redundant — it is not, because Phase 8's dividend path and any future partial-payment path (`LABR-08`: "a firm that cannot cover payroll pays what it can") need a `transfer_up_to` sibling whose return genuinely differs from its argument. Give both the same return type from day one so the call sites never need to change shape.

**Note:** LEDG-01 dissolves most of LEDG-03's risk — if no agent holds cash, there is no `firm.cash -= intended` to get wrong. The residual risk is a *derived accumulator* (`payroll_paid_this_month`, `revenue_this_tick`) living on a Phase 6+ struct. Record in `books.rs` docs: **derive such totals from the journal, or bump them by the returned value; never by the intended value.**

### Pattern 4: A `Posting` carries both legs, so zero-sum is a per-posting property

```rust
pub struct Posting {
    pub seq: u32,               // index within this tick's journal
    pub kind: PostingKind,      // Transfer | Produce | Consume | Endow
    pub debit: Account,         // who paid / who lost units
    pub credit: Account,        // who received
    pub cash: Money,            // moved debit → credit
    pub good: GoodId,
    pub units: i64,             // moved credit → debit for a sale
    pub cash_residual: i64,     // total_money - opening_stock, AFTER this posting
    pub goods_residual: i64,    // produced - consumed - Σstock, AFTER this posting
}
```

LEDG-07 ("every sale moves units one way and equal cash the other") is then checkable on a single `Posting` without any aggregation, and — importantly — **without needing an economic notion of "a sale"**, which this phase does not have. The check is: for `PostingKind::Transfer` with `units != 0`, the cash leg and the units leg name the same pair of accounts in opposite directions.

### Pattern 5: Residuals in the journal, so localisation is a scan and not a replay

The naive reading of "bisect the journal" is *replay the tick from a snapshot up to posting k and re-evaluate*. That needs a snapshot of every balance at tick start (220 `Money` = 1.76 KB, cheap) and `O(log J)` replays of `O(J)` postings each.

**Do not do that.** Have `Books::record()` compute the two residuals as it appends. That makes the residual a *field*, localisation a scan over already-computed values, and replay unnecessary. The cost is two `O(n_accounts)` sums per posting — which is where the real cost is, so see Pitfall 9 for the incremental form.

### Anti-Patterns to Avoid

- **Binary search over the journal.** Unsound; see Pitfall 2. Measured wrong answer this session.
- **`debug_assert!` anywhere on the invariant path.** Verified compiled out. A grep guard asserting `src/invariants.rs` and `src/books.rs` contain neither `debug_assert` nor `cfg(debug_assertions)` is the ROADMAP's stated criterion and is cheap.
- **A `pub fn` that returns `&mut Money` or `&mut Vec<Money>`.** It hands the caller the mutation point `transfer` is supposed to monopolise, and no grep for `set_cash` finds it. Guard on the *return type*, not the *name*.
- **An `impl Default for Books`.** It would construct books with a zero opening stock, against which every conservation check trivially passes. There must be exactly one constructor and it must take the stock.
- **Recomputing `opening_stock` from the balances it checks.** See Pitfall 5.
- **Consuming an RNG draw in the invariant phase.** Would shift every downstream sub-stream and silently re-trajectory every run (CORE-04).
- **A "check everything" function that short-circuits on the first `Ok`** or that collects `Vec<Violation>`. First violation wins, matching `Params::validate`'s established convention `[VERIFIED: src/config.rs:137-139 — "The first violation wins. There is no value in enumerating the rest"]`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Checked money arithmetic | A new checked-add helper in `books.rs` | `Money`'s operators (panic in every profile) and `checked_add`/`checked_sub` (return `Result`) | Both halves ship and neither substitutes for the other `[VERIFIED: src/money.rs:1-30 module docs — "Neither half may be deleted in favour of the other."]` |
| Conserving division | A rounding helper for dividends/pro-rata | `Money::split(n)` | The ascending-index remainder rule is a locked trajectory-visible contract `[VERIFIED: src/money.rs:118-131 — "the first `\|remainder\|` recipients — indices `0..\|remainder\|`, ascending — each receive one extra cent… do not \"tidy\" it into a rounding helper"]` |
| Generational firm addressing | A `FirmId` variant or a slot-only key | `ids::FirmId { slot, generation }`, `ids::Account` | `Account`'s own doc comment says it is the type this phase posts against `[VERIFIED: src/ids.rs:64-66 — "The addressing type Phase 2's ledger posts against, so that a household and a firm sharing an underlying index are never the same account."]` |
| Firm storage with respawn safety | A `Vec<Firm>` with manual generation tracking | `ids::FirmArena<T>` | Exposes **no** element-removal operation at all `[VERIFIED: src/ids.rs:84-91]`. Note: Phase 2 stores *balances* keyed by `FirmSlot`, not occupants — see Pitfall 10 |
| Error enums with formatted messages | `impl Display` by hand | `thiserror` 2.0.20 `#[error("…")]` | Nested `Display` interpolation verified this session |
| Compile-failure testing | Adding `trybuild` | Extend `tests/lints.sh` with a probe file under `tests/lint-probes/` | The idiom, the `trap`-based restore and the CI wiring already exist `[VERIFIED: tests/lints.sh:36-52; .github/workflows/*.yml runs `bash tests/lints.sh`]` |
| Fault injection | A cargo feature | `#[cfg(test)]` methods in `src/books.rs` | Verified: unreachable from `tests/` (`E0599`), reachable from unit tests including private fields |

**Key insight:** Phase 1 deliberately built the vocabulary this phase needs and then stopped. Every primitive listed above already carries a tested contract and a documented rationale. A new helper here is not merely redundant — it is a *second* rule for the same operation, and the two will diverge in a way that shows up as an emergent trajectory difference rather than a test failure.

## Configuration Surface

LEDG-08's gate is a config key. CORE-10 makes that a four-file change with three tests watching, so the planner must budget for it.

**Recommended shape** — a new table, one key:

```toml
[invariants]
# Whether the liveness check runs. OFF for Phase 3's pre-economics empty run
# (3,650 ticks with zero transactions); ON from Phase 6, when wages make the
# first money move.
# GRADE: PROJECT | SOURCE: ROADMAP Phase 2 criterion 3 (cross-phase constraint recorded by the roadmapper) | CADENCE: none
liveness_enabled = false
```

**Why a new `[invariants]` table rather than a key under `[sim]`:** the deferred journal-to-disk flag lands in the same table in Phase 3, and a `[sim]` table that accumulates diagnostic switches next to `households` and `seed` blurs "what the economy is" with "what we are watching".

**Why a bool and not `liveness_min_transactions_per_tick: u32` with 0 meaning off:** a magic-value-means-disabled encoding is a hidden second parameter. If a minimum above 1 is ever wanted, add a second key then.

**Why not a CLI flag or env var:** `main.rs` has exactly three flags and `--seed`/`--out` affect nothing else `[VERIFIED: src/main.rs:17-30]`; an env override is "an input that is neither in the committed config nor the log", explicitly rejected project-wide.

**The four-file change and the three tests that watch it:**

1. `config/baseline.toml` — the table, the key, and **exactly two preceding comment lines**: a human description, then the `# GRADE: … | SOURCE: … | CADENCE: …` line. `tests/provenance.rs::every_key_has_a_source_grade` and `::no_annotation_is_orphaned` enforce the shape `[VERIFIED: config/baseline.toml:18-24 — "Each key below is preceded by exactly two comment lines: a human description, then a machine-checkable line of the shape…"]`.
2. `src/config.rs` — a `pub struct Invariants { pub liveness_enabled: bool }` with `#[serde(deny_unknown_fields)]`, and `pub invariants: Invariants` on `Params`. **No `Option`, no `#[serde(default)]`** — `tests/config_strict.rs::no_optional_fields_in_the_config_schema` greps `src/config.rs` for `Option<` and fails on it `[VERIFIED: tests/config_strict.rs:409-420]`.
3. `config/PROVENANCE.md` — a row for `invariants.liveness_enabled`, or `tests/provenance.rs::every_config_key_has_a_provenance_row` fails.
4. `src/config.rs` `Params::validate` — nothing to validate for a bool; no change. Note that the doc-comment convention there is that "Bounds that are not imposed by a consumer in this crate are deliberately absent" `[VERIFIED: src/config.rs:133-136]`.

**Also true, and worth stating so nobody panics:** `tests/config_strict.rs::every_key_is_required` deletes each leaf key in turn and asserts a `missing field` error naming it. A `bool` leaf behaves identically to the existing integers, and the test's floor assertion is `paths.len() >= 40` — a *lower* bound, so adding a key does not break it `[VERIFIED: tests/config_strict.rs:135-139]`. `the_schema_and_the_shipped_config_name_the_same_leaves` passes as long as steps 1 and 2 are both done — it exists precisely to catch doing only one.

**Changing the config changes the config hash**, which is taken over the raw file bytes including comments `[VERIFIED: src/config.rs:14-17]`. That is correct and expected; no golden log exists yet to invalidate.

## Error Type Design

Two error types, both `thiserror` 2.0.20, both in the library (never `anyhow`).

### `books::PostError` — a refused posting

The books declined to act; **nothing was written**. This is a legitimate runtime condition (an overdraft is an economic event, not a bug).

```rust
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PostError {
    #[error("{account} cannot pay {amount_cents} cents: balance is {balance_cents} cents")]
    Overdraft { account: Account, amount_cents: i64, balance_cents: i64 },

    #[error("{account} cannot ship {units} units of good {good}: stock is {stock} units")]
    ShortStock { account: Account, good: GoodId, units: i64, stock: i64 },

    #[error("{0} is not an account in these books")]
    UnknownAccount(Account),
}
```

`Copy` is achievable because every field is `Copy` — keep it that way (no `String`) so a `PostError` can be logged, matched on and returned without a clone.

### `invariants::Violation` — the books are wrong

A bug, always. Every variant carries **tick, the named agent, and the offending posting** — the three things LEDG-09 requires printed. Verified this session that `thiserror` renders a nested `Display` field correctly; the actual output was:

```
tick 1234: money conservation broken by - 1 cents (books hold 1999999, opening stock 2000000); offending posting #97 transfer household:0 -> firm:0:0 cash=1c units=0 good=0
```

```rust
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Violation {
    #[error("tick {tick}: money conservation broken by {delta_cents} cents \
             (books hold {actual_cents}, opening stock {expected_cents}); \
             offending posting {posting}")]
    MoneyConservation {
        tick: u32, expected_cents: i64, actual_cents: i64, delta_cents: i64,
        posting: Posting,
    },

    #[error("tick {tick}: goods conservation broken by {delta_units} units of good {good} \
             (produced {produced} - consumed {consumed} - stock {stock}); \
             offending posting {posting}")]
    GoodsConservation {
        tick: u32, good: GoodId, produced: i64, consumed: i64, stock: i64,
        delta_units: i64, posting: Posting,
    },

    #[error("tick {tick}: {account} holds a negative {field}: {value}; \
             offending posting {posting}")]
    Negative {
        tick: u32, account: Account, field: NegativeField, value: i64,
        posting: Posting,
    },

    #[error("tick {tick}: posting {posting} is not zero-sum: {detail}")]
    ZeroSum { tick: u32, posting: Posting, detail: ZeroSumDetail },

    #[error("tick {tick}: liveness — {counted} transactions recorded, at least {required} required")]
    Liveness { tick: u32, counted: u32, required: u32 },
}
```

Three deliberate choices in that shape:

- **`Posting` is embedded by value, not by index.** An index into a buffer that `end_of_tick` is about to clear is a dangling reference in all but name; by the time a human reads the message the journal is gone.
- **`field: NegativeField` and `detail: ZeroSumDetail` are small enums, not `String`.** Keeps `Violation` `Clone + PartialEq + Eq`, which is what lets a test assert the exact expected violation rather than substring-matching a message. Substring-matching an error message is how a negative test passes for the wrong reason.
- **`Liveness` carries no `Posting`** — by construction there was none. Do not invent a placeholder posting to make the variants uniform; a synthetic posting in an error message is a lie a future reader will chase.

**`Violation` must NOT be `Copy`** — leave the derive off, because a future variant may need a `Vec` and `Copy` is a compatibility promise this type has no reason to make.

## Negative Test Design

The phase gate. Four violation classes, seeded deliberately, each proven to halt.

### The mechanism: `#[cfg(test)]` in `src/books.rs`

**Verified this session, both directions:**
- A `#[cfg(test)] pub fn corrupt(&mut self)` on `Books` is callable from a `#[cfg(test)] mod` in the same file, and that test can *also* write private fields directly (`b.cash[1] -= 1`).
- The identical call from `tests/it.rs` fails to compile: `error[E0599]: no method named 'corrupt' found for struct 'Books' in the current scope`.

So the corruption vocabulary is **absent from every build that is not the crate's own unit-test build**, enforced by the compiler rather than by a feature flag, a review, or a grep. No production hole exists to leave.

The four seeded corruptions, all as private `#[cfg(test)]` methods on `Books`:

| Class | Requirement | Seeding | Expected `Violation` |
|-------|-------------|---------|----------------------|
| **A dropped cent** | LEDG-04 | Decrement one account's cash by 1 without a matching credit | `MoneyConservation { delta_cents: -1, … }` naming the account and the last posting before the residual went non-zero |
| **An over-credited sale** | LEDG-04 + LEDG-07 | Credit the buyer's counterparty more than was debited | `MoneyConservation { delta_cents: +n }` — and, if the corruption is applied *as a posting*, `ZeroSum` fires first if it is ordered first. **Choose the check order deliberately; see below.** |
| **A driven-negative balance** | LEDG-06 | Set one account's cash below zero *while keeping the total intact* (move the deficit to another account) | `Negative { account, field: Cash, value }` — and crucially **not** `MoneyConservation`, because the total still conserves. This is the case that proves the two checks are independent |
| **A non-zero-sum trade** | LEDG-07 | Append a posting whose cash leg and units leg disagree in direction or counterparty | `ZeroSum { posting, detail }` |

Plus the fifth, which needs **no corruption at all**:

| **Nothing traded** | LEDG-08 | Run a tick with the liveness check enabled and issue no postings | `Liveness { counted: 0, required: 1 }` |

The liveness case is the one violation reachable through the entirely public API. That makes it the natural **end-to-end** halt demonstration, and it is why the config gate exists.

### Check ordering is part of the contract

Because a single corruption can trip more than one check, the *order* of `ALL_CHECKS` decides which `Violation` a test sees. Verified this session: with the order money → goods → non-negative → zero-sum → liveness, a dropped cent reports `money 999 != 1000` and a negative-balance-with-conserved-total reports `account 1 is negative`. Both are the diagnostically useful answer.

Recommended order and why: **money conservation first**, because a leak is the highest-severity finding and reporting it as "some account went negative" would send a debugger to the wrong place. **Liveness last**, because it is the only check that can fire on books that are entirely correct.

Write an order test that reads `ALL_CHECKS` and asserts the exact `CheckId` sequence, mirroring Phase 3's `const PHASES` order test.

### What "halts the run" means in a phase with no run loop

Phase 2 has no tick pipeline — Phase 3 owns `const PHASES` and `main.rs` is a tracer `[VERIFIED: src/main.rs:32-62]`. So the phase gate splits:

| Level | What Phase 2 can prove | Where |
|-------|------------------------|-------|
| **Unit** | Each of the four corruptions produces the exactly-right `Violation` value (`assert_eq!` on the enum, not a substring match) | `src/books.rs` / `src/invariants.rs` `#[cfg(test)] mod` |
| **Integration** | A library-level tick loop *aborts at the right tick* and does not continue — i.e. the `?` really propagates and the loop really stops | `tests/invariant_halt.rs`, driving `Books` + `CheckSet` in a `for tick in 0..N` loop with `?` |
| **Process** | A non-zero exit code with `Display` on stderr | **Phase 3.** Design for it now: keep `Violation` in the library so `main.rs` can `?` it through `anyhow` |

**Recommend the planner record this as an explicit cross-phase constraint**, in the same way the roadmapper recorded the liveness gate: *Phase 3's negative test runs the built binary with `liveness_enabled = true` against the empty economy and asserts a non-zero exit plus a stderr line naming tick 0.* That test is free once the pipeline exists, needs no fault injection at all, and closes the process level of criterion 2. Without it recorded, the process level falls through the gap between the two phases.

## Making LEDG-02 Provable

The ROADMAP phrases criterion 1 as "a test observing the books mid-transaction is impossible to write". An unfalsifiable criterion cannot be verified, only asserted. Convert it into four checkable facts:

### 1. The borrow probe (compile-fail) — positive evidence

Add `tests/lint-probes/books_borrow_probe.rs.txt`:

```rust
use sim::books::Books;
#[test]
fn observing_the_books_mid_transaction() {
    let mut books = Books::new(/* … */);
    let watcher = &books;                    // shared borrow, held live
    books.transfer(from, to, amount).unwrap();
    let _ = watcher.total_money();           // shared borrow used after
}
```

`tests/lints.sh` copies it in, asserts `cargo build --tests` **fails**, asserts the output contains `E0502`, and removes it under the existing `trap`. Verified this session that exactly this shape produces:

```
error[E0502]: cannot borrow `b` as mutable because it is also borrowed as immutable
```

The `E0502` string assertion matters: a bare "the build failed" assertion would stay green if the probe stopped compiling for an unrelated reason (a renamed constructor), which is the same hole `tests/lints.sh` already documents for its own check 2.

### 2. The no-callback grep — closes the hole leg 1 leaves open

**Reproduced this session:** a `&mut self` method taking `hook: impl Fn(&B)` handed the hook a mid-transaction view (total 50 against opening 100) and compiled and ran clean. So leg 1 alone is *false*.

Guard: assert that no line in `src/books.rs` inside an `impl Books` signature names `impl Fn`, `impl FnMut`, `impl FnOnce`, `dyn Fn`, `&dyn `, or `Box<dyn`. State the reason in the guard's failure message, because the next person to add a `&mut Books` method with a logging hook will otherwise read the guard as arbitrary.

### 3. The no-interior-mutability grep + lint

Assert `src/books.rs` names none of `RefCell`, `Cell`, `UnsafeCell`, `Rc<`, `Arc<`, `Mutex`, `RwLock`, `OnceCell`, `LazyCell`. Additionally add the four std ones to `clippy.toml` `disallowed-types` — **verified cheap**: `tests/lints.sh` check 3 counts *disallowed-**methods*** call sites only, and check 2 asserts merely that *some* disallowed-type diagnostic appears, so new `disallowed-types` entries require no probe update `[VERIFIED: tests/lints.sh:145-175 — MARKED counts `// BANNEDCALL$` in the float probe; check 2 case-matches "use of a disallowed type"]`.

`#![forbid(unsafe_code)]` already closes the raw-pointer route `[VERIFIED: src/lib.rs:7]`.

### 4. The atomicity test — positive, executable, and the strongest of the four

See Code Example 3. This is the one that turns "impossible to write" into "here is the test, it passes, and here is the mutant that fails it".

## Goods Conservation Shape (LEDG-05)

The roadmapper flagged an open question: is purchased food consumed immediately, or held as household stock? Phase 7 resolves it (MKT-06). **The identity must keep one shape either way**, and it can:

### The recommendation: inventory is per-`Account`, not per-firm

```
produced − consumed − Σ_{a ∈ all accounts} stock[a][g] == 0     for each good g
```

Store stock in the same two-vector shape as cash — `hh_stock: Vec<i64>` and `firm_stock: Vec<i64>` (per good, when a second good arrives: `Vec<Vec<i64>>` indexed by `GoodId`). Then:

- **Immediate-consumption model (Phase 7 option A):** a purchase is `ship` (firm stock → household stock) immediately followed by `consume(household, good, units)`. `hh_stock` returns to zero within the tick. The identity is unchanged.
- **Household-stock model (Phase 7 option B):** the purchase ships; `consume` happens on a later tick against held stock. `hh_stock` is non-zero across ticks. The identity is unchanged.

The *only* difference between the two worlds is whether a household's stock slot is ever non-zero at the moment the invariant runs. **No formula, no field, and no check changes.** That is the "one shape" the SUMMARY asks for `[VERIFIED: .planning/research/SUMMARY.md § Gaps to Address — "Purchased food: consumed immediately or held as household inventory? … Handle: model `consume` explicitly either way so the identity keeps one shape; resolve in Phase 7."]`.

### `consume` must be a real posting, not a subtraction

`consume(account, good, units)` decrements that account's stock **and** increments `consumed`, **and** appends a `PostingKind::Consume` posting. If it only did the first two, a consumption bug would be invisible in the journal and LEDG-09 could not name it. This is also what MKT-06 asks for ("Consumption is an explicit modelled step").

**Note for the planner:** `produced` and `consumed` are running totals maintained *independently* of the stock vectors. That independence is what makes LEDG-05 a real check rather than a tautology — two separately-maintained tallies are compared. Do **not** "simplify" by deriving `produced` from the journal at check time; then both sides come from one source and the check proves nothing.

### The corollary for `Books::new`

Initial inventory (`firm.initial_inventory_units` exists in the config `[VERIFIED: src/config.rs — "pub initial_inventory_units: i64"]`) must be booked as `PostingKind::Endow` and counted in `produced`, or the identity fails at tick 0 by exactly the endowment. The same is true of `money.total_money_cents` versus `opening_stock`. State both in `Books::new`'s doc comment — this is the single most likely "it fails on tick 0 and nobody knows why" defect in the phase.

## Common Pitfalls

### Pitfall 1: `&mut self` is assumed sufficient for atomicity

**What goes wrong:** the plan asserts LEDG-02 holds "because `transfer` takes `&mut self`", ships, and a later phase adds a logging or metrics hook parameter that silently reopens the hole.
**Why it happens:** the borrow checker's guarantee is about *external* observers. A callback is an *internal* one and is exempt by construction.
**How to avoid:** the four-leg construction in Pattern 2; specifically the no-callback grep guard.
**Warning signs:** any `&mut Books` method whose signature has more than the accounts and the amount; any `Books` field of a trait-object type.
**Evidence:** reproduced this session — hook observed total 50 against opening 100, program exited 0.

### Pitfall 2: Bisection over the journal names the wrong posting

**What goes wrong:** a binary search for the first non-zero residual assumes the predicate has monotone onset. Residuals cancel. The search then reports a later, healthy-looking posting and a debugger spends a day at the wrong place in the tick.
**Why it happens:** LEDG-09 says the word "bisected", and binary search is the reflexive reading of it.
**How to avoid:** linear `iter().position(|p| p.residual != 0)`. Measured at **80 ns/tick** for 274 postings, which is *less* than the conservation recompute it accompanies.
**Warning signs:** any `while lo < hi` over the journal.
**Evidence:** measured this session — journal broken at #50, healed at #120, broken differently at #200. Linear scan → `Some(50)`. Bisection → `200`.

### Pitfall 3: A float type name in `books.rs` or `invariants.rs` fails an existing test

**What goes wrong:** `tests/numeric_det.rs::confinement_of_the_float_domain` reads every file under `src/` and fails any non-allowlisted file that names `f16`/`f32`/`f64`/`f128` **as a whole word anywhere on the line, comments included**. The allowlist is exactly `["numeric.rs", "config.rs"]` `[VERIFIED: tests/numeric_det.rs:88-91, 154-168 — "const FLOAT_ALLOWLIST: [&str; 2] = ["numeric.rs", "config.rs"];" and `names_a_float_type` reads the raw `line`, not `without_line_comment(line)`]`.
**Why it happens:** the natural way to write "money is never `f64` here" in a module doc comment is to *write* `f64`.
**How to avoid:** word the `books.rs` and `invariants.rs` module docs around the type names, exactly as `src/numeric.rs` was worded around `powf`/`exp`/`ln`/`log` `[VERIFIED: STATE.md — "src/numeric.rs contains no occurrence of the substrings powf, exp, ln or log anywhere, including in prose"]`. Say "floating point" or "the float domain", never the type name.
**Also:** float *literals* are checked only after stripping from the first `//`, so `0.25` in a comment is fine but `0.25` in code is not `[VERIFIED: tests/numeric_det.rs:124-151]`. Money and units are integers, so this should never arise in code.
**Warning signs:** the phrase "not an f64" anywhere in a doc comment.

### Pitfall 4: LEDG-01 is verified against types that do not exist yet

**What goes wrong:** the plan adds a `grep -r "set_cash" src/` guard, it passes trivially because `Household` and `Firm` are Phase 3's, and the guard is then believed to be protecting something.
**How to avoid:** Pattern 1 — assert the positive property (only `books.rs` may write a balance) and record the inherited obligation in the module docs so Phase 3's planner meets it.
**Warning signs:** any Phase 2 guard whose search set is empty. A guard over an empty set must itself assert the set is non-empty — the discipline `tests/lints.sh` already applies (`if [ "${#RUST_SOURCES[@]}" -eq 0 ]; then fail …`) `[VERIFIED: tests/lints.sh:214-217]`.

### Pitfall 5: The conservation baseline is derived from the thing it checks

**What goes wrong:** `opening_stock` is computed as `sum(balances)` at tick 0 by the same code path that computes it every tick. The check then compares a number to itself and passes forever.
**Why it happens:** it looks like DRY.
**How to avoid:** `opening_stock: Money` is a field set once in `Books::new` **from `params.money.total_money_cents`** — the config value, an input independent of the balances. Then assert at construction that the endowment sums to it, and fail loudly if not. That construction-time assertion is a different check from the per-tick one and both are needed.
**Warning signs:** `Books::new` with no money parameter; an `opening_stock` that is `mut`.

### Pitfall 6: A derived accumulator reintroduces LEDG-03

**What goes wrong:** Phase 6+ adds `payroll_paid: Money` to a firm and bumps it by the *intended* wage rather than the *transferred* amount; a partial payment (LABR-08) then leaks in the accumulator while the ledger itself stays perfect.
**How to avoid:** `transfer` returns the amount moved (Pattern 3); document the rule in `books.rs` where a Phase 6 author will read it; prefer deriving such totals from the journal.
**Warning signs:** any `Money` field outside `books.rs` that is added to rather than assigned.

### Pitfall 7: The non-negativity check silently covers nothing

**What goes wrong:** LEDG-06 names "cash, inventory and headcount". Headcount has no owner in Phase 2 — no employment relation exists until Phase 6. A check written as "iterate the headcounts" over an empty structure passes vacuously and nobody notices when Phase 6 adds headcount somewhere else.
**How to avoid:** two options, and the plan must pick one explicitly. (a) Check cash and inventory now, and record headcount as an **inherited obligation on Phase 6** in both the module docs and the roadmap note. (b) Introduce `headcount: Vec<u32>` in `Books` now, as the books' third quantity, so Phase 6 has nowhere else to put it. **(b) is recommended** — it is consistent with "the books own every quantity that must conserve", it is trivially non-negative by using `u32`, and it removes the cross-phase promise entirely. Note that a `u32` headcount makes the non-negativity check for that column a type-level fact rather than a runtime one; say so in the docs rather than writing an unreachable check.
**Warning signs:** a check whose loop body is never entered in any test.

### Pitfall 8: The new `disallowed-types` entries are assumed expensive

**What goes wrong (in reverse):** the planner *skips* adding `RefCell`/`Rc`/`Cell`/`Mutex` to `clippy.toml`, fearing it must regenerate the 66-path float list or update the probe's `// BANNEDCALL` counts.
**Why it is cheap:** `tests/lints.sh` check 3 counts only `use of a disallowed method` diagnostics against `// BANNEDCALL$` markers in the float probe; the types list is asserted separately and only by presence `[VERIFIED: tests/lints.sh:145-181]`. Adding a type entry touches neither count.
**How to avoid:** just add them, with a reason string naming LEDG-02.

### Pitfall 9: The residual recompute is put inside `record()` naively

**What goes wrong:** Pattern 5 asks each `Posting` to carry the residuals after it is applied. Computed as a full `O(n_accounts)` sum per posting, that is 274 postings × 220 accounts × 3,650 ticks ≈ 2.2 × 10⁸ adds for the decade — still fast, but 300× the cost of the per-tick check and now on the hot path of every economic phase.
**How to avoid:** maintain the residuals **incrementally**. `record()` knows the posting's own net cash delta (zero for a conserving transfer, non-zero only if a caller managed to break it) and its own net units delta. Carry `running_cash_residual` and `running_goods_residual` as `Books` fields, add the posting's delta, and store the result on the posting. That is O(1) per posting.
**And then the per-tick full recompute stays**, because it is the independent cross-check: the incremental residual is derived from the *postings*, the per-tick sum is derived from the *balances*, and the invariant is that they agree. Two sources, genuinely independent — which is exactly what makes the check non-vacuous.
**Warning signs:** `self.total_money()` called inside `record()`.

### Pitfall 10: Firm balances are keyed by `FirmId` and stale across a respawn

**What goes wrong:** a `BTreeMap<FirmId, Money>` keyed on the full generational ID. Phase 10 respawns slot 3, the new firm's `FirmId` has generation 1, and its balance lookup misses — the money of generation 0 is orphaned in the map and conservation breaks at exactly the tick a firm goes bankrupt.
**How to avoid:** key **balances** on `FirmSlot` (the position, stable for the whole run `[VERIFIED: src/ids.rs:33-35 — "Stable for the whole run: a slot is reused by a respawned firm, never removed and never moved."]`), and key **postings** on `Account` (the full identity, so the journal records *which* firm). The two keyings are deliberately different and the reason must be in the docs, or someone will "unify" them.
**Warning signs:** any `Vec` or map of balances indexed by `FirmId` rather than `FirmSlot`.

### Pitfall 11: A negative test that asserts on a message substring

**What goes wrong:** `assert!(err.to_string().contains("conservation"))` passes when the *wrong* check fired, when the tick is wrong, and when the posting named is the wrong one.
**How to avoid:** `Violation` derives `PartialEq + Eq + Clone` (Error Type Design), so assert the whole value: `assert_eq!(err, Violation::MoneyConservation { tick: 7, delta_cents: -1, posting: expected, … })`. Keep **one** separate test that asserts the rendered `Display` contains tick, agent and posting — that one is testing the message contract, which is a different thing and is what LEDG-09 literally requires printed.

## Code Examples

All five compiled and ran on `rustc 1.94.1` this session. They are illustrative skeletons, not drop-in code — the real ones use `Money`, `Account` and `GoodId` from Phase 1.

### 1. Compute-then-commit `transfer` (LEDG-02, LEDG-03)

```rust
/// THE only cash-mutation point. Compute-then-commit: every fallible step
/// runs before any write, so a refusal leaves the books exactly as they were.
/// Returns the amount actually moved — callers subtract THIS, never the
/// amount they asked for (LEDG-03).
pub fn transfer(&mut self, from: Account, to: Account, amount: Money)
    -> Result<Money, PostError>
{
    // --- compute phase: reads only, no mutation whatsoever ---------------
    let from_before = self.cash_of(from).ok_or(PostError::UnknownAccount(from))?;
    let to_before   = self.cash_of(to).ok_or(PostError::UnknownAccount(to))?;

    let from_after = from_before.checked_sub(amount).map_err(|_| PostError::Overdraft {
        account: from, amount_cents: amount.cents(), balance_cents: from_before.cents(),
    })?;
    if from_after < Money::ZERO {
        return Err(PostError::Overdraft {
            account: from, amount_cents: amount.cents(), balance_cents: from_before.cents(),
        });
    }
    let to_after = to_before.checked_add(amount).map_err(|_| /* … */)?;

    // --- commit phase: infallible assignments only -----------------------
    *self.cash_mut(from).expect("resolved in the compute phase") = from_after;
    *self.cash_mut(to).expect("resolved in the compute phase")   = to_after;
    self.record(PostingKind::Transfer, from, to, amount, GoodId(0), 0);
    Ok(amount)
}
```

Note `checked_sub`/`checked_add` — the **named `Result` half** of `Money`'s split API, not the operators. The operators panic, and a panic in the compute phase would be fine but a panic in the *commit* phase would not be; using the non-panicking half throughout makes the distinction unnecessary to reason about.

### 2. The atomicity mutant (what the test must fail on)

```rust
// The BAD design, kept only in the test as the mutant.
fn transfer_naive(&mut self, amount: i64) {
    self.a -= amount;
    assert!(self.a >= 0, "overdraft");   // panics AFTER a write
    self.b += amount;
}
```

### 3. The executable LEDG-02 atomicity test

```rust
use std::panic::{catch_unwind, AssertUnwindSafe};

#[test]
fn a_refused_transfer_leaves_the_books_exactly_as_they_were() {
    let mut books = Books::new(/* opening stock 1000 cents */);
    let before = books.total_money();

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        books.transfer(h0, f0, Money::from_cents(5_000))   // more than h0 holds
    }));

    // It returned an error; it did NOT panic; and nothing moved.
    assert!(outcome.is_ok(), "transfer panicked instead of returning Err");
    assert!(matches!(outcome.unwrap(), Err(PostError::Overdraft { .. })));
    assert_eq!(books.total_money(), before);
    assert_eq!(books.cash_of(h0), Some(before_h0));
    assert_eq!(books.cash_of(f0), Some(before_f0));
}
```

Measured this session on the two designs: **naive → total −400 against an opening 100, and `catch_unwind` observed it. Compute-then-commit → total 100, `Err("overdraft")`, no panic.**

### 4. Localisation — linear, not bisected (LEDG-09)

```rust
/// The first posting after which the residual was non-zero.
///
/// A LINEAR scan, deliberately. A binary search assumes the residual has a
/// monotone onset; it does not — a dropped cent healed later by an equal
/// over-credit returns the residual to zero, and the search then names a
/// later, unrelated posting. Measured: broken at #50, healed at #120, broken
/// again at #200 — bisection answers 200, the linear scan answers 50.
fn first_breaking_posting(journal: &[Posting], residual: fn(&Posting) -> i64)
    -> Option<&Posting>
{
    journal.iter().find(|posting| residual(posting) != 0)
}
```

### 5. Check set built once from config, iterated with no per-tick conditional (LEDG-08)

```rust
type CheckFn = fn(&Books, u32) -> Result<(), Violation>;

/// The full, ordered table. The ORDER is part of the contract: one test
/// asserts this exact CheckId sequence, as Phase 3's `const PHASES` does.
const ALL_CHECKS: [(CheckId, &str, CheckFn); 5] = [
    (CheckId::MoneyConservation, "money_conservation", check_money),
    (CheckId::GoodsConservation, "goods_conservation", check_goods),
    (CheckId::NonNegative,       "non_negative",       check_non_negative),
    (CheckId::ZeroSum,           "zero_sum",           check_zero_sum),
    (CheckId::Liveness,          "liveness",           check_liveness),
];

pub struct CheckSet { active: Vec<(CheckId, &'static str, CheckFn)> }

impl CheckSet {
    /// The ONE place the liveness gate is read, for the whole crate.
    pub fn from_params(params: &Params) -> CheckSet {
        let live = params.invariants.liveness_enabled;
        CheckSet {
            active: ALL_CHECKS.iter().copied()
                .filter(|(id, _, _)| live || *id != CheckId::Liveness)
                .collect(),
        }
    }

    /// The per-tick path: no conditional, no config lookup, no flag branch.
    pub fn run(&self, books: &Books, tick: u32) -> Result<(), Violation> {
        for (_, _name, check) in &self.active { check(books, tick)?; }
        Ok(())
    }
}
```

Verified output this session:

```
CHECKSET off = ["money_conservation", "goods_conservation", "non_negative", "zero_sum"]
CHECKSET on  = [… , "liveness"]
empty tick, liveness OFF -> Ok(())
empty tick, liveness ON  -> Err("tick 0: liveness — 0 transactions")
negative balance, money still conserves -> Err("tick 42: account 1 is negative")
dropped cent -> Err("tick 7: money 999 != 1000")
```

The last two lines matter: they show the two checks are **independent** — a conserved total with a negative account is caught by non-negativity and not by conservation, and vice versa.

## Cost Analysis

All measured this session, release profile with `overflow-checks = true`, on this machine.

| Operation | Per tick | Per 3,650-tick decade | Note |
|-----------|----------|------------------------|------|
| Money conservation recompute (220 accounts, `checked_add` fold, balances mutated each tick so the sum cannot be hoisted) | **175 ns** | **0.64 ms** | The CONTEXT's "~220 `i64` adds per tick" estimate is confirmed and the cost is negligible |
| Linear journal scan for the first non-zero residual (274 postings) | **80 ns** | **0.29 ms** | Cheaper than the recompute it accompanies |
| Incremental residual maintenance in `record()` | O(1) per posting | ~10⁶ ops | Pitfall 9 |
| Naive full recompute inside `record()` | ~48 µs | ~176 ms | 300× worse; avoid (Pitfall 9) |

Even the naive variant would finish a decade in under a second. **There is no performance reason to weaken any check, and the plan should not entertain a "sampled" or "every N ticks" invariant.** The measured numbers exist so that a future "this is too slow" argument has to contend with data.

## Runtime State Inventory

> Not a rename/refactor/migration phase — this is new-module construction on a greenfield surface. Section retained with explicit findings because the phase does touch committed artefacts.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | **None.** The project writes no datastore yet; `runs/` output is Phase 3's. Verified: `src/main.rs` creates `--out` but writes no file | none |
| Live service config | **None.** No external service exists — verified: `Cargo.toml` has no network crate; the only dependencies are `rand`, `serde`, `toml`, `sha2`, `thiserror`, `clap`, `anyhow` | none |
| OS-registered state | **None.** No scheduler task, service or daemon — verified: repo contains only `.github/workflows/` as automation | none |
| Secrets / env vars | **None.** The sim reads no env var; all input is the config file plus `--seed` (project rule, `src/main.rs`) | none |
| Build artifacts | `target/` only, and it is not committed. **But:** `Cargo.lock` and the config hash **do** change — the config hash is over `config/baseline.toml`'s raw bytes, so adding `[invariants]` changes it | Expected and correct. No golden log or `insta` snapshot exists yet to invalidate `[VERIFIED: no `snapshots/` directory; no `runs/` directory; `insta` is not yet a dependency]` |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `rustc` / `cargo` | everything | ✓ | **1.94.1 (e408947bf 2026-03-25)** — matches `rust-toolchain.toml` exactly | — |
| `cargo clippy` | the lint gate | ✓ | pinned via `rust-toolchain.toml` `components` | — |
| `cargo fmt` | CI formatting step | ✓ | same | — |
| `bash` | `tests/lints.sh`, `tests/toolchain.sh` | ✓ | — | — |
| `git` | `tests/lints.sh` check 4b (`git ls-files`) | ✓ | — | — |
| crates.io network | **not needed** | n/a | offline build verified working from the local registry cache this session | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `libtest` (`cargo test`), rustc **1.94.1**, plus `proptest` **1.11.0** for properties |
| Config file | `Cargo.toml` (`[dev-dependencies] proptest = "1.11.0"`); regressions in `.proptest-regressions/`, committed |
| Quick run command | `cargo test --locked --lib books invariants` |
| Full suite command | `cargo test --locked --all-targets && cargo test --locked --release --all-targets && bash tests/lints.sh && bash tests/toolchain.sh && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check` |

**What this phase needs beyond what Phase 1 established:** nothing new is installed. Three additions to existing infrastructure:

1. `tests/lints.sh` gains the **LEDG-02 borrow probe** (a new `tests/lint-probes/books_borrow_probe.rs.txt`) and three grep guards (no callback in a `&mut Books` signature; no interior-mutability type in `books.rs`; no `debug_assert`/`cfg(debug_assertions)` in `books.rs` or `invariants.rs`). The `trap`-based restore and CI wiring already exist.
2. `clippy.toml` gains four `disallowed-types` entries (`RefCell`, `Cell`, `Rc`, `Mutex` — and `Arc`, `RwLock`, `UnsafeCell`, `OnceCell` if the planner wants the full set). Verified cheap: no probe count changes.
3. Three new test files under `tests/` (`ledger_props.rs`, `ledger_atomicity.rs`, `invariant_halt.rs`), plus `#[cfg(test)] mod` blocks in `src/books.rs` and `src/invariants.rs`.

**The release-profile run is not redundant** and CI already does it `[VERIFIED: .github/workflows/*.yml — "Test (release)" step runs `cargo test --locked --release --all-targets`]`. For this phase it is the *primary* profile: LEDG-10's whole claim is about what the release binary contains.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LEDG-01 | Balances are private to `books`; only `books.rs` writes one; no `set_cash`, no `&mut Money` escapes | source guard (grep) + unit | `bash tests/lints.sh` | ❌ Wave 0 (extend `tests/lints.sh`) |
| LEDG-02 (leg 1) | A shared borrow held across a `transfer` is `E0502` | **compile-fail** probe | `bash tests/lints.sh` | ❌ Wave 0 (`tests/lint-probes/books_borrow_probe.rs.txt`) |
| LEDG-02 (leg 2) | No `&mut Books` method takes a callback | source guard (grep) | `bash tests/lints.sh` | ❌ Wave 0 |
| LEDG-02 (leg 3) | `books.rs` names no interior-mutability type | source guard + clippy | `cargo clippy --all-targets --all-features -- -D warnings` | ❌ Wave 0 (`clippy.toml` entries) |
| LEDG-02 (leg 4) | A refused/failing transfer leaves the books byte-identical | **integration** (`catch_unwind`) | `cargo test --release --test ledger_atomicity` | ❌ Wave 0 |
| LEDG-03 | `transfer` returns the amount moved and it equals the books' delta, for all inputs | **property** (`proptest`) | `cargo test --release --test ledger_props transfer_return_matches_delta` | ❌ Wave 0 |
| LEDG-03 | `Money::split` parts sum exactly to the whole | property — **already passing from Phase 1** | `cargo test --lib money::split` | ✅ `src/money.rs` `mod split_tests` |
| LEDG-04 | Total money equals opening stock after every tick, for any random posting sequence | **property** (`proptest`) | `cargo test --release --test ledger_props conservation_under_random_postings` | ❌ Wave 0 |
| LEDG-04 | A seeded dropped cent produces exactly `Violation::MoneyConservation` | **negative** unit | `cargo test --release --lib invariants::negative` | ❌ Wave 0 |
| LEDG-05 | `produced − consumed − Σ stock == 0` per good, in both consumption models | unit + **property** | `cargo test --release --test ledger_props goods_identity_holds` | ❌ Wave 0 |
| LEDG-06 | No account holds negative cash, stock or headcount | unit + **negative** unit (conserved total, one account negative) | `cargo test --release --lib invariants::negative` | ❌ Wave 0 |
| LEDG-07 | Every units-bearing posting moves equal cash the other way | unit + **negative** unit | `cargo test --release --lib invariants::negative` | ❌ Wave 0 |
| LEDG-08 | A tick with zero transactions fails when gated on and passes when gated off | unit (both directions) | `cargo test --release --lib invariants::liveness` | ❌ Wave 0 |
| LEDG-08 | The config gate is read exactly once, at construction | source guard: `liveness_enabled` appears in exactly one file, `src/invariants.rs` | `bash tests/lints.sh` | ❌ Wave 0 |
| LEDG-09 | The reported posting is the FIRST non-conserving one, including when a later posting heals the residual | unit (the cancelling-residual case, explicitly) | `cargo test --release --lib invariants::localise` | ❌ Wave 0 |
| LEDG-09 | The `Display` of every `Violation` variant names tick, agent and posting | unit over all variants | `cargo test --release --lib invariants::message` | ❌ Wave 0 |
| LEDG-10 | No `debug_assert` / `cfg(debug_assertions)` in `books.rs` or `invariants.rs` | source guard (grep) | `bash tests/lints.sh` | ❌ Wave 0 |
| LEDG-10 | The invariant phase returns `Result` and a tick loop **aborts** on the seeded violation, at the right tick, and does not continue | **integration** | `cargo test --release --test invariant_halt` | ❌ Wave 0 |
| LEDG-10 | All four negative tests pass **in the release profile** | profile coverage | `cargo test --locked --release --all-targets` | ✅ CI step exists; new tests inherit it |
| — (order contract) | `ALL_CHECKS` is in the exact documented `CheckId` order | unit | `cargo test --release --lib invariants::order` | ❌ Wave 0 |
| — (config) | The new key is required, annotated, and has a provenance row | **already-existing** tests, re-run | `cargo test --test config_strict --test provenance` | ✅ — but they will FAIL until all four config files are updated |

### Sampling Rate

- **Per task commit:** `cargo test --locked --lib books invariants` (sub-second; the unit and negative tests)
- **Per wave merge:** `cargo test --locked --all-targets && cargo test --locked --release --all-targets`
- **Phase gate:** the full suite command above, all six steps green — matching CI exactly, before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `src/books.rs` — the module itself, with its `#[cfg(test)] mod` corruption vocabulary (LEDG-01/02/03/07)
- [ ] `src/invariants.rs` — `Violation`, `CheckId`, `ALL_CHECKS`, `CheckSet`, localisation (LEDG-04…10)
- [ ] `src/lib.rs` — `pub mod books;` and `pub mod invariants;`, keeping the flat alphabetical order
- [ ] `tests/ledger_props.rs` — proptest strategies for a valid `Books` and a random posting sequence
- [ ] `tests/ledger_atomicity.rs` — the `catch_unwind` LEDG-02 test
- [ ] `tests/invariant_halt.rs` — the library-level tick loop that must abort
- [ ] `tests/lint-probes/books_borrow_probe.rs.txt` — the `E0502` compile-fail probe
- [ ] `tests/lints.sh` — extended with the borrow probe and four grep guards, each asserting its search set is non-empty
- [ ] `clippy.toml` — interior-mutability `disallowed-types` entries
- [ ] `config/baseline.toml` + `src/config.rs` + `config/PROVENANCE.md` — the `[invariants]` table (all three, or `the_schema_and_the_shipped_config_name_the_same_leaves` fails)
- [ ] Framework install: **none needed** — `proptest` 1.11.0 and `thiserror` 2.0.20 are already locked

## Security Domain

`security_enforcement: true`, `security_asvs_level: 1` `[VERIFIED: .planning/config.json]`. This phase is an in-process, single-threaded computation over a local TOML file. There is no network, no user session, no persistence, no untrusted input beyond the config file — which Phase 1 already validates.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No principal, no credential, no session in this system |
| V3 Session Management | no | Same |
| V4 Access Control | no (in the security sense) | The *language-level* access control (private fields, `#[cfg(test)]`) is the LEDG-01/LEDG-02 mechanism, not a security control |
| V5 Input Validation | **yes** | The one new input is `invariants.liveness_enabled`. Controlled by `serde` + `deny_unknown_fields` + no defaults, already the project's pattern `[VERIFIED: src/config.rs:102-110]`. A `bool` has no out-of-domain value, so `Params::validate` needs no new arm |
| V6 Cryptography | no | No new cryptography. `sha2` is used only for the config hash, unchanged |
| V7 Error Handling & Logging | **yes** | `Violation`'s `Display` is written to stderr on halt. It contains only tick numbers, agent IDs, cents and units — **no path, no hostname, no wall-clock, no PID**. This is a determinism requirement (TICK-06) that happens also to be the right information-disclosure posture. Guard: assert no `Violation` variant interpolates a `Path`, `env::` value or `Instant` |
| V12 File Handling | no | This phase opens no file. `--out` handling is unchanged in `main.rs` |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Integer overflow / wraparound in ledger arithmetic | Tampering | `Money`'s checked operators (panic in every profile) plus `overflow-checks = true`. Already delivered and tested `[VERIFIED: src/money.rs; Cargo.toml [profile.release]]` |
| Panic mid-mutation leaving inconsistent state | Tampering | Compute-then-commit (Pattern 2 leg 4). Reproduced this session as `-400` against an opening `100` in the naive design |
| Test-only mutation surface reachable in production | Elevation of Privilege | `#[cfg(test)]`, verified unreachable from `tests/` (`E0599`) and therefore from any consumer. No cargo feature is introduced |
| Denial of service via unbounded journal growth | Availability | Per-tick buffer with `Vec::clear()` in `end_of_tick`, never `Vec::new()` (capacity reuse, bounded by one tick's posting count) |
| Information disclosure in a halt message | Information Disclosure | See V7 above — and it is already a determinism requirement, so the guard serves both |
| Unsafe code introducing aliasing | Tampering | `#![forbid(unsafe_code)]`, crate-wide `[VERIFIED: src/lib.rs:7]` |

No ASVS L1 control is unmet and no threat above requires a new mitigation beyond what the phase already plans.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Each agent owns its cash; transfers mutate two agents | One `Books` owns everything; agents own no value | Settled in this project's research pass | Dissolves the two-mutable-borrows problem rather than working around it with `Rc<RefCell<…>>` |
| `debug_assert!` for internal invariants | Invariants as a pipeline phase returning `Result` | Settled in this project's research pass | Verified this session that `debug_assert!`'s body is not evaluated in a release build with `overflow-checks = true` — the two Cargo flags are independent |
| Bisect a journal to find a violation | Carry the running residual on each posting; linear-scan for the first non-zero | This research | Measured: bisection gives the wrong answer on cancelling residuals |

**Deprecated / outdated for this phase:**
- `rand` 0.9-era API in any example (`Rng`→`RngExt`, `gen`→`random`) — irrelevant here since the invariant phase consumes no RNG, but worth stating so no example is copied in.
- A ring buffer of the last N transfers (mentioned in the project research SUMMARY as the Phase 2 deliverable). Superseded by the per-tick `Vec`, which is strictly stronger: it cannot lose the offending posting, and it is bounded anyway.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | ~274 postings per tick (10⁶ over a decade ÷ 3,650) is the right order of magnitude for the journal | Cost Analysis, Pitfall 2 | Low. Even 100× that is sub-second per decade. The estimate comes from the CONTEXT's own "~10⁶ postings"; it is arithmetic on an estimate, not a measurement, and no economics exist yet to measure |
| A2 | `[invariants]` as a new config table, with `liveness_enabled: bool`, is the shape the project wants | Configuration Surface | Low-medium. If the reviewer prefers the key under `[sim]`, the change is mechanical (one struct field moves). Worth a moment's confirmation before the config-touching task runs, because it also fixes where Phase 3's journal-to-disk flag lands |
| A3 | `headcount` should live in `Books` now (Pitfall 7 option b) rather than being an inherited Phase 6 obligation | Pitfall 7 | Medium. This is a genuine design fork the CONTEXT does not settle. Option (b) is recommended but option (a) is defensible. **Flag for the planner to decide explicitly rather than letting it be decided by whoever writes the check first** |
| A4 | Phase 3, not Phase 2, owns the process-level halt demonstration (non-zero exit + stderr) | Negative Test Design | Medium. If the reviewer reads ROADMAP criterion 2 as requiring a *process* halt within Phase 2, the phase needs a minimal run loop it would otherwise not build — duplicating Phase 3's `const PHASES`. Recommend confirming the split and recording it as a cross-phase constraint, as the liveness gate already was |
| A5 | Adding `serde::Serialize` to `Posting` now (for Phase 3's `events.jsonl`) is worth doing pre-emptively | Standard Stack | Low. If wrong, it is one derive to delete. If right, it avoids a wire-shape change after Phase 3 snapshots exist |
| A6 | Zero-sum (LEDG-07) is meaningfully checkable in Phase 2, where no economic "sale" exists | Pattern 4 | Low-medium. The check as designed is structural (cash leg and units leg name the same account pair in opposite directions), which is well-defined without economics — but it will have no *real* posting to test against until Phase 7. Its negative test uses a synthesised posting |

## Open Questions

1. **Does ROADMAP criterion 2 require a process-level halt inside Phase 2?**
   - What we know: the criterion says "halts the run and prints the tick, the agent and the offending posting". Phase 2 has no run loop; `main.rs` is a tracer and Phase 3 owns `const PHASES`.
   - What's unclear: whether "the run" means the process or the tick loop.
   - Recommendation: prove it at the library level in Phase 2 (a `for tick` loop that aborts), and record an explicit cross-phase constraint that Phase 3's binary-level negative test uses `liveness_enabled = true` against the empty economy — which needs no fault injection and is free once the pipeline exists. Surface this at the plan checkpoint.

2. **Where does `headcount` live?**
   - What we know: LEDG-06 names it; no employment relation exists until Phase 6.
   - What's unclear: whether the books own it (making the phrase "every quantity that must conserve" literal) or Phase 6 introduces it.
   - Recommendation: the books own it, as `Vec<u32>` keyed by `FirmSlot`. Note that `u32` makes the non-negativity of that column a type-level fact; document it rather than writing an unreachable runtime check. See Assumption A3.

3. **Does the initial endowment go through `transfer`, or is it a distinct `Endow` posting?**
   - What we know: money must enter the books somewhere, and `Money::from_cents` is documented as being for "config parsing and initial endowment only" `[VERIFIED: src/money.rs:63-67]`.
   - What's unclear: whether endowment postings appear in tick 0's journal (and therefore in the liveness count, which would make a "liveness passes because we endowed" false positive possible).
   - Recommendation: `PostingKind::Endow` recorded in `Books::new`'s own journal, **and `end_of_tick` semantics such that construction does not count as tick 0's transactions**. Otherwise the liveness check could pass on the strength of the endowment alone — exactly the degenerate pass LEDG-08 exists to close. Make this explicit; it is the subtlest correctness trap in the phase.

4. **Should `firm_cash_total()` be cached for OWN-07?**
   - What we know: the CONTEXT asks that `firm_cash / total_money` be cheap. Measured: a full 220-account sum is 175 ns.
   - Recommendation: no cache. Expose `firm_cash_total()` and `total_money()` as O(n) recomputes. A cache is a second source of truth that can drift, and the drift would be invisible precisely because the ratio is a *diagnostic* rather than a checked invariant.

## Sources

### Primary (HIGH confidence)

- **First-hand compile-and-run experiments on `rustc 1.94.1` (this session)** — every measurement and every behavioural claim marked "verified this session":
  - `debug_assert!` body not evaluated under `[profile.release] overflow-checks = true`; `cfg!(debug_assertions) == false` in that profile
  - Money conservation recompute: 175 ns/tick over 220 accounts, mutated per tick, `black_box`-guarded
  - Linear journal scan: 80 ns/tick over 274 postings
  - Bisection unsoundness on cancelling residuals: bisect → 200, linear → 50, correct answer 50
  - Non-atomic transfer observable via `catch_unwind` + `AssertUnwindSafe`: total −400 vs opening 100; atomic version 100
  - `&mut self` + callback leaks a mid-transaction `&B`: hook observed 50 vs opening 100
  - `#[cfg(test)]` methods reachable from unit tests, `E0599` from `tests/`
  - Shared-borrow-across-mutation → `E0502`
  - `thiserror` 2.0.20 nested-`Display` interpolation in an `#[error("…")]` attribute
  - Check-set-at-construction pattern: gate on/off, and check independence
- **Local repository source, read this session** — `src/money.rs`, `src/ids.rs`, `src/lib.rs`, `src/main.rs`, `src/config.rs`, `tests/config_strict.rs`, `tests/provenance.rs`, `tests/numeric_det.rs`, `tests/lints.sh`, `tests/tracer_end_to_end.rs`, `clippy.toml`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `config/baseline.toml`, `.github/workflows/`
- **Project planning record, read this session** — `.planning/phases/02-books-journal-and-invariants/02-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/research/SUMMARY.md`, `.planning/config.json`, `./.claude/CLAUDE.md`

### Secondary (MEDIUM confidence)

- None. No web search or documentation lookup was performed — the phase's questions are all answerable against the pinned toolchain and the repository itself, and a first-hand experiment is stronger evidence than any document. `docs.rs` and `doc.rust-lang.org` are egress-blocked in this environment, as the task brief noted.

### Tertiary (LOW confidence)

- The ~274-postings-per-tick figure (Assumption A1) is arithmetic on the CONTEXT's own estimate, not a measurement. Flagged, not relied upon for any decision beyond "the cost is negligible", which holds at 100× the figure.

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — no new dependency; both crates confirmed in the committed `Cargo.lock`
- Architecture: **HIGH** — every pattern compiled and executed on the pinned toolchain this session; the two designs that *fail* were also executed, so the recommendations are backed by observed counterexamples rather than by reasoning
- Pitfalls: **HIGH** for the five reproduced first-hand (1, 2, 3, 5, 8) and the four read directly out of existing test source (3, 4, 8, and the config-surface constraints); **MEDIUM** for 6, 7, 10 and 11, which are forward-looking design hazards rather than observed failures
- Cost analysis: **HIGH** — measured, on this machine, in the release profile the sim actually uses
- Configuration surface: **MEDIUM-HIGH** — the mechanics are verified against the four existing tests that police them; the *shape* (`[invariants]` table) is a recommendation (Assumption A2)

**Research date:** 2026-08-31
**Valid until:** 2026-09-30 for the ecosystem claims (stable, and no dependency changes). The repository-source claims are valid until the cited files change — every one carries a path so the planner can re-check cheaply. The toolchain claims are valid for as long as `rust-toolchain.toml` names 1.94.1, which is load-bearing for determinism and should not move within this milestone.
