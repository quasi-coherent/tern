use std::error::Error as StdError;

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
    /// Other.
    #[default]
    Other,
}
