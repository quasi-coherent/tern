use chrono::{DateTime, Utc};
use sqlx::{Acquire, ConnectOptions as _, Database, Executor as _};
use tern_core::context::{ConnStr, HistoryRelid, MigrationExecutor};
use tern_core::error::TernResult;
use tern_core::migration::MigrationData;

use crate::backend::ExecutorBackend;
use crate::sqlx_executor::SqlxError;

/// Options for building `sqlx` executors.
///
/// This is the associated type [`SqlxAnyExecutor::Options`], so is used to
/// build all `sqlx` executor types by specializing `Db: Database`.
#[derive(Clone)]
pub struct SqlxAnyExecutorOptions<Db: Database> {
    options: <Db::Connection as sqlx::Connection>::Options,
}

impl<Db: Database> SqlxAnyExecutorOptions<Db> {
    /// Create new options from a connection string.
    pub fn new(conn: &ConnStr) -> TernResult<Self> {
        let url = conn.try_into_url()?;
        let options =
            <Db::Connection as sqlx::Connection>::Options::from_url(&url)
                .map_err(SqlxError::from)?;
        Ok(Self { options })
    }

    /// New from the `sqlx::Connection`'s associated `Options`.
    pub fn from_options(
        options: <Db::Connection as sqlx::Connection>::Options,
    ) -> Self {
        Self { options }
    }

    /// Return a reference to the connection options.
    pub fn get_options(
        &self,
    ) -> &<Db::Connection as sqlx::Connection>::Options {
        &self.options
    }

    /// Consume this type, returning the inner options.
    pub fn inner(self) -> <Db::Connection as sqlx::Connection>::Options {
        self.options
    }

    /// Return the database URL.
    ///
    /// This does not retain any setting that does not have a representation in
    /// URL format.
    pub fn db_url(&self) -> url::Url {
        self.options.to_url_lossy()
    }
}

/// A `MigrationExecutor` over any `sqlx::Database`.
#[derive(Debug)]
pub struct SqlxAnyExecutor<Db: Database>(Db::Connection);

impl<Db: Database> SqlxAnyExecutor<Db> {
    /// Create new options from a connection string.
    pub async fn new(conn: &ConnStr) -> TernResult<Self> {
        let this = <Db::Connection as sqlx::Connection>::connect(conn)
            .await
            .map(Self)
            .map_err(SqlxError::from)?;
        Ok(this)
    }

    /// New from the `sqlx::Connection`'s associated `Options`.
    pub async fn from_options(
        opts: &<Db::Connection as sqlx::Connection>::Options,
    ) -> TernResult<Self> {
        let this = <Db::Connection as sqlx::Connection>::connect_with(opts)
            .await
            .map(Self)
            .map_err(SqlxError::from)?;
        Ok(this)
    }

    /// Return the underlying connection for custom operations.
    pub fn inner_mut(&mut self) -> &mut Db::Connection {
        &mut self.0
    }
}

impl<Db> MigrationExecutor for SqlxAnyExecutor<Db>
where
    Db: Database + ExecutorBackend,
    Db::Connection: Send + Sync,
    for<'c> &'c mut <Db as Database>::Connection:
        sqlx::Executor<'c, Database = Db> + sqlx::Acquire<'c, Database = Db>,
    for<'q> <Db as Database>::Arguments<'q>: sqlx::IntoArguments<'q, Db>,
    for<'r> MigrationData: sqlx::FromRow<'r, <Db as Database>::Row>,
    String: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    i64: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    DateTime<Utc>: sqlx::Type<Db> + for<'a> sqlx::Encode<'a, Db>,
    for<'r> (bool,): sqlx::FromRow<'r, <Db as Database>::Row>,
{
    async fn connect(conn: &ConnStr) -> TernResult<Self> {
        Self::new(conn).await
    }

    async fn send_tx(&mut self, query: &str) -> TernResult<()> {
        async {
            let mut tx = self.0.begin().await?;
            let conn = tx.acquire().await?;
            conn.execute(sqlx::raw_sql(query)).await?;
            tx.commit().await
        }
        .await
        .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn send_notx(&mut self, query: &str) -> TernResult<()> {
        self.inner_mut()
            .execute(sqlx::raw_sql(query))
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn create_if_not_exists(
        &mut self,
        history: HistoryRelid,
    ) -> TernResult<()> {
        let sql = Db::init_history_query(history);
        log::trace!("running {sql}");
        self.inner_mut()
            .execute(sqlx::raw_sql(&sql))
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn drop_if_exists(
        &mut self,
        history: HistoryRelid,
    ) -> TernResult<()> {
        let sql = Db::drop_history_query(history);
        log::trace!("running {sql}");
        self.inner_mut()
            .execute(sqlx::raw_sql(&sql))
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn history_exists(
        &mut self,
        history: HistoryRelid,
    ) -> TernResult<bool> {
        let sql = Db::check_history(history);
        log::trace!("running {sql}");
        let exists: bool = sqlx::query_scalar(&sql)
            .fetch_one(self.inner_mut())
            .await
            .map_err(SqlxError::from)?;
        Ok(exists)
    }

    async fn select_where_version_between(
        &mut self,
        history: HistoryRelid,
        min_version: Option<i64>,
        max_version: Option<i64>,
    ) -> TernResult<Vec<MigrationData>> {
        let minv = min_version.unwrap_or(i64::MIN);
        let maxv = max_version.unwrap_or(i64::MAX);
        let sql = Db::get_applied_where_query(history, None, None);
        log::trace!("running {sql}");
        let applied = sqlx::query_as::<Db, MigrationData>(&sql)
            .bind(minv)
            .bind(maxv)
            .fetch_all(self.inner_mut())
            .await
            .map_err(SqlxError::from)?;
        Ok(applied)
    }

    async fn insert_into(
        &mut self,
        history: HistoryRelid,
        applied: &MigrationData,
    ) -> TernResult<()> {
        let sql = Db::insert_applied_query(history, applied);
        log::trace!("running {sql}");
        sqlx::query::<Db>(&sql)
            .bind(applied.version())
            .bind(applied.description().to_string())
            .bind(applied.content().to_string())
            .bind(applied.duration_millis())
            .bind(applied.applied_at())
            .execute(self.inner_mut())
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn delete_from(
        &mut self,
        history: HistoryRelid,
        version: i64,
    ) -> TernResult<()> {
        let sql = Db::delete_applied_query(history, version);
        log::trace!("running {sql}");
        sqlx::query::<Db>(&sql)
            .bind(version)
            .execute(self.inner_mut())
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }

    async fn insert_or_update(
        &mut self,
        history: HistoryRelid,
        applied: &MigrationData,
    ) -> TernResult<()> {
        let sql = Db::upsert_applied_query(history, applied);
        log::trace!("running {sql}");
        sqlx::query::<Db>(&sql)
            .bind(applied.version())
            .bind(applied.description().to_string())
            .bind(applied.content().to_string())
            .bind(applied.duration_millis())
            .bind(applied.applied_at())
            .execute(self.inner_mut())
            .await
            .map_err(SqlxError::from)?;
        Ok(())
    }
}
