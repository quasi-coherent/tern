//! Top-level application types.
//!
//! The module defines the main interface [`TernMigrate`], which combines a
//! custom context with a collection of migrations over that context.  This
//! combination is suitable for any operation performed during a database
//! migration.  These operations are organized and made available by the app
//! type [`Tern`].
use tern_core::context::MigrationContext;

use crate::migration::{DownMigrationSet, UpMigrationSet};

/// `TernMigrate` is a migration context combined with a set of migrations
/// constructing the database.
pub trait TernMigrate: MigrationContext {
    /// Migration set to construct the target database.
    fn up_migrations(&self) -> UpMigrationSet<Self>
    where
        Self: Sized;
}

/// The `TernMigrate` context is `Invertible` if it can produce a set of down
/// migrations for reverting the database to an earlier state.
pub trait Invertible: TernMigrate {
    /// Migration set to revert the version of the target database.
    fn down_migrations(&self) -> DownMigrationSet<Self>
    where
        Self: Sized;
}

/// `Tern` is the main application.
pub struct Tern<T> {
    /// Migrate.
    pub migrate: T,
}

impl<T: TernMigrate> Tern<T> {
    /// Create a new `Tern` app with migrations `T`.
    pub fn new(migrate: T) -> Self {
        Self { migrate }
    }
}
