//! Supporting backend operations for `tern`.
use tern_core::migration::{HistoryTable, MigrationData};

pub(crate) mod mysql;
pub(crate) mod postgres;
pub(crate) mod sqlite;

/// Internal helper trait collecting the queries to impl stuff.
#[allow(dead_code)]
pub(crate) trait ExecutorBackend {
    /// Query for `MigrationExecutor::check_history`.
    fn check_history(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::init`.
    fn init_history_query(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::drop_history`.
    fn drop_history_query(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::get_all_applied`.
    fn get_all_applied_query(history: HistoryTable) -> String;

    /// Query for `MigrationExecutor::insert_applied`.
    fn insert_applied_query(
        history: HistoryTable,
        data: &MigrationData,
    ) -> String;

    /// Query for `MigrationExecutor::delete_applied`.
    fn delete_applied_query(history: HistoryTable, version: i64) -> String;

    /// Query for `MigrationExecutor::upsert_applied`.
    fn upsert_applied_query(
        history: HistoryTable,
        data: &MigrationData,
    ) -> String;
}
