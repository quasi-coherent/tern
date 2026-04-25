//! The `migrate` command.
//!
//! ## Subcommands
//!
//! * `Apply` one or more migrations
//! * `Revert` one or more migrations
mod apply;
pub use apply::{Apply, ApplyArgs, ApplyInput};

mod revert;
pub use revert::{Revert, RevertArgs, RevertInput};
