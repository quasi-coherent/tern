//! Supporting backend operations for `tern`.
use tern_core::context::HistoryRelid;
use tern_core::migration::MigrationData;

pub(crate) mod mysql;
pub(crate) mod postgres;
pub(crate) mod sqlite;

/// Internal helper trait collecting the queries to impl stuff.
#[allow(dead_code)]
pub(crate) trait ExecutorBackend {
    /// Query for `MigrationExecutor::history_exists`.
    fn check_history(history: HistoryRelid) -> String;

    /// Query for `MigrationExecutor::create_if_not_exists`.
    fn init_history_query(history: HistoryRelid) -> String;

    /// Query for `MigrationExecutor::drop_if_exists`.
    fn drop_history_query(history: HistoryRelid) -> String;

    /// Query for `MigrationExecutor::select_where_version_between`.
    fn get_applied_where_query(
        history: HistoryRelid,
        min_version: Option<i64>,
        max_version: Option<i64>,
    ) -> String;

    /// Query for `MigrationExecutor::insert_into`.
    fn insert_applied_query(
        history: HistoryRelid,
        data: &MigrationData,
    ) -> String;

    /// Query for `MigrationExecutor::delete_from`.
    fn delete_applied_query(history: HistoryRelid, version: i64) -> String;

    /// Query for `MigrationExecutor::insert_or_update`.
    fn upsert_applied_query(
        history: HistoryRelid,
        data: &MigrationData,
    ) -> String;
}
