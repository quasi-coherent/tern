use std::fmt::{self, Display, Formatter};
use tern_core::error::{ErrorKind, MigrationError};

pub mod any;

#[cfg(feature = "sqlx_mysql")]
pub mod mysql;

#[cfg(feature = "sqlx_postgres")]
pub mod postgres;

#[cfg(feature = "sqlx_sqlite")]
pub mod sqlite;

/// An error coming from `sqlx`.
///
/// Converts to [`TernError`](tern_core::error::TernError).
#[derive(Debug, thiserror::Error)]
pub struct SqlxError(sqlx::Error, Option<usize>);

impl SqlxError {
    fn err_idx(e: sqlx::Error, idx: usize) -> Self {
        Self(e, Some(idx))
    }
}

impl Display for SqlxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.1.unwrap_or_default(), self.0)
    }
}

impl From<sqlx::Error> for SqlxError {
    fn from(value: sqlx::Error) -> Self {
        Self(value, None)
    }
}

impl MigrationError for SqlxError {
    fn message(&self) -> String {
        let err = self.0.to_string();
        match self.1 {
            Some(idx) => format!("at statement {idx}: {err}"),
            _ => err,
        }
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Executor
    }
}
