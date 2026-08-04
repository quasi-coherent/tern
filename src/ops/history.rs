//! The `history` command.
//!
//! ## Subcommands
//!
//! * `DropIfExists` the history table.
//! * `CreateIfNotExists` history table for a migration set.
#[doc(inline)]
pub use tern_core::ops::ddl::{CreateIfNotExists, DropIfExists};
