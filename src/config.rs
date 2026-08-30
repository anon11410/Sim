//! The parameter file is the whole input.
//!
//! Two rules make this reproducible (CORE-10):
//!
//! 1. `deny_unknown_fields` on **every** struct, not just the root — the root
//!    attribute alone catches a stray table but not a stray key inside a
//!    nested one.
//! 2. The config hash is taken over the **raw file bytes**, never over the
//!    parsed `Params`. A hash of a Rust value is not stable across compiler
//!    releases, and hashing bytes means a comment change (which carries a
//!    parameter's source grade) also changes the hash — which is correct.
//!
//! No field in this file is `Option<T>` and none carries a serde `default`:
//! either is an invisible input that would not appear in the committed config.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Everything that can go wrong turning a path into a `Params`.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config file `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("config file `{path}` is not valid UTF-8")]
    Utf8 {
        path: PathBuf,
        #[source]
        source: std::str::Utf8Error,
    },
    #[error("could not parse config file `{path}`")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// The whole parameter tree. Widened to the full parameter set by plan 01-06.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub sim: Sim,
    pub money: MoneySection,
}

/// Run shape: how long, how many agents, and the seed of record.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sim {
    pub ticks: u32,
    pub seed: u64,
    pub households: u32,
    pub firms: u32,
}

/// The money stock. Integer cents — a float here would end conservation.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MoneySection {
    pub total_money_cents: i64,
}

/// Read `path` once as raw bytes, hash those bytes, and parse those same bytes.
///
/// Returns the parsed parameters and the lowercase hex SHA-256 of the file as
/// it was read — the two are guaranteed to describe the same byte sequence
/// because the file is read exactly once.
pub fn load(path: &Path) -> Result<(Params, String), ConfigError> {
    let bytes = std::fs::read(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let hash = config_hash(&bytes);

    let text = std::str::from_utf8(&bytes).map_err(|source| ConfigError::Utf8 {
        path: path.to_path_buf(),
        source,
    })?;

    let params: Params = toml::from_str(text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;

    Ok((params, hash))
}

/// Lowercase hex SHA-256 of `bytes`.
///
/// Hex is built byte by byte rather than through a `LowerHex` impl on the
/// digest type, so the idiom survives a future `sha2` major bump.
pub fn config_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest.iter() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}
