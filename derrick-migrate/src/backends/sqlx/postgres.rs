use derrick_core::error::{DatabaseError, Error};
use derrick_core::prelude::*;
use derrick_core::reexport::BoxFuture;
use derrick_core::types::{
    AppliedMigration, HistoryRow, HistoryTableInfo, Migration, MigrationSource,
};
use log::{debug, info};
use sqlx::{postgres, Acquire, Executor, PgPool, Postgres};
use std::time::Instant;

use crate::migrate::pg::PgHistoryTableInfo;
use crate::migrate::validate::Validate;

/// A `Migrate` for `sqlx::PgPool`.
#[derive(Clone)]
pub struct SqlxPgMigrate {
    pool: PgPool,
    history_table: SqlxPgHistoryTable,
}

/// Additional options to create the `Migrate`.
/// This is minimal in that it only has the history
/// table.
#[derive(Debug, Clone)]
pub struct SqlxPgHistoryTable {
    schema: Option<String>,
    table_name: String,
}

impl HistoryTable for SqlxPgHistoryTable {
    fn new(info: &HistoryTableInfo) -> Self {
        Self::new(info.schema(), info.table_name())
    }

    fn table(&self) -> String {
        let table_name = self.table_name();
        match self.schema() {
            Some(schema) => format!("{schema}.{table_name}"),
            // unqualified goes to a default location
            _ => table_name,
        }
    }

    fn create_if_not_exists_query(&self) -> String {
        let pg_tbl = PgHistoryTableInfo::new(self.table());
        pg_tbl.create_if_not_exists_query()
    }

    fn select_star_from_query(&self) -> String {
        let pg_tbl = PgHistoryTableInfo::new(self.table());
        pg_tbl.select_star_from_query()
    }

    fn insert_into_query(&self, _: &AppliedMigration) -> String {
        let sql = format!(
            "
INSERT INTO {}(version, description, content, duration_sec)
  VALUES ($1, $2, $3, $4);",
            self.table(),
        );

        sql
    }
}

impl Migrate for SqlxPgMigrate {
    type History = SqlxPgHistoryTable;
    // We don't need anything more to initialize.
    type Init = ();

    fn initialize(
        db_url: String,
        history: Self::History,
        _: Self::Init,
    ) -> BoxFuture<'static, Result<Self, Error>> {
        Box::pin(async move {
            let opts = db_url.parse::<postgres::PgConnectOptions>().into_error()?;
            let pool = postgres::PgPoolOptions::new()
                .connect_with(opts)
                .await
                .into_error()?;
            Ok(SqlxPgMigrate::new(pool, history))
        })
    }

    fn check_history_table(&mut self) -> BoxFuture<'_, Result<(), Error>> {
        let history = self.history_table();
        let sql = history.create_if_not_exists_query().clone();

        Box::pin(async move {
            debug!("running `create table if exists` query");
            sqlx::query(&sql)
                .execute(self.pool())
                .await
                .into_error_void()
        })
    }

    fn get_history_rows(&mut self) -> BoxFuture<'_, Result<Vec<HistoryRow>, Error>> {
        Box::pin(async move {
            let history = self.history_table();
            let sql = history.select_star_from_query();

            debug!("running select query");
            let rows = sqlx::query_as::<Postgres, HistoryRow>(&sql)
                .fetch_all(self.pool())
                .await
                .into_error()?;

            Ok(rows)
        })
    }

    fn insert_new_applied<'a, 'c: 'a>(
        &'c mut self,
        applied: &'a AppliedMigration,
    ) -> BoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let history = self.history_table();
            let sql = history.insert_into_query(applied);

            debug!("running insert query");
            sqlx::query(&sql)
                .bind(applied.version)
                .bind(applied.description.clone())
                .bind(applied.content.clone())
                .bind(applied.duration_ms)
                .execute(self.pool())
                .await
                .into_error()?;

            Ok(())
        })
    }

    fn apply_no_tx<'a, 'c: 'a>(
        &'c mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        Box::pin(async move {
            let sql = &migration.sql;
            let now = Instant::now();

            info!("applying migration {}...", migration.version);
            // We have to use `sqlx::raw_sql` because the `query_*`
            // functions use prepared statements, and a migration with
            // more than one query cannot be sent as a prepared statement.
            self.pool()
                .execute(sqlx::raw_sql(&sql))
                .await
                .into_error_with(migration)?;
            let duration_ms = now.elapsed().as_millis() as i64;
            let applied = migration.new_applied(duration_ms);

            info!("migration {} applied", migration.version);
            self.insert_new_applied(&applied).await.into_error_void()?;

            Ok(applied)
        })
    }

    fn apply_tx<'a, 'c: 'a>(
        &'c mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        Box::pin(async move {
            let sql = migration.sql.to_string();
            let mut tx = self.pool().begin().await.into_error()?;
            let conn = tx.acquire().await.into_error()?;

            let now = Instant::now();

            info!("applying migration {}...", migration.version);
            // We have to use `sqlx::raw_sql` because the `query_*`
            // functions use prepared statements, and a migration with
            // more than one query cannot be sent as a prepared statement.
            conn.execute(sqlx::raw_sql(&sql))
                .await
                .into_error_with(migration)?;
            let duration_ms = now.elapsed().as_millis() as i64;

            let applied = migration.new_applied(duration_ms);
            let history = self.history_table();
            let insert_sql = history.insert_into_query(&applied).clone();

            info!("migration {} applied", migration.version);
            sqlx::query(&insert_sql)
                .bind(applied.version)
                .bind(applied.description.clone())
                .bind(applied.content.clone())
                .bind(applied.duration_ms)
                .execute(&mut *conn)
                .await
                .into_error_void()?;

            tx.commit().await.into_error()?;

            Ok(applied)
        })
    }

    fn validate_source(
        source: Vec<MigrationSource>,
        applied: Vec<AppliedMigration>,
    ) -> Result<(), Error> {
        Validate::run_validation(source, applied)
    }
}

impl SqlxPgMigrate {
    pub fn new(pool: PgPool, history_table: SqlxPgHistoryTable) -> Self {
        Self {
            pool,
            history_table,
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn history_table(&self) -> &SqlxPgHistoryTable {
        &self.history_table
    }
}

impl SqlxPgHistoryTable {
    pub fn new(schema: Option<String>, table_name: String) -> Self {
        Self { schema, table_name }
    }

    pub fn schema(&self) -> Option<String> {
        self.schema.clone()
    }

    pub fn table_name(&self) -> String {
        self.table_name.clone()
    }
}
