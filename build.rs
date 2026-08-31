//! Capture the compiler's version string at **build** time, for `run_meta.json`
//! (TICK-05).
//!
//! The alternative — invoking the compiler during a run — would put a process
//! spawn on the behaviour path for a string the model never reads. A build
//! script keeps it off entirely: the version is baked into the binary as a
//! compile-time environment value and the run does nothing but copy it into its
//! own record.
//!
//! **This file is not on the behaviour path and is not searched by the source
//! guards**, every one of which derives its file set from
//! `git ls-files -- 'src/*.rs'`. That is why the environment access below is
//! legitimate here and nowhere else: a run may not read its environment, but
//! the build that produces the run may describe itself.
//!
//! `RUSTC` is read from the environment rather than assumed, because Cargo sets
//! it to the compiler it is actually driving — which under a `rust-toolchain.toml`
//! pin is a shim, not whatever `rustc` a bare `PATH` lookup would find.
//!
//! The version string reaches only `run_meta.json`, which is excluded from the
//! determinism diff. No diffed file carries it, so a machine with a different
//! compiler produces byte-identical logs and a distinguishable run record —
//! which is the point of recording it at all.

fn main() {
    let compiler = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let version = std::process::Command::new(compiler)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        // A build that cannot describe its own compiler still builds, and says
        // so in the record rather than failing or inventing a plausible
        // version. A wrong version string in a reproducibility artefact is
        // worse than an admitted unknown.
        .unwrap_or_else(|| "unknown".to_owned());

    println!("cargo:rustc-env=SIM_RUSTC_VERSION={version}");
    println!("cargo:rerun-if-changed=build.rs");
}
