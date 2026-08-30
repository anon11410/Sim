//! The CLI. Three flags, no simulation logic (CORE-08).
//!
//! `anyhow` appears in this file and nowhere else — the library uses typed
//! `thiserror` errors so callers can match on them.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use sim::money::Money;
use sim::rng::{Purpose, Rngs};

/// A minimal closed-economy simulation.
#[derive(Debug, Parser)]
#[command(name = "sim", version, about)]
struct Cli {
    /// Parameter file. The whole input, besides `--seed`.
    #[arg(long)]
    config: PathBuf,

    /// Override the seed in the config file. The value that runs is the value
    /// recorded (D-26) — a run must be reproducible from its own record.
    #[arg(long)]
    seed: Option<u64>,

    /// Directory for run output. Affects no behaviour.
    #[arg(long)]
    out: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (params, config_sha256) = sim::config::load(&cli.config)
        .with_context(|| format!("loading config from {}", cli.config.display()))?;

    // The effective seed: the override when present, the config value
    // otherwise. This is the value printed, and the value later persisted.
    let effective_seed = cli.seed.unwrap_or(params.sim.seed);

    if let Some(out) = &cli.out {
        // `--out` is an operator-supplied path joined only with fixed
        // filenames, never assembled from config content (threat T-1-04).
        std::fs::create_dir_all(out)
            .with_context(|| format!("creating output directory {}", out.display()))?;
    }

    let rngs = Rngs::new(effective_seed);
    let mut probe = rngs.stream(0, 0, Purpose::TracerProbe);
    let draw = probe.below(1_000_000);

    let money = Money::from_cents(params.money.total_money_cents) + Money::ZERO;

    println!(
        "tracer effective_seed={effective_seed} config_sha256={config_sha256} draw={draw} money_cents={}",
        money.cents()
    );

    Ok(())
}
