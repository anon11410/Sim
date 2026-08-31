//! The CLI. Three flags, no simulation logic (CORE-08).
//!
//! `anyhow` appears in this file and nowhere else — the library uses typed
//! `thiserror` errors so callers can match on them.
//!
//! The body is deliberately one straight sequence with one subtlety in it: the
//! run log is **finished before the run's outcome is inspected**. Terminating
//! the process runs no destructors, so a buffered writer would lose whatever it
//! still held — and the ticks leading to a halt are exactly the diagnostic
//! evidence someone needs. The comma-separated writer's own drop-time flush
//! also discards its error, so a write failure would be silent. `finish` is the
//! only place either is caught.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use sim::books::Books;
use sim::invariants::CheckSet;
use sim::log::{RunWriter, Sink};
use sim::phases::Ctx;
use sim::rng::Rngs;
use sim::world::World;

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
    #[arg(long, default_value = "runs/latest")]
    out: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // The hash is bound and not yet consumed: plan 03-03 writes it into the
    // run record, which is the artefact that makes a run reproducible from its
    // own directory. Loading it here rather than there keeps the whole input to
    // a run read in one place.
    let (params, _config_sha256) = sim::config::load(&cli.config)
        .with_context(|| format!("loading config from {}", cli.config.display()))?;

    // The effective seed: the override when present, the config value
    // otherwise. This is the value that runs, and the value that is recorded.
    let effective_seed = cli.seed.unwrap_or(params.sim.seed);

    let mut books = Books::new(&params).context("opening the books from the configuration")?;
    let mut world = World::new(&params);
    let rngs = Rngs::new(effective_seed);
    let checks = CheckSet::from_params(&params);

    // `--out` is an operator-supplied path joined only with fixed
    // filenames, never assembled from config content (threat T-1-04,
    // continued as T-03-04).
    let mut writer = RunWriter::new(&cli.out)
        .with_context(|| format!("opening the run directory {}", cli.out.display()))?;

    let outcome = {
        let mut ctx = Ctx {
            world: &mut world,
            books: &mut books,
            rngs: &rngs,
            checks: &checks,
            sink: &mut writer,
        };
        sim::phases::run(&mut ctx, params.sim.ticks)
    };

    // BEFORE the outcome is inspected. See the module docs.
    writer
        .finish()
        .with_context(|| format!("finishing the run log in {}", cli.out.display()))?;

    if let Err(violation) = outcome {
        // The rendered violation and nothing else: it names the tick and the
        // check, and carries no path, host name, process identifier or
        // wall-clock reading (TICK-06, T-03-05). Adding context here would put
        // the environment back into a message that is read beside a diffed log.
        eprintln!("{violation}");
        std::process::exit(1);
    }

    Ok(())
}
