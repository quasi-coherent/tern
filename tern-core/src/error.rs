//! Error interface for `tern`.
use std::error::Error as StdError;
use std::fmt::Display;

use crate::migration::MigrationId;

/// Alias for a result whose error type is [`TernError`].
pub type TernResult<T> = Result<T, TernError>;

/// All the ways the lifecycle of applying migrations can end in failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TernError {
    /// Error returned during the migration.
    #[error("error returned from migration operation: {0}")]
    Migration(#[source] Box<dyn MigrationError>),

    /// Error from an inner executor type.
    #[error(transparent)]
    Executor(Box<dyn StdError + Send + Sync>),

    /// Errors encountered while building a query.
    #[error("builder operation failed: {0}")]
    QueryBuilder(String),

    /// Error processing a migration source.
    #[error("could not parse migration query: {0}")]
    Sql(#[from] std::io::Error),

    /// There was an error while using the history table.
    #[error("error using history table: {0}")]
    History(&'static str),

    /// Local migration source has fewer migrations than the history table.
    #[error(
        "missing source: {local} migrations found but {history} have been applied: {msg}"
    )]
    MissingSource {
        /// The version in local source.
        local: i64,
        /// The version in the database.
        history: i64,
        /// Description of the error.
        msg: String,
    },

    /// Found a duplicate migration in the source.
    #[error("migration version {0} already exists")]
    Duplicate(i64),

    /// The source migrations and the history are not synchronized in a way that
    /// is expected.
    #[error("inconsistent migration source: {msg}: {at_issue:?}")]
    OutOfSync {
        /// Local migration IDs that are inconsistent with the history table.
        at_issue: Vec<MigrationId>,
        /// Description of the error.
        msg: String,
    },

    /// The options passed are not valid.
    #[error("invalid parameter for the operation requested: {0}")]
    Invalid(String),

    /// Failed operation.
    #[error("operation failed with migration {id}: {message}")]
    Op {
        /// The error message.
        message: String,
        /// The ID of the failed migration.
        id: MigrationId,
    },

    /// Failed sending statement.
    #[error("operation failed on statement {idx}: {message}")]
    Statement {
        /// The error message.
        message: String,
        /// The index of the statement that failed.
        idx: u32,
    },

    /// There is a missing down migration, or no down migrations altogether.
    #[error("missing version {0} down migration")]
    MissingDown(i64),
}

impl TernError {
    pub(crate) fn op<E: Display>(e: E, id: &MigrationId) -> Self {
        Self::Op { message: e.to_string(), id: id.clone() }
    }

    pub(crate) fn stat<E: Display>(e: E, idx: u32) -> Self {
        Self::Statement { message: e.to_string(), idx }
    }

    /// From an executor error.
    pub fn exec_err<E: StdError + Send + Sync + 'static>(e: E) -> Self {
        Self::Executor(Box::new(e))
    }
}

impl StdError for Box<dyn MigrationError> {}

/// An error that was returned during the course of a migration run.
pub trait MigrationError: StdError + Send + Sync + 'static {
    /// Primary human-readable error message.
    fn message(&self) -> String;

    /// Returns the kind of error, if supported.
    fn kind(&self) -> ErrorKind;
}

impl<E: MigrationError> From<E> for TernError {
    fn from(value: E) -> Self {
        Self::Migration(Box::new(value))
    }
}

/// The error kind.
///
/// This enum is to be used to identify common categories of error.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Error that occurred during migration source validation.
    Validation,
    /// Error coming from the underlying [`MigrationExecutor`].
    ///
    /// [`MigrationExecutor`]: crate::executor::MigrationExecutor
    Executor,
    /// Error that occurred during some administrative operation.
    Admin,
    /// Error coming from a custom context.
    Custom,
    /// Error during a migration operation.
    Ops,
    /// Other.
    #[default]
    Other,
}

/// Utility to append an available migration ID to the error.
pub trait ResultWithId<T> {
    /// `TernResult` with error having the ID of the failed migration.
    fn map_err_id(self, id: &MigrationId) -> TernResult<T>;
}

impl<T, E: Display> ResultWithId<T> for Result<T, E> {
    fn map_err_id(self, id: &MigrationId) -> TernResult<T> {
        match self {
            Err(e) => Err(TernError::op(e, id)),
            Ok(v) => Ok(v),
        }
    }
}
