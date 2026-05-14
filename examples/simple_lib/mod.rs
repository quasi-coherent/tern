//! # simple
//!
//! This simple example shows how a custom context can be used to inject logic
//! into a migration at runtime.
use tern::TernMigrate;
use tern::error::{ErrorKind, MigrationError, TernResult};
use tern::executor::sqlx::{SqlxError, SqlxPgExecutor, sqlx_lib};

/// The custom `TernMigrate` context.
///
/// Every context requires some `Executor`.  The attribute `executor_via` can
/// be used to point to the field containing a value that implements `Executor`.
/// Without this, `SimpleMigrate` itself would need that.
///
/// The custom nature of this context is `GetEnvVar`, which gets the value of an
/// environment variable, and the method `get_max_x`, which gets the current max
/// value in the column `x`.  These values are interpolated in the INSERT
/// statement of a migration.
///
/// This is very contrived, but demonstrates the capability.
#[derive(TernMigrate)]
#[tern(source = "examples/simple_lib/migrations", table = "_simple_history")]
pub struct SimpleMigrate {
    /// The type that impls `Executor`.
    ///
    /// This is the minimum required of a `TernMigrate` and beyond this, any
    /// number of any other type of value can be added.
    #[tern(executor_via)]
    pub exec: SqlxPgExecutor,
    /// Whatever.
    pub env: GetEnvVar,
}

impl SimpleMigrate {
    /// Simple init from a connection string.  `SqlxPgExecutor` can also be
    /// created from `sqlx` types `PoolOptions<Db>` and `ConnectOptions<Db>`
    /// for more flexibility.  These symbols are exported by the module
    /// `tern::sqlx::postgres`.
    ///
    /// Additionally, `SqlxPgExecutor` has the method `inner` exposing the inner
    /// `PgPool` for when the result of an arbitrary query is needed.
    pub async fn new(db_url: &str) -> TernResult<Self> {
        let exec = SqlxPgExecutor::new(db_url).await?;
        Ok(Self { exec, env: GetEnvVar })
    }

    /// Gets the maximum `x` in the table `simple_example`.
    pub async fn get_max_x(&self) -> TernResult<i64> {
        let maxx: i64 =
            sqlx_lib::query_scalar("SELECT max(x) FROM simple_example;")
                .fetch_optional(self.exec.inner())
                .await
                .map_err(SqlxError::from)?
                .unwrap_or_default();

        Ok(maxx)
    }
}

/// Value we put in the context.  It adds environment variables to the migration
/// context.
pub struct GetEnvVar;

impl GetEnvVar {
    /// Get the var as a string.
    pub fn get_var(&self, key: &str) -> TernResult<String> {
        let var = std::env::var(key).map_err(ExampleError::Unset)?;
        Ok(var)
    }

    /// Get an arbitrary `T: FromStr`.
    pub fn get_from_str<T>(&self, key: &str) -> Option<T>
    where
        T: std::str::FromStr,
    {
        let Ok(v) = self.get_var(key) else {
            return None;
        };
        T::from_str(&v).ok()
    }
}

/// A custom error type.
///
/// It implements `MigrationError`, so `?`-s into `TernError`.
#[derive(Debug, thiserror::Error)]
pub enum ExampleError {
    #[error("variable not found in environment: {0}")]
    Unset(std::env::VarError),
    #[error("error writing to query: {0}")]
    Query(std::fmt::Error),
}

impl MigrationError for ExampleError {
    fn message(&self) -> String {
        self.to_string()
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Custom
    }
}
