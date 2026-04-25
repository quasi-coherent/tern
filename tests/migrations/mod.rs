use tern::TernApp;
use tern::executor::sqlx::SqlxPgExecutor;

#[derive(TernApp)]
#[tern(
    source = "tests/migrations/migrations01",
    table = "_test_derive_history"
)]
pub struct TestMigrate01 {
    #[tern(executor_via)]
    exec: SqlxPgExecutor,
}
