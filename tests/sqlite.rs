use tern::TernApp;
use tern::error::{TernError, TernResult};
use tern::executor::{SqlxSqliteExecutor, sqlx};
use tern::test::{Properties, property_fn};

#[derive(TernApp)]
#[tern(
    source = "tests/migrations/sqlite/updown",
    table = "_tern_harness_sqlite_history"
)]
pub struct SqliteUpDown {
    #[tern(executor_via)]
    pub exec: SqlxSqliteExecutor,
}

fn ensure(cond: bool, msg: &str) -> TernResult<()> {
    cond.then_some(()).ok_or_else(|| TernError::Invalid(msg.to_string()))
}

async fn count(app: &mut SqliteUpDown, sql: &str) -> TernResult<i64> {
    sqlx::query_scalar(sql)
        .fetch_one(app.exec.inner_mut())
        .await
        .map_err(TernError::exec_err)
}

async fn table_exists(app: &mut SqliteUpDown, table: &str) -> TernResult<bool> {
    count(
        app,
        &format!(
            "SELECT count(*) FROM sqlite_master \
             WHERE type = 'table' AND name = '{table}'"
        ),
    )
    .await
    .map(|n| n == 1)
}

async fn absent(app: &mut SqliteUpDown, table: &str) -> TernResult<()> {
    table_exists(app, table).await.and_then(|found| {
        ensure(!found, &format!("table {table} should not exist"))
    })
}

async fn present(app: &mut SqliteUpDown, table: &str) -> TernResult<()> {
    table_exists(app, table)
        .await
        .and_then(|found| ensure(found, &format!("table {table} should exist")))
}

fn sqlite_updown_properties() -> Properties<SqliteUpDown> {
    Properties::new()
        .with(
            1,
            property_fn(
                async |app: &mut SqliteUpDown| {
                    absent(app, "tern_sqlite_ud_a").await
                },
                async |app: &mut SqliteUpDown| {
                    present(app, "tern_sqlite_ud_a").await
                },
            ),
        )
        .with(
            2,
            property_fn(
                async |app: &mut SqliteUpDown| {
                    absent(app, "tern_sqlite_ud_b1").await?;
                    absent(app, "tern_sqlite_ud_b2").await
                },
                async |app: &mut SqliteUpDown| {
                    present(app, "tern_sqlite_ud_b1").await?;
                    present(app, "tern_sqlite_ud_b2").await
                },
            ),
        )
        .with(
            3,
            property_fn(
                async |app: &mut SqliteUpDown| {
                    absent(app, "tern_sqlite_ud_c").await
                },
                async |app: &mut SqliteUpDown| {
                    present(app, "tern_sqlite_ud_c").await?;
                    let n = count(app, "SELECT count(*) FROM tern_sqlite_ud_c")
                        .await?;
                    ensure(n == 2, "expected 2 rows in tern_sqlite_ud_c")
                },
            ),
        )
}

tern::test_suite! {
    app = SqliteUpDown,
    source = "tests/migrations/sqlite/updown",
    url = "sqlite://unused-by-this-backend",
    context = |exec| SqliteUpDown { exec },
    properties = sqlite_updown_properties(),
}
