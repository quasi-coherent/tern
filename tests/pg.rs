use tern::TernApp;
use tern::error::{TernError, TernResult};
use tern::executor::{SqlxPgExecutor, sqlx};
use tern::test::{Properties, property_fn};

#[derive(TernApp)]
#[tern(
    source = "tests/migrations/pg/updown",
    table = "_tern_harness_pg_history"
)]
pub struct PgUpDown {
    #[tern(executor_via)]
    pub exec: SqlxPgExecutor,
}

fn ensure(cond: bool, msg: &str) -> TernResult<()> {
    cond.then_some(()).ok_or_else(|| TernError::Invalid(msg.to_string()))
}

async fn count(app: &mut PgUpDown, sql: &str) -> TernResult<i64> {
    sqlx::query_scalar(sql)
        .fetch_one(app.exec.inner_mut())
        .await
        .map_err(TernError::exec_err)
}

async fn table_exists(app: &mut PgUpDown, table: &str) -> TernResult<bool> {
    count(
        app,
        &format!(
            "SELECT count(*) FROM pg_tables WHERE tablename = '{table}' \
             AND schemaname = current_schema()"
        ),
    )
    .await
    .map(|n| n == 1)
}

async fn absent(app: &mut PgUpDown, table: &str) -> TernResult<()> {
    table_exists(app, table).await.and_then(|found| {
        ensure(!found, &format!("table {table} should not exist"))
    })
}

async fn present(app: &mut PgUpDown, table: &str) -> TernResult<()> {
    table_exists(app, table)
        .await
        .and_then(|found| ensure(found, &format!("table {table} should exist")))
}

fn pg_updown_properties() -> Properties<PgUpDown> {
    Properties::new()
        .with(
            1,
            property_fn(
                async |app: &mut PgUpDown| absent(app, "tern_pg_ud_a").await,
                async |app: &mut PgUpDown| present(app, "tern_pg_ud_a").await,
            ),
        )
        .with(
            2,
            property_fn(
                async |app: &mut PgUpDown| {
                    absent(app, "tern_pg_ud_b1").await?;
                    absent(app, "tern_pg_ud_b2").await
                },
                async |app: &mut PgUpDown| {
                    present(app, "tern_pg_ud_b1").await?;
                    present(app, "tern_pg_ud_b2").await
                },
            ),
        )
        .with(
            3,
            property_fn(
                async |app: &mut PgUpDown| absent(app, "tern_pg_ud_c").await,
                async |app: &mut PgUpDown| {
                    present(app, "tern_pg_ud_c").await?;
                    let n =
                        count(app, "SELECT count(*) FROM tern_pg_ud_c").await?;
                    ensure(n == 2, "expected 2 rows in tern_pg_ud_c")
                },
            ),
        )
}

tern::test_suite! {
    app = PgUpDown,
    source = "tests/migrations/pg/updown",
    env = "PG_DATABASE_URL",
    context = |exec| PgUpDown { exec },
    properties = pg_updown_properties(),
}
