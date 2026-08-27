use std::fmt::{self, Display, Formatter};
use tern_core::error::{ErrorKind, MigrationError};

mod any;
pub use any::{SqlxAnyExecutor, SqlxAnyExecutorOptions};

#[cfg(feature = "sqlx_mysql")]
mod mysql;
pub use mysql::{SqlxMySqlExecutor, SqlxMySqlExecutorOptions};

#[cfg(feature = "sqlx_postgres")]
mod postgres;
pub use postgres::{SqlxPgExecutor, SqlxPgExecutorOptions};

#[cfg(feature = "sqlx_sqlite")]
mod sqlite;
pub use sqlite::{SqlxSqliteExecutor, SqlxSqliteExecutorOptions};

/// Database errors from the underlying `sqlx` crate.
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

impl From<url::ParseError> for SqlxError {
    fn from(value: url::ParseError) -> Self {
        let err = sqlx::Error::Configuration(Box::new(value));
        Self(err)
    }
}
