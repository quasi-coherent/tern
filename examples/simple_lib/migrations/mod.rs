//! This module defines the migrations and the type deriving [`TernApp`] that
//! will run them.
//!
//! [`TernApp`]: tern::TernApp
use tern::error::TernResult;
use tern::executor::{ConnStr, SqlxError, SqlxPgExecutor, sqlx};
use tern::{ExecutorOptions, TernApp};

use super::ExampleError;

/// This example's `TernApp`.
///
/// To use the derive macro for `TernApp`, a [`MigrationExecutor`] has to be
/// accessible.  The attribute `executor_via` can be used to point to a field
/// containing one.
///
/// Here we're using `SqlxPgExecutor`, provided by `tern`, that uses sqlx and
/// connects to a postgres database under the hood.  Without the attribute, we'd
/// need to have `SimpleExample` provide the `MigrationExecutor` methods.
///
/// The custom nature of this context is `GetEnvVar`, which gets the value of an
/// environment variable, and the method `get_max_x`, which gets the current max
/// value in the column `x`.  These values are interpolated into the INSERT
/// statement of a migration.
///
/// This is very contrived, but demonstrates the capability.
///
/// [`MigrationExecutor`]: tern::executor::MigrationExecutor
#[derive(Debug, TernApp)]
#[tern(source = "examples/simple_lib/migrations", table = "_simple_history")]
pub struct SimpleExample {
    /// The type that impls `MigrationExecutor`.  This can be created using the
    /// [`ConnStr`] type.
    ///
    /// This is the minimum required of a `TernApp` and beyond this practically
    /// any number of any other type of value can be added.
    ///
    /// [`ConnStr`]: tern::executor::sqlx::ConnStr
    #[tern(executor_via)]
    pub exec: SqlxPgExecutor,
    /// Whatever the heart desires.
    pub env: GetEnvVar,
}

impl SimpleExample {
    /// To run the app from a CLI we can define a type `U` that implements both
    /// `clap::Args` and `tern::ContextOptions<SimpleExample>`:
    ///
    /// ```rust,ignore
    /// # use tern::{Tern, ContextOptions};
    /// # async fn f(_: U) where U: ContextOptions<SimpleExample> + clap::Args {
    /// use tern::Tern;
    ///
    /// match Tern::<SimpleExample>::run_with::<U>(|opts| opts.initialize()).await {
    ///     Ok(v) => println!("operation succeeded: {v}"),
    ///     Err(e) => println!("operation failed: {e}"),
    /// }
    /// # }
    /// ```
    ///
    /// Here, we don't have to define anything because () already works with
    /// this method.  In this case, we can use something simpler:
    ///
    /// ```rust,ignore
    /// # use tern::{Tern, ContextOptions};
    /// # async fn f() {
    /// let result = Tern::run_new(SimpleExample::init).await;
    /// # }
    /// ```
    pub async fn new(conn: ConnStr) -> TernResult<Self> {
        let exec = SqlxPgExecutor::new(&conn).await?;
        Ok(Self { exec, env: GetEnvVar })
    }

    /// Gets the maximum `x` in the table `simple_example`.
    pub async fn get_max_x(&mut self) -> TernResult<i64> {
        let maxx: i64 =
            sqlx::query_scalar("SELECT max(x) FROM simple_example;")
                .fetch_optional(self.exec.inner_mut())
                .await
                .map_err(SqlxError::from)?
                .unwrap_or_default();
        Ok(maxx)
    }
}

/// Value we put in the context.  It adds environment variables to the migration
/// context.
#[derive(Clone, Copy, Debug, Default)]
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
        let Ok(v) = Self.get_var(key) else {
            return None;
        };
        T::from_str(&v).ok()
    }
}
