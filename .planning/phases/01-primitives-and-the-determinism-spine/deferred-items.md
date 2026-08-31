
## From plan 01-04 (2026-08-30)

- **`cargo fmt --check` fails on files this plan does not own.** `src/money.rs`
  (10 diffs, from plan 01-03) and `tests/tracer_end_to_end.rs` (4 diffs, from
  plan 01-01) are not rustfmt-clean. `src/rng.rs` and `tests/determinism_rng.rs`
  were brought to rustfmt-clean by this plan; the other two were deliberately
  left alone under the executor scope boundary (out-of-scope files are not
  auto-fixed). Whitespace only — no behaviour implication. Natural owner is plan
  **01-07**, which builds the clippy/CI wall: if `cargo fmt --check` joins CI it
  must be preceded by a one-shot repo-wide `cargo fmt`.

  **CLOSED by plan 01-07 (2026-08-31), commits `bc6f16d` + `6d10d1a`.** The
  one-shot repo-wide `cargo fmt` ran (whitespace and layout only: derive lists
  broken one-per-line, struct literals expanded; 112 debug / 110 release tests
  still pass, clippy still clean), and `cargo fmt --check` is now a CI step, so
  the item cannot silently re-open.

## From the code-review fix pass (2026-08-31)

- **V-3a — `entrant_size_ratio_ppm` contradicts its own cited source.** The key
  ships `800000` (0.8x) while its `SOURCE:` field cites
  `size-replacing-firms = 0.2`. Three readings are possible and they are not
  equivalent: a transcription error; a deliberate derivation of `1 - 0.2`
  (which would regrade the row B to C, since a derived value is not a read
  value); or a misread source parameter. Deliberately NOT resolved from model
  memory, per D-20. Recorded as open item **V-3a** in `config/PROVENANCE.md`
  with the action for each reading.

  **OPEN — needs a human decision. GATED as of 2026-08-31, but not answered.**
  Raised as warning W1 of `01-VERIFICATION.md`, which is also what caught this
  file previously claiming no items remained open, and adjudicated as UAT test 2
  in `01-UAT.md`.

  When this was written nothing forced the check: Phase 6 SC6 covers only
  "Lengnick Table 1 rows" and this is a BAM row, so it fell outside that gate,
  while ROADMAP Phase 10 SC5 asserted "0.8x" as settled fact — the roadmap
  stating as true the very thing this item holds open. Both were corrected:

  - ROADMAP Phase 10 criterion 5 now references the config key
    `bankruptcy.entrant_size_ratio_ppm` rather than the literal 0.8x, so the
    criterion cannot be satisfied by asserting an unchecked number.
  - ROADMAP Phase 10 criterion 6 is a new **blocking gate**: V-3a adjudicated
    before **BANK-04** consumes the value, with the three readings enumerated
    and the D-20 prohibition restated.
  - ROADMAP Phase 6 criterion 6 asks that the two BAM rows (V-3, V-3a) be
    checked on the same journal pass — that is the phase where someone has
    access — while noting the blocking gate sits in Phase 10.
  - `.planning/STATE.md` records the gate alongside the item.

  The gate forces the check; it does not answer it. The value still requires a
  BAM read and must not be resolved from model memory (D-20).
