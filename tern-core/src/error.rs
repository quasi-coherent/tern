//! Error type for migration operations.
use crate::source::{Migration, MigrationId};

use std::error::Error as StdError;
use std::fmt::{Debug, Display};

/// Alias for a result whose error type is [Error].
pub type TernResult<T> = Result<T, Error>;
type BoxDynError = Box<dyn StdError + Send + Sync + 'static>;

/// All the ways the lifecycle of applying migrations can end in failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Error arising from initialization.
    #[error("could not initialize migration application: {0}")]
    Init(#[source] BoxDynError),
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
    /// The source migrations and the history are not synchronized in a way that
    /// is expected.
    #[error("inconsistent source: {msg}: {at_issue:?}")]
    OutOfSync {
        at_issue: Vec<MigrationId>,
        msg: String,
    },
    /// The options passed are not valid.
    #[error("invalid parameter for the operation requested: {0}")]
    Invalid(String),
    /// An error occurred, resulting in a partial migration run.
    #[error("migration could not complete: {source}, partial report: {report}")]
    Partial {
        source: BoxDynError,
        report: Box<dyn ReportFmt>,
    },
}

impl Error {
    pub fn to_resolve_query_error<E>(e: E) -> Self
    where
        E: std::fmt::Display,
    {
        Self::ResolveQuery(e.to_string())
    }
}

#[doc(hidden)]
pub trait ReportFmt: Send + Sync + Debug + Display + 'static {}
impl<T: Send + Sync + Debug + Display + 'static> ReportFmt for T {}

/// `DatabaseError` provides methods for converting generic errors into [Error]
/// with or without the context of a [Migration](crate::source::Migration).
pub trait DatabaseError<T, E> {
    /// Convert `E` to an [Error].
    fn tern_result(self) -> TernResult<T>;

    /// Same as [tern_result](DatabaseError::tern_result) but discard the
    /// returned value.
    fn void_tern_result(self) -> TernResult<()>;

    /// Convert `E` to an [Error] that has a given migration in the error type's
    /// source.
    fn tern_migration_result<M: Migration + ?Sized>(self, migration: &M) -> TernResult<T>;

    /// Same as [tern_migration_result](DatabaseError::tern_migration_result) but
    /// discard the returned value.
    fn void_tern_migration_result<M: Migration + ?Sized>(self, migration: &M) -> TernResult<()>;
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
}
