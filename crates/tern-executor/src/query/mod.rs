use tern_core::executor::HistoryTable;
use tern_core::migration::Applied;

pub(crate) mod mysql;
pub(crate) mod postgres;
pub(crate) mod sqlite;

/// Internal helper trait collecting the queries for an `Executor` impl.
#[allow(unused)]
pub trait ExecQueryLib {
    /// Query for `Executor::check_history`.
    fn check_history(history: HistoryTable) -> String;

    /// Query for `Executor::create_history_if_not_exists`.
    fn create_history_if_not_exists_query(history: HistoryTable) -> String;

    /// Query for `Executor::drop_history`.
    fn drop_history_query(history: HistoryTable) -> String;

    /// Query for `Executor::get_all_applied`.
    fn get_all_applied_query(history: HistoryTable) -> String;

    /// Query for `Executor::insert_applied`.
    fn insert_applied_query(history: HistoryTable, applied: &Applied)
    -> String;

    /// Query for `Executor::delete_applied`.
    fn delete_applied_query(history: HistoryTable, version: i64) -> String;

    /// Query for `Executor::upsert_applied`.
    fn upsert_applied_query(history: HistoryTable, applied: &Applied)
    -> String;
}
