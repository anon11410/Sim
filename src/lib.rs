//! `sim` — the minimal closed-economy simulation library.
//!
//! Every simulation primitive lives here; `src/main.rs` is a thin CLI over this
//! surface and holds no simulation logic (CORE-08). Integration tests under
//! `tests/` reach the whole model through `use sim::…`.

#![forbid(unsafe_code)]

pub mod books;
pub mod config;
pub mod ids;
pub mod money;
pub mod numeric;
pub mod rng;
