//! TODO(tern-derive rework): orphan module, not built by any current target
//! (no `tests/*.rs` declares `mod migrations`). It also points at a
//! `tests/migrations/migrations01` source dir that does not exist. Disabled for
//! the 4.0 compile-fix pass; revisit when wiring up the test harness.
//! Relocation for re-enabling: `tern::executor` -> `tern::exec`.
// use tern::TernApp;
// use tern::exec::sqlx::SqlxPgExecutor;
//
// #[derive(TernApp)]
// #[tern(
// source = "tests/migrations/migrations01",
// table = "_test_derive_history"
// )]
// pub struct TestMigrate01 {
// #[tern(executor_via)]
// exec: SqlxPgExecutor,
// }
