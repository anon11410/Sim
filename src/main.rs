//! The CLI. Three flags, no simulation logic (CORE-08).
//!
//! `anyhow` appears in this file and nowhere else — the library uses typed
//! `thiserror` errors so callers can match on them.
//!
//! The body is deliberately one straight sequence with two subtleties in it.
//!
//! First, the run log is **finished before the run's outcome is inspected**.
//! Terminating the process runs no destructors, so a buffered writer would lose
//! whatever it still held — and the ticks leading to a halt are exactly the
//! diagnostic evidence someone needs. The comma-separated writer's own
//! drop-time flush also discards its error, so a write failure would be silent.
//! `finish` is the only place either is caught.
//!
//! Second, the run record is written on **both** paths — a clean finish and a
//! halt — and before the process exits. A halted run is then self-describing:
//! its own record says how far it got and that it ended in a violation, which
//! is the difference between a short log someone has to explain and a short log
//! that explains itself.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

use sim::books::Books;
use sim::invariants::CheckSet;
use sim::log::{RUN_META_FILE, RunWriter, SCHEMA_VERSION, Sink};
use sim::phases::Ctx;
use sim::rng::Rngs;
use sim::world::World;

/// The compiler that built this binary, captured by `build.rs` at build time.
///
/// Read from a compile-time value rather than by invoking the compiler during a
/// run: a process spawn has no business on the behaviour path, and this string
/// is not behaviour — it reaches only the run record.
const RUSTC_VERSION: &str = env!("SIM_RUSTC_VERSION");

/// A clean finish, as the run record spells it.
const EXIT_OK: &str = "ok";

/// A run stopped by the invariant phase, as the run record spells it.
const EXIT_VIOLATION: &str = "violation";

/// The run's own record: what ran, on what, and how it ended (TICK-05).
///
/// **The single quarantined file in the run directory.** It is excluded from
/// the determinism diff and is the only file that may carry a wall clock — a
/// permission for a start time, not a licence to put the environment beside the
/// logs. Every value here is an integer, a fixed word or a hex digest.
///
/// **There is deliberately no duration field.** A duration differs between two
/// otherwise identical runs, so a determinism test that compared this file
/// would fail for a reason that has nothing to do with the economy — and the
/// natural repair is to widen the comparison, which would be permanent while
/// the reason for it would be forgotten. `ticks_completed` and `exit` carry
/// everything a reader needs about how the run ended.
///
/// **No path, host name or process identifier.** The exclusion from the diff is
/// about the wall clock; it is not a general environment allowance for a file
/// that ships beside the logs.
///
/// The field order is the emitted order.
#[derive(Debug, Serialize)]
struct RunMeta<'a> {
    schema_version: &'a str,
    seed: u64,
    config_sha256: &'a str,
    rustc: &'a str,
    ticks_completed: u32,
    exit: &'a str,
}

/// Write the run record into `dir`.
///
/// **Pretty-printed, one field per line.** The file is excluded from the
/// determinism diff, so the shape costs nothing elsewhere, and one field per
/// line is what makes the record greppable field by field — a compact single
/// line answers "how many of these three fields are present?" with one however
/// many are there.
fn write_run_meta(dir: &Path, meta: &RunMeta<'_>) -> Result<()> {
    let path = dir.join(RUN_META_FILE);
    let mut file = File::create(&path)
        .with_context(|| format!("creating the run record {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, meta)
        .with_context(|| format!("writing the run record {}", path.display()))?;
    file.write_all(b"\n")
        .and_then(|()| file.flush())
        .with_context(|| format!("finishing the run record {}", path.display()))?;
    Ok(())
}

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

    // The hash goes into the run record below, which is the artefact that makes
    // a run reproducible from its own directory. Loading it here rather than
    // there keeps the whole input to a run read in one place, and guarantees the
    // hash and the parameters describe the same bytes.
    let (params, config_sha256) = sim::config::load(&cli.config)
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

    // How far the run got, on both paths. On a halt the failing tick did not
    // complete — the invariant phase is position 7 and the log is position 8 —
    // so the tick number the world stopped on IS the count of completed ticks.
    // On a clean finish every configured tick completed.
    let (ticks_completed, exit) = match &outcome {
        Ok(()) => (params.sim.ticks, EXIT_OK),
        Err(_) => (world.tick, EXIT_VIOLATION),
    };
    write_run_meta(
        &cli.out,
        &RunMeta {
            schema_version: SCHEMA_VERSION,
            seed: effective_seed,
            config_sha256: &config_sha256,
            rustc: RUSTC_VERSION,
            ticks_completed,
            exit,
        },
    )?;

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
