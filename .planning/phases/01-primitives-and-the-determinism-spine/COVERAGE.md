# API Coverage — Phase 01: Primitives and the Determinism Spine

No external API integration: this phase builds a self-contained Rust simulation
crate with no network surface of any kind. There is no external API, service, or
SDK to enumerate, so a capability matrix would have to be fabricated to exist.

## Why the detector fired

The `api-coverage` detector requires an integration verb and an external-API noun
in the same clause. Three lines in this phase's artifacts satisfy that pattern
while describing the crate's **own** Rust API, not an external one:

| File | Line | Text |
|---|---|---|
| `01-CONTEXT.md` | 268 | "**Phase 2 (ledger)** consumes `Money`, its checked API and `split`" |
| `01-01-SUMMARY.md` | 267 | "**Phase 2 (ledger)** consumes `Money` and its checked API" |
| `01-03-PLAN.md` | 46 | "config ingestion consumes the Result-returning named API, never the panicking operator" |

In all three, "API" is the first-party surface of the `Money` newtype and the
config loader — the boundary between Phase 1 and Phase 2 of this same crate.

## Evidence of no external surface

- `Cargo.toml` dependencies are `rand`, `serde`, `toml`, `sha2`, `thiserror`,
  `clap`, `anyhow`. None performs I/O beyond the local filesystem.
- `Cargo.lock` contains no HTTP or async-runtime crate — no `reqwest`, `hyper`,
  `ureq`, `curl`, `tokio`, `http`, `surf`, or `isahc`.
- `src/` contains no URLs, no `fetch`, and no endpoint references.
- The project brief mandates a closed economy whose only inputs are the config
  file and `--seed`, and whose only outputs are log files on disk. An external
  call would be an invisible input and would break the reproducibility contract
  that is this phase's entire purpose.

## Scope

This declaration covers Phase 01 only. A later phase that genuinely integrates an
external API produces its own coverage matrix; the gate stays armed.
