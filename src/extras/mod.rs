//! Additional utilities and add-ons.

#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "cli")]
pub use tern_cli::{self as cmd, TernCli, args};
