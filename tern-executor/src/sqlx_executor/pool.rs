use chrono::{DateTime, Utc};
use sqlx::pool::PoolOptions;
use sqlx::{Acquire as _, Database, Executor as _};
use tern_core::error::{TernError, TernResult};
use tern_core::executor::{HistoryTable, MigrationExecutor};
use tern_core::migration::AppliedMigration;
use tern_core::query::{Query, Statement, Statements};

use crate::sqlx_pool::SqlxQueryLib;

/// A [`MigrationExecutor`](tern_core::executor::MigrationExecutor) for
/// [`sqlx::Pool`] over a generic [`Database`](sqlx::Database).
pub struct SqlxExecutor<Db: Database>(sqlx::Pool<Db>);

impl<Db: Database> SqlxExecutor<Db> {
    /// New `SqlxExecutor` from a simple connection string.
    pub async fn new(db_url: &str) -> TernResult<Self> {
        let pool = sqlx::Pool::connect(db_url).await?;
        Ok(Self(pool))
    }

    /// New `SqlxExecutor` from more general options.
    pub async fn from_options(
        pool_opts: PoolOptions<Db>,
        conn_opts: <Db::Connection as sqlx::Connection>::Options,
    ) -> TernResult<Self> {
        let pool = pool_opts.connect_with(conn_opts).await?;
        Ok(Self(pool))
    }

    /// Return the underlying connection pool for custom operations.
    pub fn inner(&self) -> &sqlx::Pool<Db> {
        &self.0
    }

    /// Helper for the `Statement` variant of `Query`.
    async fn send_statement(
        conn: &mut <Db as Database>::Connection,
        statement: &Statement,
    ) -> Result<(), sqlx::Error>
    where
        for<'c> &'c mut <Db as Database>::Connection: sqlx::Executor<'c, Database = Db>,
    {
        conn.execute(sqlx::raw_sql(statement)).await?;
        Ok(())
    }

    /// Helper for the `Statements` variant of `Query`.
    async fn send_statements(&self, statements: &Statements) -> Result<(), sqlx::Error>
    where
        for<'c> &'c mut <Db as Database>::Connection: sqlx::Executor<'c, Database = Db>,
    {
        for statement in statements.iter() {
            self.inner().execute(sqlx::raw_sql(statement)).await?;
        }
        Ok(())
    }
}

impl<Db> MigrationExecutor for SqlxExecutor<Db>
where
    Db: Database + SqlxQueryLib,
    for<'c> &'c mut <Db as Database>::Connection: sqlx::Executor<'c, Database = Db>,
    for<'q> <Db as Database>::Arguments<'q>: sqlx::IntoArguments<'q, Db>,
    for<'r> AppliedMigration: sqlx::FromRow<'r, <Db as Database>::Row>,
    String: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    i64: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    DateTime<Utc>: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    for<'r> (bool,): sqlx::FromRow<'r, <Db as Database>::Row>,
{
    async fn apply(&mut self, query: &Query) -> TernResult<()> {
        match query {
            Query::One(statement) => {
                let mut tx = self.0.begin().await?;
                let conn = tx.acquire().await?;
                Self::send_statement(conn, statement).await?;
                tx.commit().await?;
            }
            Query::Many(statements) => {
                self.send_statements(statements).await?;
            }
        }
        Ok(())
    }

    async fn check_history(&mut self, history: HistoryTable) -> TernResult<()> {
        let sql = Db::check_history(history);
        let exists: bool = sqlx::query_scalar(&sql).fetch_one(self.inner()).await?;
        if exists {
            Ok(())
        } else {
            Err(TernError::History("history table not found"))
        }
    }

    async fn create_history_if_not_exists(&mut self, history: HistoryTable) -> TernResult<()> {
        let sql = Db::create_history_if_not_exists_query(history);
        self.inner().execute(sqlx::raw_sql(&sql)).await?;
        Ok(())
    }

    async fn drop_history(&mut self, history: HistoryTable) -> TernResult<()> {
        let sql = Db::drop_history_query(history);
        self.inner().execute(sqlx::raw_sql(&sql)).await?;
        Ok(())
    }

    async fn get_all_applied(
        &mut self,
        history: HistoryTable,
    ) -> TernResult<Vec<AppliedMigration>> {
        let sql = Db::get_all_applied_query(history);
        let applied = sqlx::query_as::<Db, AppliedMigration>(&sql)
            .fetch_all(self.inner())
            .await?;
        Ok(applied)
    }

    async fn insert_applied(
        &mut self,
        history: HistoryTable,
        applied: &AppliedMigration,
    ) -> TernResult<()> {
        let sql = Db::insert_applied_query(history);
        sqlx::query::<Db>(&sql)
            .bind(applied.version())
            .bind(applied.description())
            .bind(applied.content())
            .bind(applied.duration_millis())
            .bind(applied.applied_at())
            .execute(self.inner())
            .await?;
        Ok(())
    }

    async fn reset_last_applied(&mut self, history: HistoryTable, version: i64) -> TernResult<()> {
        let sql = Db::reset_last_applied_query(history, version);
        sqlx::query::<Db>(&sql)
            .bind(version)
            .execute(self.inner())
            .await?;
        Ok(())
    }

    async fn upsert_applied(
        &mut self,
        history: HistoryTable,
        applied: &AppliedMigration,
    ) -> TernResult<()> {
        let sql = Db::upsert_applied_query(history);
        sqlx::query::<Db>(&sql)
            .bind(applied.version())
            .bind(applied.description())
            .bind(applied.content())
            .bind(applied.duration_millis())
            .bind(applied.applied_at())
            .execute(self.inner())
            .await?;
        Ok(())
    }
}
