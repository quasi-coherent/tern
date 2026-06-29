use chrono::{DateTime, Utc};
use sqlx::pool::PoolOptions;
use sqlx::{Acquire as _, Database, Executor as _};
use tern_core::context::MigrationExecutor;
use tern_core::error::{TernError, TernResult};
use tern_core::migration::{HistoryTable, MigrationData};

use crate::backend::ExecutorBackend;
use crate::sqlx_executor::SqlxError;
use crate::{ConnStr, ExecutorOptions};

/// `ExecutorOptions` for any of the `MigrationExecutor`s coming from `sqlx`.
pub struct SqlxAnyExecutorOptions<Db: Database> {
    pool: PoolOptions<Db>,
    conn: <Db::Connection as sqlx::Connection>::Options,
}

impl<Db: Database> SqlxAnyExecutorOptions<Db> {
    /// New from `sqlx` options.
    pub fn new(
        pool: PoolOptions<Db>,
        conn: <Db::Connection as sqlx::Connection>::Options,
    ) -> Self {
        Self { pool, conn }
    }
}

impl<Db: Database> ExecutorOptions<SqlxAnyExecutor<Db>>
    for SqlxAnyExecutorOptions<Db>
{
    async fn connect(self) -> TernResult<SqlxAnyExecutor<Db>> {
        SqlxAnyExecutor::from_options(self.pool, self.conn).await
    }
}

impl<Db: Database> ExecutorOptions<SqlxAnyExecutor<Db>> for ConnStr {
    async fn connect(self) -> TernResult<SqlxAnyExecutor<Db>> {
        SqlxAnyExecutor::new(&self.0).await
    }
}

/// A `MigrationExecutor` over any `sqlx::Database`.
#[derive(Debug)]
pub struct SqlxAnyExecutor<Db: Database>(sqlx::Pool<Db>);

impl<Db: Database> SqlxAnyExecutor<Db> {
    /// New value from a connection string.
    pub async fn new(db_url: &str) -> TernResult<Self> {
        let pool =
            sqlx::Pool::connect(db_url).await.map_err(SqlxError::from)?;
        Ok(Self(pool))
    }

    /// New from more general options.
    pub async fn from_options(
        pool_opts: PoolOptions<Db>,
        conn_opts: <Db::Connection as sqlx::Connection>::Options,
    ) -> TernResult<Self> {
        let pool =
            pool_opts.connect_with(conn_opts).await.map_err(SqlxError::from)?;
        Ok(Self(pool))
    }

    /// Return the underlying connection pool for custom operations.
    pub fn inner(&self) -> &sqlx::Pool<Db> {
        &self.0
    }
}

impl<Db> MigrationExecutor for SqlxAnyExecutor<Db>
where
    Db: Database + ExecutorBackend,
    for<'c> &'c mut <Db as Database>::Connection:
        sqlx::Executor<'c, Database = Db>,
    for<'q> <Db as Database>::Arguments<'q>: sqlx::IntoArguments<'q, Db>,
    for<'r> MigrationData: sqlx::FromRow<'r, <Db as Database>::Row>,
    String: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    i64: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    DateTime<Utc>: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    for<'r> (bool,): sqlx::FromRow<'r, <Db as Database>::Row>,
{
    async fn send_tx(&mut self, query: &str) -> TernResult<()> {
        async {
            let mut tx = self.0.begin().await?;
            let conn = tx.acquire().await?;
            conn.execute(sqlx::raw_sql(query)).await
        }
        .await
        .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn send_notx(&mut self, query: &str) -> TernResult<()> {
        self.inner()
            .execute(sqlx::raw_sql(query))
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn init_history(&mut self, history: HistoryTable) -> TernResult<()> {
        let sql = Db::init_history_query(history);
        log::trace!("running {sql}");
        self.inner()
            .execute(sqlx::raw_sql(&sql))
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn drop_history(&mut self, history: HistoryTable) -> TernResult<()> {
        let sql = Db::drop_history_query(history);
        log::trace!("running {sql}");
        self.inner()
            .execute(sqlx::raw_sql(&sql))
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn check_history(&mut self, history: HistoryTable) -> TernResult<()> {
        let sql = Db::check_history(history);
        log::trace!("running {sql}");
        let exists: bool = sqlx::query_scalar(&sql)
            .fetch_one(self.inner())
            .await
            .map_err(SqlxError::from)?;
        if exists {
            Ok(())
        } else {
            Err(TernError::History("history table not found"))
        }
    }

    async fn get_all_applied(
        &mut self,
        history: HistoryTable,
    ) -> TernResult<Vec<MigrationData>> {
        let sql = Db::get_all_applied_query(history);
        log::trace!("running {sql}");
        let applied = sqlx::query_as::<Db, MigrationData>(&sql)
            .fetch_all(self.inner())
            .await
            .map_err(SqlxError::from)?;
        Ok(applied)
    }

    async fn insert_applied(
        &mut self,
        history: HistoryTable,
        applied: &MigrationData,
    ) -> TernResult<()> {
        let sql = Db::insert_applied_query(history, applied);
        log::trace!("running {sql}");
        sqlx::query::<Db>(&sql)
            .bind(applied.version())
            .bind(applied.description())
            .bind(applied.content())
            .bind(applied.duration_millis())
            .bind(applied.applied_at())
            .execute(self.inner())
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn delete_applied(
        &mut self,
        history: HistoryTable,
        version: i64,
    ) -> TernResult<()> {
        let sql = Db::delete_applied_query(history, version);
        log::trace!("running {sql}");
        sqlx::query::<Db>(&sql)
            .bind(version)
            .execute(self.inner())
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn upsert_applied(
        &mut self,
        history: HistoryTable,
        applied: &MigrationData,
    ) -> TernResult<()> {
        let sql = Db::upsert_applied_query(history, applied);
        log::trace!("running {sql}");
        sqlx::query::<Db>(&sql)
            .bind(applied.version())
            .bind(applied.description())
            .bind(applied.content())
            .bind(applied.duration_millis())
            .bind(applied.applied_at())
            .execute(self.inner())
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }
}

impl<Db: Database> Clone for SqlxAnyExecutor<Db> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
