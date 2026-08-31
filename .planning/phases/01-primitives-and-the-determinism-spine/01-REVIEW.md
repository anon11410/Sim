---
phase: 01-primitives-and-the-determinism-spine
reviewed: 2026-08-31T00:00:00Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - src/lib.rs
  - src/main.rs
  - src/money.rs
  - src/rng.rs
  - src/ids.rs
  - src/numeric.rs
  - src/config.rs
  - tests/tracer_end_to_end.rs
  - tests/money_props.rs
  - tests/determinism_rng.rs
  - tests/ids_generational.rs
  - tests/numeric_det.rs
  - tests/config_strict.rs
  - tests/provenance.rs
  - tests/toolchain.sh
  - tests/lints.sh
  - tests/lint-probes/float_ban_probe.rs.txt
  - tests/lint-probes/hazard.rs.txt
  - clippy.toml
  - Cargo.toml
  - rust-toolchain.toml
  - config/baseline.toml
  - config/PROVENANCE.md
  - .github/workflows/ci.yml
  - .gitignore
  - .proptest-regressions/money_props.txt
findings:
  critical: 3
  warning: 13
  info: 6
  total: 22
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-08-31
**Depth:** standard
**Files Reviewed:** 26
**Status:** issues_found

## Summary

The determinism spine is unusually well built: the RNG sub-stream facade is bijectively keyed
with a real (not debug-only) field-width assert, `pack_stream_key`'s layout arithmetic is correct
at the boundary, `Money`'s operator/`Result` split does what its doc claims, and the clippy ban
lists were verified by execution to fire on both widths and on the clock methods. The lint guard
scripts are genuinely adversarial — they inject hazards rather than assert configuration.

Three defects are nevertheless provable, and all three are in the "silent wrong number" class the
project exists to prevent:

1. `Money::split` — the one conservation-critical function — **panics on a valid input**
   (`i64::MAX` split 1 way), confirmed by execution. The remainder bump is computed before the
   code knows whether any recipient is bumped.
2. `config::load` performs **no semantic validation at all**. Confirmed by execution: a config
   with `households = 0`, `ticks = 99999999` (past the 24-bit RNG key field), a **negative** money
   stock and `initial_expected_demand = nan` loads cleanly and the binary prints a tracer line.
   The module's own doc comment states `CAL-01 requires it strictly positive`; nothing enforces it.
3. `FirmArena::live_ids` truncates the slot index with `index as u16`, so the identity type whose
   entire purpose is "identities never silently alias" can silently alias.

The remaining findings are concentrated on the guards. Several cannot bite: the completeness proof
in `tests/lints.sh` check 3 covers the 58 float bans but not the 2 clock bans; the serde-default
grep misses `#[serde(rename = "x", default)]`; `ALL_PURPOSES` completeness is compared only against
itself; the rayon ban is a line-anchored grep of `Cargo.toml` while the getrandom ban next to it
correctly uses `cargo tree`; and the `Money::split` property strategies exclude exactly the region
(n = 1, zero, negatives) where finding 1 lives.

No structural pre-pass (`<structural_findings>`) was supplied with this review, so every finding
below is narrative.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `Money::split` panics on a valid amount because the remainder bump is computed unconditionally

**File:** `src/money.rs:150-152`
**Severity:** BLOCKER

`bumped` is computed before the code checks whether `extra_recipients > 0`. When
`remainder == 0` and `base == i64::MAX`, the eager `checked_add(1)` fails and the function aborts
on an amount it should have returned untouched.

Verified by execution against the release profile:

```
Money::from_cents(i64::MAX).split(1)
  -> panicked at 'Money overflow on split remainder distribution'
```

The correct answer is `vec![Money(i64::MAX)]`. There is no overflow: the remainder is zero, so no
recipient is bumped, so the `+1` is dead arithmetic on that path. This is the single function the
brief names as the conservation guard, and it aborts a run rather than conserving. The unit tests
(`split_tests`) and the proptest strategies both stay inside `1..1_000_000`, so nothing catches it
(see WR-07).

**Fix:** only compute the bump when someone is actually bumped.

```rust
let extra_recipients = remainder.unsigned_abs();
let extra_cent: i64 = if remainder < 0 { -1 } else { 1 };

let mut parts = Vec::with_capacity(n as usize);
for index in 0..u64::from(n) {
    parts.push(Money(if index < extra_recipients {
        base.checked_add(extra_cent)
            .expect("Money overflow on split remainder distribution")
    } else {
        base
    }));
}
parts
```

Add `(i64::MAX, 1)`, `(i64::MIN, 1)`, `(0, 1)` and a negative-amount arm to `split_tests`.

---

### CR-02: `config::load` validates nothing beyond TOML shape — out-of-domain parameters are accepted silently

**File:** `src/config.rs:187-219`
**Severity:** BLOCKER

`load` does exactly three things: read bytes, hash them, parse them — plus one money-headroom
check (`stock.checked_add(stock)`). Every other parameter is accepted as written. The struct
doc comments state domain requirements that no code enforces.

Verified by execution. A `baseline.toml` mutated to
`households = 0`, `ticks = 99999999`, `total_money_cents = -2000000`,
`initial_expected_demand = nan` produced:

```
tracer effective_seed=42 config_sha256=28fbca7a… draw=776863 money_cents=-2000000
```

No error, no warning. Concrete consequences, each with the failing input:

| Input | Consequence |
|---|---|
| `sim.ticks = 99999999` (> 2²⁴−1 = 16 777 215) | `pack_stream_key`'s tick assert fires **at tick 16 777 216**, after hours of simulated economy. A run-start rejection is the only useful place to fail. |
| `sim.households = 20000000` (> 2²⁴−1) | same, on the agent field. |
| `sim.firms = 70000` (> `u16::MAX`) | `FirmSlot` is a `u16`; see CR-03 — slots alias silently rather than panicking. |
| `money.total_money_cents = -2000000` | a negative money pile passes `checked_add(self)` and the tracer prints `money_cents=-2000000`. The brief's core invariant is "money is a fixed pile"; a negative pile is not one. |
| `firm.initial_expected_demand = nan` (TOML 1.0 accepts the `nan` and `inf` literals) | flows to `pow_frac_det`, whose positivity check is `debug_assert!` only. Confirmed: `pow_frac(-1.0, 0.9)` returns `NaN` in release with no panic, and `demand_to_units(NaN)` returns `0`. A whole firm's demand expectation becomes zero units with no diagnostic. |
| `firm.initial_expected_demand = 0.0` or `-1.0` | same path; the field's own comment says "CAL-01 requires it strictly positive". |
| `household.consumption_exponent_ppm = 0` or `1000000` | `pow_frac_det` documents its domain as `0 < alpha < 1`; outside it the release build returns `1.0` or a truncated garbage value rather than failing. |
| `sim.month_days = 0`, `sim.households = 0`, `sim.firms = 0` | zero divisors and an empty economy accepted. |
| `household.supplier_list_size = 999` with `sim.firms = 20` | `Stream::sample_k` asserts `k <= pool.len()` — a panic on tick 1 rather than a config rejection. |

**Fix:** add a `Params::validate(&self) -> Result<(), ConfigError>` called from `load` after the
parse, with a `ConfigError::Domain { path, key, why }` variant. Minimum set, expressed against the
constants that already exist:

```rust
use crate::rng::{AGENT_BITS, TICK_BITS};
use crate::numeric::PPM_SCALE;

fn validate(&self) -> Result<(), &'static str> {
    if self.sim.ticks == 0 || u64::from(self.sim.ticks) > (1u64 << TICK_BITS) - 1 {
        return Err("sim.ticks must be in 1..=16_777_215 (the sub-stream key's tick field)");
    }
    let agents = self.sim.households.max(self.sim.firms);
    if u64::from(agents) > (1u64 << AGENT_BITS) - 1 {
        return Err("sim.households/firms exceed the sub-stream key's agent field");
    }
    if self.sim.firms == 0 || u64::from(self.sim.firms) > u16::MAX as u64 {
        return Err("sim.firms must be in 1..=65_535 (FirmSlot is a u16)");
    }
    if self.sim.households == 0 { return Err("sim.households must be non-zero"); }
    if self.sim.month_days == 0 { return Err("sim.month_days must be non-zero"); }
    if self.money.total_money_cents <= 0 {
        return Err("money.total_money_cents must be strictly positive");
    }
    // The one float field: finite AND strictly positive (CAL-01).
    if !(self.firm.initial_expected_demand > 0.0)
        || !self.firm.initial_expected_demand.is_finite()
    {
        return Err("firm.initial_expected_demand must be finite and strictly positive (CAL-01)");
    }
    let a = i64::from(self.household.consumption_exponent_ppm);
    if a == 0 || a >= PPM_SCALE {
        return Err("household.consumption_exponent_ppm must be in 1..999_999 (pow_frac domain)");
    }
    // Every *_prob_ppm must be <= PPM_SCALE — see WR-05.
    Ok(())
}
```

Note `is_finite` and `>` are comparisons, not banned methods, so this stays inside D-11's rules —
but the check must live in `src/config.rs`, the only other file `tests/numeric_det.rs` allowlists,
and each such line must contain `expected_demand` to satisfy that test's narrower per-line rule.

---

### CR-03: `FirmArena::live_ids` truncates the slot index with `as u16`, silently aliasing identities

**File:** `src/ids.rs:178-187` (cast at :183); enabling gap at `src/ids.rs:103-113`
**Severity:** BLOCKER

```rust
.map(|(index, record)| FirmId {
    slot: FirmSlot(index as u16),   // <- lossy
    generation: record.generation,
})
```

`with_occupants` accepts a `Vec<T>` of any length and never bounds it against `u16::MAX`. For an
arena of 65 537 slots, `live_ids()` returns `FirmSlot(0)` for both index 0 and index 65 536 — two
distinct firms carrying the same `FirmId`. Every slot at index ≥ 65 536 is also unreachable through
`get`/`get_mut`/`id_at`, which index with `id.slot.0 as usize`.

This is the exact failure mode the module's header calls "the hardest class of defect to find in an
emergent system", produced by the type that exists to prevent it. `clippy::cast_possible_truncation`
lives in `pedantic` and is not enabled, so nothing in the tree catches it. It is reachable today
only via an unvalidated `sim.firms` (CR-02), but the arena is public API and the cast is wrong
independently of how it is called.

**Fix:** close it at construction, where the error is attributable, and remove the lossy cast.

```rust
pub fn with_occupants(occupants: Vec<T>) -> Self {
    assert!(
        occupants.len() <= u16::MAX as usize,
        "FirmArena holds at most {} slots; FirmSlot is a u16 and a wider index would \
         silently alias two firms onto one identity",
        u16::MAX
    );
    // ...
}

pub fn live_ids(&self) -> Vec<FirmId> {
    self.slots
        .iter()
        .enumerate()
        .map(|(index, record)| FirmId {
            slot: FirmSlot(u16::try_from(index).expect("arena length is bounded at construction")),
            generation: record.generation,
        })
        .collect()
}
```

---

## Warnings

### WR-01: The two clock bans in `clippy.toml` are the only entries no guard exercises

**Files:** `clippy.toml:45-46`, `tests/lints.sh:123-170`, `tests/lint-probes/float_ban_probe.rs.txt`

`clippy.toml`'s own header states the problem precisely: "Clippy SILENTLY IGNORES a
disallowed-methods path it cannot resolve … That same silence is why `tests/lints.sh` compares the
probe's diagnostic count against its call-site count." That comparison covers 58 float call sites.
It covers zero clock call sites — `grep -n 'SystemTime\|Instant'` across `tests/`, `src/` and both
probes returns nothing.

So `std::time::SystemTime::now` and `std::time::Instant::now` are the two entries in the file whose
resolution is asserted by nothing. I verified by hand that they currently do fire (injecting both
calls produced exactly 2 `use of a disallowed method` errors), so this is a coverage hole rather
than a live break — but the wall-clock ban is one of the top determinism hazards in `CLAUDE.md`,
and it is the one entry a toolchain move or a path typo could silently disable.

**Fix:** add the two calls to the probe with the same marker, so `MARKED` rises to 60 and the
equality check covers them:

```rust
// ---- the clock, banned for the same reason as the floats ----
pub fn probe_systemtime_now() -> std::time::SystemTime { std::time::SystemTime::now() } // BANNEDCALL
pub fn probe_instant_now() -> std::time::Instant { std::time::Instant::now() } // BANNEDCALL
```

---

### WR-02: The serde-default grep is defeated by attribute ordering, and the exhaustive test only covers keys already in the shipped file

**File:** `tests/config_strict.rs:271-276` (grep), `tests/config_strict.rs:121-155` (exhaustive test)

```rust
assert!(!text.contains("serde(default"), ...);
```

This matches `#[serde(default)]` and `#[serde(default = "…")]` and nothing else. All of the
following are real serde defaults that pass the check:

- `#[serde(rename = "x", default)]`
- `#[serde(skip_serializing_if = "…", default)]`
- any multi-line attribute, because `contains` is over the whole file text and the substring
  spans a newline

The file's own comment concedes the grep is "necessary but not sufficient" and points at
`every_key_is_required` as the real proof. But `every_key_is_required` enumerates leaf paths of
`config/baseline.toml` and deletes each in turn — so it can only see fields that are already in
the shipped file. A field added to `Params` with a default and *not* added to `baseline.toml`
satisfies both checks and is exactly the hidden hardcoded parameter CORE-10 forbids.

**Fix:** (a) make the grep attribute-order agnostic and line-based; (b) close the second half by
asserting the schema and the shipped file have the same leaf set, not just that the shipped leaves
are required:

```rust
// (a)
for (n, line) in text.lines().enumerate() {
    let stripped: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        !(stripped.contains("serde(") && (stripped.contains("(default") || stripped.contains(",default"))),
        "{}:{} carries a serde default", path.display(), n + 1
    );
}

// (b) round-trip: serialising a parsed Params must reproduce the shipped leaf set exactly,
// so a schema field with no config key fails.
let params: Params = toml::from_str(&raw).unwrap();
let round_tripped: toml::Value = toml::from_str(&toml::to_string(&params).unwrap()).unwrap();
let mut schema_paths = Vec::new();
leaf_paths(&round_tripped, &[], &mut schema_paths);
schema_paths.sort();
let mut shipped_paths = paths.clone();
shipped_paths.sort();
assert_eq!(schema_paths, shipped_paths, "the schema and the shipped config disagree on leaf keys");
```

---

### WR-03: `ALL_PURPOSES` completeness is enforced by a hand-written array length and tested only against itself

**File:** `src/rng.rs:94-107`, and the tests at `src/rng.rs:310-326`, `src/rng.rs:410-419`,
`tests/determinism_rng.rs:84-105`

`ALL_PURPOSES: [Purpose; 12]` is written by hand. Every test that touches it compares it to
itself:

- `pack_stream_key_is_injective_over_a_swept_grid` asserts `swept == 40 * 40 * ALL_PURPOSES.len()`
- `every_purpose_discriminant_is_distinct_and_non_zero` asserts `seen.len() == ALL_PURPOSES.len()`
- `distinct_keys_give_distinct_streams` sweeps `ALL_PURPOSES`

Appending `Purpose::SomethingNew = 51` to the enum without adding it to the array compiles, passes
every test, and the new discriminant is never checked for collision or for injectivity. The array's
own doc comment ("the injectivity sweep is only as complete as this array") names the risk but
nothing makes it structural — and the enum is explicitly append-only, so this *will* be exercised
by later phases.

**Fix:** force a compile error on a new variant with an exhaustive match. Adding a variant then
fails to compile until it is added here.

```rust
/// Compile-time completeness guard for `ALL_PURPOSES`. A new `Purpose` variant
/// makes this match non-exhaustive, which is a compile error, not a silent gap.
const fn purpose_is_listed(p: Purpose) -> bool {
    match p {
        Purpose::TracerProbe
        | Purpose::ActivationOrderHouseholds
        | Purpose::ActivationOrderFirms
        | Purpose::LabourSample
        | Purpose::EmployedSearchCoin
        | Purpose::GoodsSample
        | Purpose::SupplierRevision
        | Purpose::PriceInactionCoin
        | Purpose::PriceStep
        | Purpose::WageStep
        | Purpose::PlanningOffsetInit
        | Purpose::BankruptcyOwnerDraw => true,
    }
}

const _: () = {
    let mut i = 0;
    while i < ALL_PURPOSES.len() {
        assert!(purpose_is_listed(ALL_PURPOSES[i]));
        i += 1;
    }
};
```

---

### WR-04: `tests/lints.sh` check 4 guards `disallowed_types` but not `disallowed_methods`, and its alias regex misses `pub(crate)`

**File:** `tests/lints.sh:179-192`

Two asymmetries in the escape-hatch check:

**4b (line 191-192)** searches only for `clippy::disallowed_types` exemptions:

```bash
-En '#!?\[(allow|expect)\(clippy::disallowed_types' -- "${RUST_SOURCES[@]}"
```

`#![allow(clippy::disallowed_methods)]` at the top of a module disables the entire 66-entry float
ban and the clock ban, and this check does not look for it. Check 3 proves the ban list *resolves*;
nothing proves no file opts out of it. `#[allow(warnings)]` also evades both patterns.

**4a (line 179-180)** anchors the alias regex on `^[[:space:]]*(pub[[:space:]]+)?type`. A
`pub(crate) type Index = std::collections::HashMap<u32, u32>;` does not match — `pub(crate)` is not
`pub` followed by whitespace — so the visibility form most likely to be used inside a single crate
is precisely the one the guard misses.

**Fix:**

```bash
assert_absent "a file carries a lint exemption for a determinism ban" \
    -En '#!?\[(allow|expect)\((warnings|clippy::(all|disallowed_types|disallowed_methods))' \
    -- "${RUST_SOURCES[@]}"

assert_absent "a type alias to a hashed collection exists under src/" \
    -rEn '^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?type[[:space:]]+[A-Za-z0-9_]+.*=.*Hash(Map|Set)' src/
```

---

### WR-05: `Stream::below(0)` and out-of-range `coin_ppm` are debug-asserted only, so release returns a silently wrong answer

**File:** `src/rng.rs:222-234`

`below` guards its precondition with `debug_assert!(n > 0, "below(0) has no valid result")`. In
release that assert is compiled out and `((x as u128 * 0) >> 64) as u64` evaluates to `0` — a
plausible-looking index into an empty pool rather than a failure. The module makes exactly the
opposite choice one function earlier: `pack_stream_key` uses a real `assert!` with the rationale
"a silent field overrun would … corrupt a run without failing anything (T-1-12)". The same argument
applies here and is not applied.

`coin_ppm(p_ppm)` takes a `u32` with no upper bound. `coin_ppm(2_000_000)` always returns `true`
because `below(1_000_000)` can never reach it. This is not hypothetical: `baseline.toml` already
ships several ppm keys above 1 000 000 (`price_ceiling_over_mc_ppm = 1150000`,
`entrant_price_ratio_ppm = 1260000`). Wiring one of those into a coin in a later phase produces a
deterministic always-true with no diagnostic anywhere.

**Fix:** promote both to real asserts on the same grounds as `pack_stream_key`.

```rust
pub fn below(&mut self, n: u64) -> u64 {
    assert!(n > 0, "below(0) has no valid result");
    // ...
}

pub fn coin_ppm(&mut self, p_ppm: u32) -> bool {
    assert!(
        i64::from(p_ppm) <= crate::numeric::PPM_SCALE,
        "coin_ppm called with {p_ppm} ppm, which is above 1_000_000 and therefore always true"
    );
    self.below(1_000_000) < u64::from(p_ppm)
}
```

---

### WR-06: The float→integer crossing maps `NaN` to `0` in release, converting a broken computation into a plausible number

**File:** `src/numeric.rs:114-117`; upstream at `src/numeric.rs:68-72`

```rust
pub fn demand_to_units(x: f64) -> i64 {
    debug_assert!(x.is_finite(), "a non-finite value reached the crossing");
    x.round() as i64
}
```

Rust's saturating float→int cast maps `NaN` to `0`. Confirmed by execution in the release profile:
`demand_to_units(f64::NAN)` returns `0` with no panic, and `pow_frac(-1.0, 0.9)` returns `NaN` with
no panic (its domain guards are also `debug_assert!`).

So the chain "config supplies a bad demand → `pow_frac` returns NaN → crossing returns 0" runs end
to end in a release build without a single diagnostic, and a firm that produces nothing looks like a
firm that chose to produce nothing. The doc comment reasons carefully about saturation for large
magnitudes but does not address NaN, which is the case that produces a *plausible* wrong answer
rather than an obviously extreme one.

**Fix:** make the finiteness check unconditional at the crossing — it is one comparison per call and
this is the only crossing in the crate.

```rust
pub fn demand_to_units(x: f64) -> i64 {
    assert!(x.is_finite(), "a non-finite value ({x}) reached the float/integer crossing");
    x.round() as i64
}
```

Do the same for `pow_frac_det`'s two domain preconditions, or return `Result` and let the caller
decide. CR-02's config validation is the complementary fix; both are wanted.

---

### WR-07: The `Money::split` property strategies exclude n = 1, zero and negative amounts — the exact region where CR-01 lives

**File:** `tests/money_props.rs:34, 46-48, 58`

All three split properties use `amount in 1i64..1_000_000, n in 2u32..64`. The claimed invariant is
"the parts always sum exactly back to the whole, for all `(amount, n)`" (module header, and
`CLAUDE.md` §7). What is actually tested is a strictly positive amount below one million, split
between 2 and 63 ways.

Untested and reachable: `n == 1` (the single-recipient case — CR-01), `amount == 0`, negative
amounts (covered only by one hand-written unit test), and amounts anywhere near the `i64`
boundaries. This is why CR-01 shipped.

**Fix:** widen the strategies and keep a narrow arm for the remainder-specific property.

```rust
#[test]
fn split_parts_sum_to_the_whole(amount in i64::MIN / 2..i64::MAX / 2, n in 1u32..64) {
    let whole = Money::from_cents(amount);
    let parts = whole.split(n);
    prop_assert_eq!(parts.len(), n as usize);
    prop_assert_eq!(parts.into_iter().sum::<Money>(), whole);
}
```

Note `split_part_spread_is_at_most_one_cent` needs `(largest - smallest).abs() <= 1` once negatives
are admitted.

---

### WR-08: The rayon ban is a line-anchored grep of `Cargo.toml`; the getrandom ban two checks later correctly uses `cargo tree`

**File:** `tests/toolchain.sh:27-29`

```bash
if grep -Eq '^[[:space:]]*rayon[[:space:]]*=' Cargo.toml; then
```

This matches only the inline dependency form at the start of a line. It does not match:

- `[dependencies.rayon]` table form
- `rayon = { version = "1" }` written under `[dev-dependencies]` — arguably out of scope, but the
  check does not distinguish, so it is scope-by-accident
- **any transitive rayon**, which is the realistic way data parallelism enters a dependency graph

Check 4 in the same file already demonstrates the sound technique for exactly this
(`cargo tree --edges normal | grep getrandom`), and the comment there explains why the graph and
not the manifest is the right thing to search. The rayon check does not follow its own neighbour.

**Fix:**

```bash
# Reuse the graph, not the manifest: a transitive rayon is still threads.
if echo "$TREE" | grep -Eq '(^|[^a-z-])rayon( |$|v)'; then
    fail "a data-parallelism crate (rayon) is reachable from the behaviour path"
fi
```

(move the `TREE=$(cargo tree --edges normal)` assignment above this check).

---

### WR-09: A grade-B provenance row cites a source value that is not the shipped value, with no derivation

**File:** `config/baseline.toml:150-152`, `config/PROVENANCE.md:101`

```
entrant_size_ratio_ppm = 800000
# GRADE: B | SOURCE: BAM `size-replacing-firms` = 0.2, via annotated replication (UNVERIFIED)
```

The row states a source value of **0.2** and a shipped value of **0.8**. Either the shipped value
is a transcription error, or it is derived (`1 − 0.2`?) — in which case the row is grade **C**
(derived arithmetic) and the derivation belongs in the SOURCE field, exactly as
`incumbent_trim_per_tail` does it ("derived arithmetic — 5% of 20 firms = 1").

None of the six provenance tests can see this: test 5 checks only that a row *exists* for the key,
and test 6 checks only that grade-B rows stay `UNVERIFIED`. `V-3` flags the BAM rows as unread but
does not record this specific numeric mismatch, so the Phase 6 gate has nothing pointing at it.

**Fix:** either correct the value, or regrade the row to C and write the derivation into both the
config annotation and the PROVENANCE row; and add the mismatch to V-3 explicitly so the Phase 6
verifier is looking for it.

---

### WR-10: `confinement_of_the_float_domain` matches float *type names*, so untyped float arithmetic is invisible to it

**File:** `tests/numeric_det.rs:84-91, 115-127, 147-158`

The test asserts no file outside `["numeric.rs", "config.rs"]` names `f16`/`f32`/`f64`/`f128`.
Rust does not require naming the type:

```rust
// in any src/ file — passes confinement_of_the_float_domain today
let smoothing = 0.25;              // inferred f64
let expected = observed * smoothing + expected * (1.0 - smoothing);
```

No banned method is called, so clippy is silent; no type name appears, so the confinement test is
silent. The module header calls the pair "the lint catches the accidental call, this catches the
spread" — but the spread that matters most (float arithmetic creeping into a behaviour module) is
in neither's coverage.

Second, smaller issue: the allowlist is compared against `path.file_name()`, so a future
`src/market/numeric.rs` is allowlisted by coincidence of basename rather than by being the module
that owns the float domain.

**Fix:** add a literal check alongside the type-name check, and match on the path relative to
`src/` rather than the bare filename:

```rust
/// A float literal: a digit, a dot, a digit. Catches `0.25` and `1e-9`,
/// and does not fire on `1..10`, `x.0` or `Self::CONST`.
fn names_a_float_literal(line: &str) -> bool {
    let b = line.as_bytes();
    (1..b.len().saturating_sub(1)).any(|i| {
        b[i] == b'.' && b[i - 1].is_ascii_digit() && b[i + 1].is_ascii_digit()
    })
}
```

and compare `path.strip_prefix(&src)` against `["numeric.rs", "config.rs"]`.

---

### WR-11: The `sha2` pin comment cites a rationale the code deliberately does not rely on

**File:** `Cargo.toml:10`, contradicted by `src/config.rs:221-233`

```toml
sha2 = "0.10.9"          # NOT 0.11 -- 0.11's digest type has no LowerHex impl (D-25)
```

`config_hash` does not use `LowerHex`, and says so explicitly: "Hex is built byte by byte rather
than through a `LowerHex` impl on the digest type, so the idiom survives a future `sha2` major
bump." The manifest therefore blocks an upgrade for a reason the code went out of its way to
neutralise, and the next person to read the comment will believe a constraint that does not exist.

**Fix:** either delete the constraint clause and pin for the honest reason (the lockfile is the
reproducibility contract; major bumps get reviewed), or, if 0.11 is blocked by something real,
state that instead:

```toml
sha2 = "0.10.9"   # Pinned by the reproducibility contract, not by an API constraint:
                  # config_hash formats bytes itself (src/config.rs) and is major-version agnostic.
```

---

### WR-12: `raw_i64_overflow_panics_in_release` is vacuous under plain `cargo test`, and no guard asserts the profile setting itself

**File:** `tests/tracer_end_to_end.rs:136-142`; gap in `tests/toolchain.sh`

The test's own header says it "fails if anyone deletes the setting". Under `cargo test` it does
not: the `test` profile inherits `dev`, where `overflow-checks` is already on by default. Deleting
`overflow-checks = true` from `[profile.release]` leaves this test green in the debug run. It only
carries information under `cargo test --release` (where the `bench` profile inherits `release`).

CI does run that pass (`.github/workflows/ci.yml:52-53`), so this is not a live hole — but it means
the single most load-bearing line in `Cargo.toml` is protected only by a test whose name asserts a
profile it does not select, and `tests/toolchain.sh` — the script whose stated job is "the
reproducibility contract, as checkable facts" — does not check it at all, while checking four less
critical facts.

**Fix:** (a) rename to `raw_i64_overflow_panics_when_overflow_checks_are_on` and note in the comment
that it is informative only in the `--release` pass; (b) add the missing fact to `toolchain.sh`
next to the `target-cpu` check:

```bash
# 3b. The release profile cannot silently wrap. Verified: a default release build
#     wrapped `i64::MAX - 1 + 6` to a plausible negative balance.
if ! grep -Pzoq '(?s)\[profile\.release\].*?overflow-checks[[:space:]]*=[[:space:]]*true' Cargo.toml; then
    fail "[profile.release] does not set overflow-checks = true (CORE-02 / D-10)"
fi
```

---

### WR-13: `sample_k` and `shuffle_in_place` permute the caller's pool, so a shared pool reintroduces the cross-purpose coupling CORE-04 removes

**File:** `src/rng.rs:245-269`; untested by `tests/determinism_rng.rs:53-81`

Both samplers mutate `pool` in place. The doc notes this as a feature ("`pool` is permuted in place
and its first `k` entries are the returned sample") but does not warn about the consequence: if a
later phase allocates one `Vec<u32>` of firm indices at setup and reuses it across purposes and
ticks — the natural way to avoid a per-tick allocation — then the sample drawn for `GoodsSample` at
tick *t* depends on how many times `LabourSample` permuted the same buffer earlier. That is exactly
the "an added draw in one market perturbs another" failure the sub-stream design exists to
eliminate, reintroduced through shared mutable state rather than through a shared sequence.

`extra_draws_in_one_purpose_cannot_perturb_another` cannot detect this: it uses no pool at all, so
the isolation property it certifies is narrower than the property the module claims.

**Fix:** document the constraint at the API and prove it in the test that owns CORE-04.

```rust
/// # Pool aliasing
///
/// `pool` is permuted in place. A pool shared between two purposes makes the
/// second purpose's sample depend on the first's draw count, which defeats
/// sub-stream isolation from the outside. Build the pool fresh per draw site,
/// or restore its order before reuse.
```

and extend the isolation test with a pool arm:

```rust
// The same property, but with a pool: the goods sample must not depend on
// whether the labour market sampled from a pool built the same way.
let baseline = { /* fresh pool, goods sample only */ };
let after = { /* fresh pool, labour samples first, then goods from a FRESH pool */ };
assert_eq!(baseline, after);
```

---

## Info

### IN-01: Six tests are duplicated near-verbatim between `src/numeric.rs` and `tests/numeric_det.rs`

**Files:** `src/numeric.rs:137-227` and `tests/numeric_det.rs:21-78`

`pow_frac_returns_one_bit_pattern_across_many_calls` /
`pow_frac_is_bit_identical_across_many_invocations`,
`half_power_is_exactly_one_square_root` + `quarter_power_is_exactly_two_square_roots` /
`pow_frac_matches_repeated_square_roots_at_negative_powers_of_two`,
`twenty_bits_and_forty_bits_differ_somewhere_on_the_range` +
`forty_bits_and_full_precision_agree_to_one_part_in_a_billion` +
`pow_frac_uses_the_committed_bit_count` / `bit_count_is_load_bearing`, and
`the_crossing_rounds_half_away_from_zero` + `the_crossing_maps_zero_to_zero_and_saturates_out_of_range` /
`crossing_rounds_half_away_from_zero_and_saturates` are the same assertions, and `swept_inputs`
(20 000 elements) plus the 100 000-iteration loop run twice per `cargo test`.

**Fix:** keep the integration copies (they are the ones that prove the library surface is reachable,
which is CORE-08's point) and delete the `#[cfg(test)] mod tests` duplicates from `src/numeric.rs`,
or keep only the ones that exercise something not reachable publicly.

---

### IN-02: `rust_sources` is copy-pasted between two test files

**Files:** `tests/numeric_det.rs:95-111`, `tests/config_strict.rs:101-117`

Identical 17-line recursive directory walker in both. A fix to one (e.g. the symlink or the
non-UTF-8-path case) will not reach the other.

**Fix:** move it to a `tests/common/mod.rs` and `mod common;` from both.

---

### IN-03: The tracer test leaks temp directories

**File:** `tests/tracer_end_to_end.rs:20-24`

`out_dir` removes the directory *before* the run and never after, and the binary then creates it.
Five directories per `cargo test` accumulate under the system temp dir, each named with the test
process's PID so they never collide and never get reused.

**Fix:** return a guard type with a `Drop` that removes the directory, or use `tempfile` (already in
the dependency graph via `proptest` — though adding it as a direct dev-dependency is cleaner).

---

### IN-04: The lint script's injected files are not in `.gitignore`

**Files:** `.gitignore`, `tests/lints.sh:37-38`

`tests/_probe.rs` and `tests/_hazard.rs` are created and removed by the trap on `EXIT INT TERM`. A
`SIGKILL`, a full disk, or a container teardown leaves them in the working tree, where the next
`cargo test` compiles a deliberately-broken file and the next `git add -A` commits it.

**Fix:** add to `.gitignore`:

```
# Injected by tests/lints.sh; removed by its EXIT trap. Ignored so an
# ungraceful interrupt cannot leave a known-bad file staged.
/tests/_probe.rs
/tests/_hazard.rs
```

---

### IN-05: `every_key_is_required`'s floor is one below the actual key count

**File:** `tests/config_strict.rs:129-133`

`assert!(paths.len() >= 40, ...)` while `config/PROVENANCE.md:58` states, and the file contains,
**41** leaf keys. One key can be deleted from `baseline.toml` without this tripwire noticing — and
the deletion would not be caught elsewhere, since the loop only iterates keys that are present.

**Fix:** `assert_eq!(paths.len(), 41, ...)`, matching PROVENANCE.md's own count, so adding or
removing a parameter is a deliberate two-file edit.

---

### IN-06: `no_optional_fields_in_the_config_schema` bans the literal `Option<` anywhere in `src/config.rs`

**File:** `tests/config_strict.rs:279-292`

The check is a whole-file substring scan, not a scan of struct field declarations. It will
false-positive the moment `config.rs` uses `Option` for anything ordinary — an internal helper
returning `Option<&str>`, a `path.file_name()` chain, or the `Params::validate` suggested in CR-02
if it happens to use one. The failure message will then accuse the file of a schema defect it does
not have.

**Fix:** scope the scan to lines that look like field declarations inside the schema structs, e.g.
only lines matching `^\s*pub\s+\w+\s*:` — which is where an optional field would actually have to
appear to become a hidden default.

---

_Reviewed: 2026-08-31_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
