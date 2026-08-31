# Phase 3: World, Tick Pipeline and Log Seam - Context

**Gathered:** 2026-08-31
**Status:** Ready for planning
**Mode:** Autonomous smart-discuss — classified **infrastructure phase**, no grey areas escalated

<domain>
## Phase Boundary

The tick becomes real: a fixed `const PHASES` table executing the brief's nine steps in order,
each completing for all agents before the next begins. `Household` and `Firm` first exist.
A run writes a complete, diffable run directory — `ticks.csv`, `events.jsonl`,
`run_meta.json`, and a generated, committed `schema/schema.json`.

Delivers TICK-01 … TICK-10 plus two obligations inherited from Phase 2.

**Contains no economic behaviour.** Nothing decides a price, a wage or a hire. The gate is
3,650 **empty** ticks that execute end to end, pass invariants, and diff byte-identically
between two runs at the same seed. That gate is deliberately unusual: it proves the machinery
before there is any economics to flatter it.

</domain>

<decisions>
## Implementation Decisions

Classified infrastructure by the smart-discuss test: every success criterion is technical
(a test asserts a name sequence, two runs diff byte-identically, a binary exits non-zero),
and no user-facing behaviour is described. No grey area was escalated for a user decision.

### Locked by prior decisions — not reopened here

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

### The wire-shape stake this phase carries

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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`books`** — owns every cent and unit; `Posting` is already `Serialize` with address fields
  rendered through `serialize_with`. `Books::accounts()` enumerates live addresses.
- **`invariants`** — `ALL_CHECKS` (5), `CheckSet`, `Violation` with `Display`. The tick
  pipeline calls the check phase as a real step returning `Result`.
- **`ids`** — `Account`, `FirmId {slot, gen}`, `FirmArena` with `respawn_in_place`, `Display`
  for every address.
- **`rng`** — sub-streams keyed `(seed, tick, agent_id, purpose)`; fixed-draw sampling only.
  Criterion 3's activation-order shuffle and per-tick draw-count column come from here.
- **`config`** — `deny_unknown_fields`, no serde defaults, `sha2` config hashing already used.
- **`money`** — checked arithmetic; `Money` is `Serialize` as an integer.

### Established Patterns

- Module-level `//!` docs carry requirement IDs (`TICK-…`) and decision IDs — the validation
  audit greps for them.
- Guards are adversarial and fixture-first: `tests/lints.sh` injects a hazard and proves the
  gate blocks it before trusting it silent. Guard 7j additionally pins a probe's call count to
  a declaration count so a probe cannot fall behind what it covers.
- Generated artifacts are committed and drift-tested against their generator (`clippy.toml`
  from std source; now `schema/schema.json` from the log types).
- **A config leaf is a FIVE-part agreement**: `config/baseline.toml` with its two-line
  `# GRADE:` block, the `src/config.rs` schema, `config/PROVENANCE.md`, the schema-leaf
  agreement, AND the hand-written `FULL` TOML fixture at `src/config.rs:503`. Phase 2 lost
  twelve lib tests by missing the fifth.

### Integration Points

- `src/lib.rs` — add `pub mod world;` and `pub mod log;`, keeping the flat alphabetical order.
- `src/main.rs` — currently a tracer; becomes the real CLI (`--config`, `--seed`, `--out`).
- Phase 4's Python harness reads the run directory across the disk boundary; the schema is the
  contract between them.

</code_context>

<specifics>
## Specific Ideas

- **The standing practice from Phase 2, recommended for this phase.** Five defects of one shape
  were found there — an assertion whose stated claim is not what it measures — and the
  242-test suite was green through every one. All five were caught by mutation: break the
  thing a check is supposed to catch, confirm the check fails, revert. Each proof cost one
  build. Criterion 3 here is exactly that discipline applied to reproducibility: a
  same-seed-identical test passes trivially if the RNG is never consumed.
- **CORRECTION — the mechanism this file first prescribed does not work, and neither does the
  ROADMAP criterion prescribing it.** `03-RESEARCH.md` built it exactly as written — an
  activation-order shuffle at 218 draws/tick plus a per-tick `rng_draws` column — ran 3,650
  ticks at seeds 42 and 43, and `cmp` returned **byte-identical**. The draw count is a
  *constant*: it proves draws happened and says nothing about which seed produced them. The
  counter-check meant to close the vacuous-reproducibility pass was itself vacuous. Measured
  fix, in both directions: add an `activation_digest` column — sha256 of the tick's
  permutation, `sha2` already being a dependency — which flips the same test to differing at
  tick 0. **The plan must amend ROADMAP criterion 3**, in the same inline-rationale shape plan
  02-01 used for the localisation clause.
- **Empty artifacts pass every test.** The empty pipeline writes `provenance.csv` and
  `events.jsonl` at 0 bytes; a cross-process hash comparison over two empty files compares the
  sha256 of the empty string with itself and passes vacuously, and pandas raises
  `EmptyDataError` on the former. Needs an eager CSV header — with `has_headers(false)`, since
  the obvious spelling emits the header twice — and the opening endowment emitted as events
  read from `books.accounts()`, which is also the origin row Phase 4's conservation replay
  requires (220 rows summing to exactly 2,000,000 cents).
- **Criterion 4 is two separate drift tests**, not one: the `PHASES` name sequence must fail on
  reorder, and the generated schema must fail on drift against the committed one.
- **Criterion 6 needs the binary, not the library.** Phase 2 proved the halt at library level
  and explicitly recorded that the process-level half lands here, so it cannot fall between
  the two phases.

</specifics>

<deferred>
## Deferred Ideas

- Any economic behaviour. Production arrives in Phase 5; nothing here decides a price, wage or
  hire.
- The Python harness itself — Phase 4 — though the schema it consumes is frozen here.
- Flipping `invariants.liveness_enabled` to `true` in the shipped config: Phase 6 criterion 7.

</deferred>
