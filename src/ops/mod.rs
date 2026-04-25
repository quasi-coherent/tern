//! Operations with `tern` migrations.
//!
//! This module provides the main commands: `history`, `migrate`, and `source`.
//!
//! Each command has subcommands; see the documentation.
pub use tern_core::ops::MigrationOp;

pub mod history;
pub mod migrate;
pub mod source;

mod result;
pub use result::{OpComplete, OpError, OpResult, OpSuccess};

/// Atomic operations.
///
/// These can be used as the unit to forming a larger operation.
pub mod core {
    pub use tern_core::ops::crud::*;
    pub use tern_core::ops::migration::*;
}
