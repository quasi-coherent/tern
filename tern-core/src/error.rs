//! Error type for migration operations.
use crate::migration::{Migration, MigrationId};
use crate::runner::{MigrationResult, Report};

use std::error::Error as StdError;

/// Alias for a result whose error type is [`Error`].
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
    #[error("error applying migration: {{name: {1}, no_tx: {2}}}: {0}")]
    ExecuteMigration(#[source] BoxDynError, MigrationId, bool),
    /// An error resolving the query before applying.
    /// Can be used as a fallthrough to map arbitrary error types to when
    /// implementing `QueryBuilder`.
    #[error("runtime could not resolve query: {0}")]
    ResolveQuery(String),
    /// Error processing a migration source.
    #[error("could not parse migration query: {0}")]
    Sql(#[from] std::fmt::Error),
    /// Local migration source has fewer migrations than the history table.
    #[error("missing source: {local} migrations found but {history} have been applied: {msg}")]
    MissingSource {
        local: i64,
        history: i64,
        msg: String,
    },
    /// The options passed are not valid.
    #[error("invalid parameter for the operation requested: {0}")]
    Invalid(String),
    /// An error occurred, resulting in a partial migration run.
    #[error("migration could not complete: {source}, partial report: {report}")]
    Partial { source: BoxDynError, report: Report },
}

impl Error {
    pub fn to_resolve_query_error<E>(e: E) -> Self
    where
        E: std::fmt::Display,
    {
        Self::ResolveQuery(e.to_string())
    }
}

/// Converting a result with a generic `std::error::Error` to one with this
/// crate's error type.
///
/// The `*_migration_result` methods allow attaching a migration to the error,
/// such as the one being handled when the error occurred.  The `with_report`
/// method allows attaching a slice of `MigrationResult` to the error to show
/// what collection of the migration set did succeed in being applied before the
/// error was encountered.
pub trait DatabaseError<T, E> {
    /// Convert `E` to an [`Error`].
    fn tern_result(self) -> TernResult<T>;

    /// Same as `tern_result` but discard the returned value.
    fn void_tern_result(self) -> TernResult<()>;

    /// Convert `E` to an [`Error`] that has a given migration in the error
    /// type's source.
    fn tern_migration_result<M: Migration + ?Sized>(self, migration: &M) -> TernResult<T>;

    /// Same as `tern_migration_result` but discard the returned value.
    fn void_tern_migration_result<M: Migration + ?Sized>(self, migration: &M) -> TernResult<()>;

    /// Attach an array of `MigrationResult`, representing a partially successful
    /// migration operation, to the error.
    fn with_report(self, report: &[MigrationResult]) -> TernResult<T>;
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

    fn void_tern_migration_result<M: Migration + ?Sized>(self, migration: &M) -> TernResult<()> {
        match self {
            Err(e) => Err(Error::ExecuteMigration(
                Box::new(e),
                migration.migration_id(),
                migration.no_tx(),
            )),
            _ => Ok(()),
        }
    }

    fn tern_result(self) -> TernResult<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Execute(Box::new(e))),
        }
    }

    fn tern_migration_result<M: Migration + ?Sized>(self, migration: &M) -> TernResult<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::ExecuteMigration(
                Box::new(e),
                migration.migration_id(),
                migration.no_tx(),
            )),
        }
    }

    fn with_report(self, migrations: &[MigrationResult]) -> TernResult<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Partial {
                source: Box::new(e),
                report: Report::new(migrations.to_vec()),
            }),
        }
    }
}
