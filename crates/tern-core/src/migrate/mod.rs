//! The main type for an app.
//!
//! [`TernMigrate`] combines migrations for building a database with a context
//! to run them.  This also defines [`Invertible`], which is the same except for
//! a set of migrations that deconstruct the database.
//!
//! This also defines `TernMigrateOp`, representing an operation to carry out
//! with a `TernMigrate` value, and several canonical operations are provided.
use futures_core::Future;

use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::migration::iter::{DownMigrationSet, UpMigrationSet};

pub mod ops;

/// A context for constructing the database with a set of up migrations over the
/// context.
pub trait TernMigrate: MigrationContext {
    /// Migration set to construct the target database.
    fn up_migrations(&self) -> UpMigrationSet<Self>
    where
        Self: Sized;
}

/// The `TernMigrate` context is `Invertible` if it can produce a set of down
/// migrations over the context for deconstructing the database.
pub trait Invertible: TernMigrate {
    /// Migration set to revert the version of the target database.
    fn down_migrations(&self) -> DownMigrationSet<Self>
    where
        Self: Sized;
}

/// An operation in a migration app.
pub trait TernMigrateOp<T: TernMigrate>: Send + Sync {
    /// The type of value returned on success.
    type Success: Send + Sync;

    /// The type of value returned when an error occurs.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute the operation.
    fn exec(
        &self,
        migrate: &mut T,
    ) -> impl Future<Output = Result<Self::Success, Self::Error>> + Send;
}

/// Configuration to build some `TernMigrate`.
pub trait TernOptions<T: TernMigrate> {
    /// Initialize `T`.
    fn connect(&self) -> impl Future<Output = TernResult<T>> + Send;
}
