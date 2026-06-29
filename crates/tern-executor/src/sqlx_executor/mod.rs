use std::fmt::{self, Display, Formatter};
use tern_core::error::{ErrorKind, MigrationError};

mod any;
pub use any::{SqlxAnyExecutor, SqlxAnyExecutorOptions};

#[cfg(feature = "sqlx_mysql")]
pub mod mysql;

#[cfg(feature = "sqlx_postgres")]
pub mod postgres;

#[cfg(feature = "sqlx_sqlite")]
pub mod sqlite;

#[derive(Debug, thiserror::Error)]
pub struct SqlxError(sqlx::Error);

impl Display for SqlxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl MigrationError for SqlxError {
    fn message(&self) -> String {
        self.to_string()
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Executor
    }
}

impl From<sqlx::Error> for SqlxError {
    fn from(value: sqlx::Error) -> Self {
        Self(value)
    }
}
