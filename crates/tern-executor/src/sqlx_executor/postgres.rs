use sqlx::postgres::Postgres;
use tern_core::migration::{HistoryTable, MigrationData};

use crate::backend::ExecutorBackend;
use crate::backend::postgres::PgBackend;
use crate::sqlx_executor::{SqlxAnyExecutor, SqlxAnyExecutorOptions};

pub use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};

/// `MigrationExecutor` for a PostgreSQL backend.
pub type SqlxPgExecutor = SqlxAnyExecutor<Postgres>;

/// `ExecutorOptions` for a PostgreSQL backend.
pub type SqlxPgExecutorOptions = SqlxAnyExecutorOptions<Postgres>;

impl ExecutorBackend for Postgres {
    fn check_history(history: HistoryTable) -> String {
        PgBackend::check_history(history)
    }

    fn init_history_query(history: HistoryTable) -> String {
        PgBackend::init_history_query(history)
    }

    fn drop_history_query(history: HistoryTable) -> String {
        PgBackend::drop_history_query(history)
    }

    fn get_all_applied_query(history: HistoryTable) -> String {
        PgBackend::get_all_applied_query(history)
    }

    fn insert_applied_query(
        history: HistoryTable,
        _: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ($1, $2, $3, $4, $5);
"
        )
    }

    fn delete_applied_query(history: HistoryTable, _: i64) -> String {
        format!(
            "
DELETE FROM {history}
WHERE version = $1;
"
        )
    }

    fn upsert_applied_query(
        history: HistoryTable,
        _: &MigrationData,
    ) -> String {
        format!(
            "
INSERT INTO {history}(version, description, content, duration_ms, applied_at)
  VALUES ($1, $2, $3, $4, $5)
  ON CONFLICT (version) DO UPDATE
  SET
    description = excluded.description,
    content = excluded.content,
    duration_ms = excluded.duration_ms,
    applied_at = excluded.applied_at;
"
        )
    }
}
