//! The main part to play in the way this migration tool works is given to the
//! traits `MigrationContext` and `MigrationSource`.  Derive macros exist for
//! both, provided that the type deriving `MigrationSource` is `super` to the
//! migration source directory if there are Rust migrations, since it has to
//! reference the module containing a Rust migration.
//!
//! For `MigrationSource`, the attribute `source` is the path to migrations
//! relative to the project root and it is required.  For `MigrationContext` the
//! attribute `table` allows you to create the schema history in a custom
//! location.  The field attribute `executor_via` points out the field that
//! should be used to implement the required functionality a database connection
//! would have.
use tern::MigrationContext;
use tern::{ContextOptions, SqlxPgExecutor, error::TernResult};

/// `SqlxPgExecutor` is a supported connection type and it already implements
/// `Executor`, so that's the field we decorate with the hint to use it for
/// the migration runner to run queries.  We're also going to use a custom
/// schema migration history table.
#[derive(MigrationContext)]
#[tern(source = "src/migrations", table = "example_schema_history")]
pub struct ExampleContext {
    #[tern(executor_via)]
    pub executor: SqlxPgExecutor,
    pub env: GetEnvVar,
}

impl ExampleContext {
    pub async fn new(db_url: &str) -> TernResult<Self> {
        let executor = SqlxPgExecutor::new(db_url).await?;
        Ok(Self {
            executor,
            env: GetEnvVar,
        })
    }

    /// The executor's underlying connection is used to query the same database
    /// we're running migrations on.  This example just gets the max value in a
    /// column, but that will be used to build the example dynamic migration.
    pub async fn max_value(&self) -> anyhow::Result<i64> {
        let max_val: i64 = sqlx::query_scalar("SELECT max(x) FROM dmd_test;")
            .fetch_optional(&self.executor.pool())
            .await?
            .unwrap_or_default();

        Ok(max_val)
    }
}

/// The additional behavior of this particular context is provided by
/// `GetEnvVar`.  It just gets an environment variable in this simple example,
/// but a type in the context could do nearly any arbitrary thing.
pub struct GetEnvVar;

impl GetEnvVar {
    pub fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// A type implementing `ContextOptions` is only needed when integrating the CLI
/// crate.  That's because the CLI creates a `Runner` and accepts the DB url
/// argument, so the implementation just allows the CLI to work with a generic
/// context.
pub struct ExampleOptions;

impl ContextOptions for ExampleOptions {
    type Ctx = ExampleContext;

    async fn connect(&self, db_url: &str) -> TernResult<ExampleContext> {
        ExampleContext::new(db_url).await
    }
}
