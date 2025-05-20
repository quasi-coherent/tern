use tern::executor::SqlxPgExecutor;
use tern::{MigrationContext, MigrationSource};

#[derive(MigrationContext, MigrationSource)]
#[tern(source = "tests/postgres/migrations", table = "pg_history")]
pub struct TestSqlxPgContext {
    #[tern(executor_via)]
    executor: SqlxPgExecutor,
}

impl TestSqlxPgContext {
    pub async fn new(db_url: &str) -> Self {
        let executor = SqlxPgExecutor::new(db_url).await.unwrap();
        Self { executor }
    }
}
