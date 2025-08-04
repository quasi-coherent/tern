//! [`Executor`] for the generic [`sqlx::Pool`][sqlx-pool], a pool of `sqlx`
//! database connections.
//!
//! [`Executor`]: crate::context::Executor
//! [sqlx-pool]: https://docs.rs/sqlx/0.8.3/sqlx/struct.Pool.html
use crate::context::Executor as MigrationExecutor;
use crate::error::{DatabaseError as _, TernResult};
use crate::source::{AppliedMigration, Query, QueryRepository};

use chrono::{DateTime, Utc};
use sqlx::pool::PoolOptions;
use sqlx::{Acquire, Connection, Database, Encode, Executor, FromRow, IntoArguments, Pool, Type};
use std::marker::PhantomData;

/// The generic `sqlx::Pool` as a migration executor backend.
pub struct SqlxExecutor<Db, Q>
where
    Db: Database,
    Q: QueryRepository,
{
    pool: Pool<Db>,
    _q: PhantomData<Q>,
}

impl<Db, Q> SqlxExecutor<Db, Q>
where
    Db: Database,
    Q: QueryRepository,
{
    /// Create a pool with default options from a connection string.
    pub async fn new(db_url: &str) -> TernResult<Self> {
        let pool = Pool::connect(db_url).await.tern_result()?;

        Ok(Self {
            pool,
            _q: PhantomData,
        })
    }

    /// Create the pool from the given options.
    pub async fn new_with(
        pool_opts: PoolOptions<Db>,
        conn_opts: <Db::Connection as Connection>::Options,
    ) -> TernResult<Self> {
        let pool = pool_opts.connect_with(conn_opts).await.tern_result()?;

        Ok(Self {
            pool,
            _q: PhantomData,
        })
    }

    /// Obtain a reference to the underlying `Pool`.
    pub fn pool(&self) -> &Pool<Db> {
        &self.pool
    }

    /// Obtain a mutable reference to the underlying `Pool`.
    pub fn pool_mut(&mut self) -> &mut Pool<Db> {
        &mut self.pool
    }
}

/// `SqlxExecutor` can be an [`Executor`] fairly straightforwardly when enough
/// bounds involving `Db: sqlx::Database` are added to make it compile.
///
/// [`Executor`]: crate::migration::Executor
impl<Db, Q> MigrationExecutor for SqlxExecutor<Db, Q>
where
    Self: Send + Sync + 'static,
    Q: QueryRepository,
    Db: Database,
    for<'c> &'c mut <Db as Database>::Connection: Executor<'c, Database = Db>,
    for<'q> <Db as Database>::Arguments<'q>: IntoArguments<'q, Db>,
    for<'r> AppliedMigration: FromRow<'r, <Db as Database>::Row>,
    String: Type<Db> + for<'a> Encode<'a, Db>,
    i64: Type<Db> + for<'a> Encode<'a, Db>,
    DateTime<Utc>: Type<Db> + for<'a> Encode<'a, Db>,
{
    type Queries = Q;

    async fn apply_tx(&mut self, query: &Query) -> TernResult<()> {
        let mut tx = self.pool.begin().await.tern_result()?;
        let conn = tx.acquire().await.tern_result()?;
        conn.execute(sqlx::raw_sql(query.sql()))
            .await
            .void_tern_result()?;
        tx.commit().await.void_tern_result()?;

        Ok(())
    }

    async fn apply_no_tx(&mut self, query: &Query) -> TernResult<()> {
        let statements = query.split_statements()?;
        for statement in statements.iter() {
            self.pool
                .execute(sqlx::raw_sql(statement.as_ref()))
                .await
                .void_tern_result()?;
        }

        Ok(())
    }

    async fn create_history_if_not_exists(&mut self, history_table: &str) -> TernResult<()> {
        let query = Q::create_history_if_not_exists_query(history_table);
        self.pool
            .execute(sqlx::raw_sql(query.sql()))
            .await
            .void_tern_result()
    }

    async fn drop_history(&mut self, history_table: &str) -> TernResult<()> {
        let query = Q::drop_history_query(history_table);
        self.pool
            .execute(sqlx::raw_sql(query.sql()))
            .await
            .void_tern_result()
    }

    async fn get_all_applied(&mut self, history_table: &str) -> TernResult<Vec<AppliedMigration>> {
        let query = Q::select_star_from_history_query(history_table);
        let applied = sqlx::query_as::<Db, AppliedMigration>(query.sql())
            .fetch_all(&self.pool)
            .await
            .tern_result()?;

        Ok(applied)
    }

    /// This expects [`insert_into_history_query`] to have placeholders for
    /// `bind`ing the fields of the `AppliedMigration`, and that they appear in
    /// the same order as they do in the [`AppliedMigration`] struct.
    ///
    /// [`insert_into_history_query`]: crate::migration::QueryRepository::insert_into_history_query
    /// [`AppliedMigration`]: crate::migration::AppliedMigration
    async fn insert_applied_migration(
        &mut self,
        history_table: &str,
        applied: &AppliedMigration,
    ) -> TernResult<()> {
        let query = Q::insert_into_history_query(history_table, applied);
        sqlx::query::<Db>(query.sql())
            .bind(applied.version)
            .bind(applied.description.clone())
            .bind(applied.content.clone())
            .bind(applied.duration_ms)
            .bind(applied.applied_at)
            .execute(&self.pool)
            .await
            .void_tern_result()?;

        Ok(())
    }

    /// Like [`insert_applied_migration`] this expects a query with placeholders
    /// lining up with the order of [`AppliedMigration`] fields.
    ///
    /// [`insert_applied_migration`]: Self::insert_applied_migration
    /// [`AppliedMigration`]: crate::migration::AppliedMigration
    async fn upsert_applied_migration(
        &mut self,
        history_table: &str,
        applied: &AppliedMigration,
    ) -> TernResult<()> {
        let query = Q::upsert_history_query(history_table, applied);
        sqlx::query::<Db>(query.sql())
            .bind(applied.version)
            .bind(applied.description.clone())
            .bind(applied.content.clone())
            .bind(applied.duration_ms)
            .bind(applied.applied_at)
            .execute(&self.pool)
            .await
            .void_tern_result()?;

        Ok(())
    }
}
