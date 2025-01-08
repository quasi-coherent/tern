use std::error::Error as StdError;

use crate::migration::{AppliedMigration, Migration};
use crate::runner::MigrationResult;

pub type TernResult<T> = Result<T, Error>;
type BoxDynError = Box<dyn StdError + Send + Sync + 'static>;

/// All the ways the lifecycle of applying migrations
/// can end in failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error that came from applying migrations.
    #[error("error applying migrations {0}")]
    Execute(#[source] BoxDynError),
    /// Error from one migration.
    #[error("error applying migration {1}: {0}")]
    ExecuteMigration(#[source] BoxDynError, i64),
    /// Error resolving query before migration.
    #[error("runtime could not resolve query {0}")]
    ResolveQuery(String),
    /// Error processing a migration source.
    #[error("could not parse migration query {0}")]
    Sql(#[from] std::fmt::Error),
    /// A migration found in schema history is missing from the source.
    #[error("migration {0} applied but missing in source")]
    VersionMissing(i64),
    /// The migration content has changed between runs.
    #[error("migration {0} modified since apply: applied {1}, source {2}")]
    VersionModified(i64, String, String),
    /// A version specified does not exist in the source.
    #[error("version {0} not found in source")]
    VersionNotPresent(i64),
    /// A version specified is older than would be valid
    /// for the operation requested.
    #[error("version {0} too old for the requested operation")]
    VersionTooOld(i64),
    /// A version specified is newer than would be valid
    /// for the operation requested.
    #[error("version {0} too new for the requested operation")]
    VersionTooNew(i64),
}

/// Converting a result with a generic `std::error::Error` to one with this
/// crate's error type.
pub trait DatabaseError<T, E> {
    fn void_tern_result(self) -> TernResult<()>;
    fn void_tern_migration_result(self, version: i64) -> TernResult<()>;
    fn tern_result(self) -> TernResult<T>;
    fn tern_migration_result(self, version: i64) -> TernResult<T>;
}

impl<T, E> DatabaseError<T, E> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn void_tern_result(self) -> TernResult<()> {
        match self {
            Err(e) => Err(Error::Execute(Box::new(e))),
            _ => Ok(()),
        }
    }

    fn void_tern_migration_result(self, version: i64) -> TernResult<()> {
        match self {
            Err(e) => Err(Error::ExecuteMigration(Box::new(e), version)),
            _ => Ok(()),
        }
    }

    fn tern_result(self) -> TernResult<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Execute(Box::new(e))),
        }
    }

    fn tern_migration_result(self, version: i64) -> TernResult<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::ExecuteMigration(Box::new(e), version)),
        }
    }
}

/// Converting the `Result<AppliedMigration, E>` of a migration attempt to the
/// pretty-printable `MigrationResult` (which combines Ok/Err).
pub trait ToMigrationResult<E> {
    fn to_migration_result<M>(self, migration: &M) -> MigrationResult
    where
        M: Migration + ?Sized;
}

impl<E> ToMigrationResult<E> for Result<AppliedMigration, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn to_migration_result<M>(self, migration: &M) -> MigrationResult
    where
        M: Migration + ?Sized,
    {
        match self {
            Ok(applied) => MigrationResult::from_applied(&applied, Some(migration.no_tx())),
            Err(e) => MigrationResult::from_failed(migration, e.to_string()),
        }
    }
}
