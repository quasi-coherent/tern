use chrono::{DateTime, Utc};
use sqlx::pool::PoolOptions;
use sqlx::{Acquire as _, Database, Executor as _};
use tern_core::error::{TernError, TernResult};
use tern_core::executor::{Executor, HistoryTable};
use tern_core::migration::Applied;
use tern_core::query::{Query, Statement};

use crate::impls::sqlx::SqlxError;
use crate::query::ExecQueryLib;

/// An `Executor` for the generic [`sqlx::Pool`].
#[derive(Debug)]
pub struct SqlxExecutor<Db: Database>(sqlx::Pool<Db>);

impl<Db: Database> SqlxExecutor<Db> {
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

    /// Helper for the `Tx` variant of `Query`.
    async fn send_tx(
        conn: &mut <Db as Database>::Connection,
        statement: &Statement,
    ) -> TernResult<()>
    where
        for<'c> &'c mut <Db as Database>::Connection:
            sqlx::Executor<'c, Database = Db>,
    {
        log::trace!("running {statement}");
        conn.execute(sqlx::raw_sql(statement))
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    /// Helper for the `Seq` variant of `Query`.
    async fn send_seq(&self, statements: &[Statement]) -> TernResult<()>
    where
        for<'c> &'c mut <Db as Database>::Connection:
            sqlx::Executor<'c, Database = Db>,
    {
        let num = statements.len();
        for (idx, st) in statements.iter().enumerate() {
            log::trace!("running statement {} of {}: {st}", idx + 1, num);
            self.inner()
                .execute(sqlx::raw_sql(st))
                .await
                .map_err(|e| SqlxError::err_idx(e, idx))?;
        }
        Ok(())
    }
}

impl<Db> Executor for SqlxExecutor<Db>
where
    Db: Database + ExecQueryLib,
    for<'c> &'c mut <Db as Database>::Connection:
        sqlx::Executor<'c, Database = Db>,
    for<'q> <Db as Database>::Arguments<'q>: sqlx::IntoArguments<'q, Db>,
    for<'r> Applied: sqlx::FromRow<'r, <Db as Database>::Row>,
    String: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    i64: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    DateTime<Utc>: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    for<'r> (bool,): sqlx::FromRow<'r, <Db as Database>::Row>,
{
    async fn apply(&mut self, query: &Query) -> TernResult<()> {
        match query {
            Query::Tx(statement) => {
                let mut tx = self.0.begin().await.map_err(SqlxError::from)?;
                let conn = tx.acquire().await.map_err(SqlxError::from)?;
                Self::send_tx(conn, statement).await?;
                tx.commit().await.map_err(SqlxError::from)?;
            },
            Query::Seq(statements) => {
                self.send_seq(statements).await?;
            },
        }
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

    async fn create_history_if_not_exists(
        &mut self,
        history: HistoryTable,
    ) -> TernResult<()> {
        let sql = Db::create_history_if_not_exists_query(history);
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

    async fn get_all_applied(
        &mut self,
        history: HistoryTable,
    ) -> TernResult<Vec<Applied>> {
        let sql = Db::get_all_applied_query(history);
        log::trace!("running {sql}");
        let applied = sqlx::query_as::<Db, Applied>(&sql)
            .fetch_all(self.inner())
            .await
            .map_err(SqlxError::from)?;
        Ok(applied)
    }

    async fn insert_applied(
        &mut self,
        history: HistoryTable,
        applied: &Applied,
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
        applied: &Applied,
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

impl<Db: Database> Clone for SqlxExecutor<Db> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
