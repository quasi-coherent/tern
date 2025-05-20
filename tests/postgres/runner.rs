use crate::migrations::TestSqlxPgContext;

use std::sync::LazyLock;
use tern::error::TernResult;
use tern::{MigrationContext, Runner};

pub static DATABASE_URL: LazyLock<String> =
    LazyLock::new(|| std::env::var("DATABASE_URL").expect("missing DATABASE_URL"));

pub struct TestRunner(Runner<TestSqlxPgContext>, TestCheck);

impl TestRunner {
    pub async fn new() -> Self {
        let ctx = TestSqlxPgContext::new(&DATABASE_URL).await;
        let check = TestCheck::new().await;
        Self(Runner::new(ctx), check)
    }

    pub fn check(&self) -> &TestCheck {
        &self.1
    }

    pub async fn apply(&mut self, through: Option<i64>) -> TernResult<usize> {
        self.0
            .run_apply(through, false)
            .await
            .map(|report| report.count())
    }

    pub async fn soft_apply(&mut self, through: Option<i64>) -> TernResult<usize> {
        self.0
            .run_soft_apply(through, false)
            .await
            .map(|report| report.count())
    }

    pub async fn drop_history(&mut self) -> TernResult<()> {
        self.0.drop_history().await
    }
}

pub struct TestCheck(sqlx::PgPool);

impl TestCheck {
    pub async fn new() -> Self {
        let pool = sqlx::PgPool::connect(&DATABASE_URL).await.unwrap();
        Self(pool)
    }

    pub async fn get_applied(&self) -> Vec<i64> {
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM pg_history ORDER BY version;")
                .fetch_all(&self.0)
                .await
                .unwrap();

        versions
    }

    pub async fn users_table_exists(&self) -> bool {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_tables WHERE schemaname = 'pg_test' AND tablename = 'users';",
        )
        .fetch_one(&self.0)
        .await
        .unwrap();

        count == 1
    }
}
