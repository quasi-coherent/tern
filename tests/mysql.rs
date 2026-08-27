//! TODO(tern-derive + testing rework): disabled for the 4.0 compile-fix pass.
//! Re-enable once the `TernApp`/`test_suite!` macros emit the new core API and
//! the testing facade is completed. Import relocations for reference:
//!   `tern::executor`               -> `tern::exec`
//!   `tern::testing::{Properties, TernTest, property_fn}` -> `Properties`/
//!     `property_fn` not yet exported by the facade
//! Original contents preserved below.
// use tern::TernApp;
// use tern::error::{TernError, TernResult};
// use tern::exec::{SqlxMySqlExecutor, sqlx};
// use tern::testing::{Properties, TernTest, property_fn};
//
// #[derive(TernApp)]
// #[tern(
// source = "tests/migrations/mysql/updown",
// table = "_tern_harness_mysql_history"
// )]
// pub struct MySqlUpDown {
// #[tern(executor_via)]
// pub exec: SqlxMySqlExecutor,
// }
//
// fn ensure(cond: bool, msg: &str) -> TernResult<()> {
// cond.then_some(()).ok_or_else(|| TernError::Invalid(msg.to_string()))
// }
//
// async fn count(app: &mut MySqlUpDown, sql: &str) -> TernResult<i64> {
// sqlx::query_scalar(sql)
// .fetch_one(app.exec.inner_mut())
// .await
// .map_err(TernError::exec_err)
// }
//
// async fn table_exists(app: &mut MySqlUpDown, table: &str) -> TernResult<bool>
// { count(
// app,
// &format!(
// "SELECT count(*) FROM information_schema.tables \
// WHERE table_schema = DATABASE() AND table_name = '{table}'"
// ),
// )
// .await
// .map(|n| n == 1)
// }
//
// async fn absent(app: &mut MySqlUpDown, table: &str) -> TernResult<()> {
// table_exists(app, table).await.and_then(|found| {
// ensure(!found, &format!("table {table} should not exist"))
// })
// }
//
// async fn present(app: &mut MySqlUpDown, table: &str) -> TernResult<()> {
// table_exists(app, table)
// .await
// .and_then(|found| ensure(found, &format!("table {table} should exist")))
// }
//
// fn mysql_updown_properties() -> Properties<MySqlUpDown> {
// Properties::new()
// .with(
// 1,
// property_fn(
// async |app: &mut MySqlUpDown| {
// absent(app, "tern_mysql_ud_a").await
// },
// async |app: &mut MySqlUpDown| {
// present(app, "tern_mysql_ud_a").await
// },
// ),
// )
// .with(
// 2,
// property_fn(
// async |app: &mut MySqlUpDown| {
// absent(app, "tern_mysql_ud_b1").await?;
// absent(app, "tern_mysql_ud_b2").await
// },
// async |app: &mut MySqlUpDown| {
// present(app, "tern_mysql_ud_b1").await?;
// present(app, "tern_mysql_ud_b2").await
// },
// ),
// )
// .with(
// 3,
// property_fn(
// async |app: &mut MySqlUpDown| {
// absent(app, "tern_mysql_ud_c").await
// },
// async |app: &mut MySqlUpDown| {
// present(app, "tern_mysql_ud_c").await?;
// let n = count(app, "SELECT count(*) FROM tern_mysql_ud_c")
// .await?;
// ensure(n == 2, "expected 2 rows in tern_mysql_ud_c")
// },
// ),
// )
// }
//
// tern::test_suite! {
// app = MySqlUpDown,
// source = "tests/migrations/mysql/updown",
// env = "MYSQL_ADMIN_DATABASE_URL",
// context = |exec| async move {
// Ok(MySqlUpDown { exec })
// },
// properties = mysql_updown_properties(),
// }
