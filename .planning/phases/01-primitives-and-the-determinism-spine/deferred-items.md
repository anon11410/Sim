
## From plan 01-04 (2026-08-30)

- **`cargo fmt --check` fails on files this plan does not own.** `src/money.rs`
  (10 diffs, from plan 01-03) and `tests/tracer_end_to_end.rs` (4 diffs, from
  plan 01-01) are not rustfmt-clean. `src/rng.rs` and `tests/determinism_rng.rs`
  were brought to rustfmt-clean by this plan; the other two were deliberately
  left alone under the executor scope boundary (out-of-scope files are not
  auto-fixed). Whitespace only — no behaviour implication. Natural owner is plan
  **01-07**, which builds the clippy/CI wall: if `cargo fmt --check` joins CI it
  must be preceded by a one-shot repo-wide `cargo fmt`.
