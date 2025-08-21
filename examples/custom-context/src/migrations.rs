use tern::cli::clap::{self, Args};
use tern::error::{Error, TernResult};
use tern::executor::SqlxPgExecutor;
use tern::{ConnectOptions, MigrationContext};

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
            .fetch_optional(self.executor.pool())
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

/// The CLI arguments required to build the example context.
#[derive(Debug, Args)]
pub struct ExampleOptions {
    /// Connection string--can be set via the environment variable `DATABASE_URL`
    #[clap(long, short = 'D', env)]
    db_url: Option<String>,
}
impl ConnectOptions for ExampleOptions {
    type Ctx = ExampleContext;

    async fn connect(&self) -> TernResult<ExampleContext> {
        let db_url = self
            .db_url
            .as_deref()
            .ok_or_else(|| Error::Init("missing db connection string".into()))?;

        ExampleContext::new(db_url).await
    }
}
