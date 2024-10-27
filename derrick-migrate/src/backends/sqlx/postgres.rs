use derrick_core::error::{DatabaseError, Error};
use derrick_core::prelude::*;
use derrick_core::reexport::BoxFuture;
use derrick_core::types::{
    AppliedMigration, HistoryRow, HistoryTableInfo, Migration, MigrationSource,
};
use log::{debug, info};
use sqlx::{postgres, Acquire, PgPool, Postgres};
use std::{
    ops::{Deref, DerefMut},
    time::Instant,
};

use crate::migrate::pg::PgHistoryTableInfo;
use crate::migrate::validate::Validate;

/// A `Migrate` for `sqlx::PgPool`.
#[derive(Clone)]
pub struct SqlxPgMigrate(PgPool);

/// The `{schema}.{name}` of the history table.
#[derive(Debug, Clone)]
pub struct SqlxPgHistoryTable(String);

impl SqlxPgHistoryTable {
    fn table(&self) -> String {
        self.0.clone()
    }
}

impl SqlxPgMigrate {
    fn pool(&self) -> &PgPool {
        &self.0
    }
}

impl Migrate for SqlxPgMigrate {
    type Table = SqlxPgHistoryTable;
    type Conn = SqlxPgMigrate;

    fn acquire(&mut self) -> &mut Self::Conn {
        self
    }

    fn validate_source(
        source: Vec<MigrationSource>,
        applied: Vec<AppliedMigration>,
    ) -> Result<(), Error> {
        Validate::run_validation(source, applied)
    }

    fn apply_no_tx<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        info!(
            version:% = migration.version,
            sql:% = migration.sql,
            no_tx:% = migration.no_tx;
            "applying migration version"
        );
        Box::pin(async move {
            let sql = &migration.sql;
            let now = Instant::now();

            sqlx::query(sql)
                .execute(self.pool())
                .await
                .into_error_with(migration)?;
            let duration_sec = now.elapsed().as_secs() as i64;
            let applied = migration.new_applied(duration_sec);

            info!(
                version:% = migration.version,
                table:? = table;
                "inserting migration into history table"
            );
            self.insert_new_applied(table, &applied)
                .await
                .into_error_void()?;

            Ok(applied)
        })
    }

    fn apply_tx<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::Table,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        info!(
            version:% = migration.version,
            sql:% = migration.sql,
            no_tx:% = migration.no_tx;
            "applying migration version"
        );
        Box::pin(async move {
            let sql = &migration.sql;
            let mut tx = self.pool().begin().await.into_error()?;
            let conn = tx.acquire().await.into_error()?;

            let now = Instant::now();
            sqlx::query(sql)
                .execute(&mut *conn)
                .await
                .into_error_with(migration)?;
            let duration_sec = now.elapsed().as_secs() as i64;

            let applied = migration.new_applied(duration_sec);
            let sql = table.insert_into_query(&applied);

            info!(
                version:% = migration.version,
                table:? = table;
                "inserting migration into history table"
            );
            sqlx::query(&sql)
                .bind(applied.version)
                .bind(applied.description.clone())
                .bind(applied.content.clone())
                .bind(applied.duration_sec)
                .execute(&mut *conn)
                .await
                .into_error_void()?;

            tx.commit().await.into_error()?;

            Ok(applied)
        })
    }
}

impl MigrateConn for SqlxPgMigrate {
    // Connection string
    type ConnInfo = String;
    type ConnTable = SqlxPgHistoryTable;

    fn connect(info: Self::ConnInfo) -> BoxFuture<'static, Result<Self, Error>> {
        Box::pin(async move {
            let opts = info.parse::<postgres::PgConnectOptions>().into_error()?;
            let pool = postgres::PgPoolOptions::new()
                .connect_with(opts)
                .await
                .into_error()?;

            Ok(SqlxPgMigrate(pool))
        })
    }

    fn create_if_not_exists(
        &mut self,
        table: &Self::ConnTable,
    ) -> BoxFuture<'_, Result<(), Error>> {
        let sql = table.create_if_not_exists_query();
        debug!(query:% = sql; "running `create table if exists` query");
        Box::pin(async move {
            sqlx::query(&sql)
                .execute(self.pool())
                .await
                .into_error_void()
        })
    }

    fn select_star_from<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::ConnTable,
    ) -> BoxFuture<'a, Result<Vec<HistoryRow>, Error>> {
        let sql = table.select_star_from_query();
        debug!(query:% = sql; "running select query");
        Box::pin(async move {
            let rows = sqlx::query_as::<Postgres, HistoryRow>(&sql)
                .fetch_all(self.pool())
                .await
                .into_error()?;

            Ok(rows)
        })
    }

    fn insert_into<'a, 'c: 'a>(
        &'c mut self,
        table: &'a Self::ConnTable,
        applied: &'a AppliedMigration,
    ) -> BoxFuture<'a, Result<(), Error>> {
        let sql = table.insert_into_query(applied);
        debug!(query:% = sql; "running insert query");
        Box::pin(async move {
            sqlx::query(&sql)
                .bind(applied.version)
                .bind(applied.description.clone())
                .bind(applied.content.clone())
                .bind(applied.duration_sec)
                .execute(self.pool())
                .await
                .into_error()?;

            Ok(())
        })
    }
}

impl HistoryTable for SqlxPgHistoryTable {
    fn new(info: &HistoryTableInfo) -> Self {
        let table_name = info
            .table_name()
            .unwrap_or("_derrick_migrations".to_string());
        let table = match info.schema() {
            Some(schema) => format!("{schema}.{table_name}"),
            // unqualified goes to a default location
            _ => table_name,
        };

        Self(table)
    }

    fn table(&self) -> String {
        self.table()
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

impl Deref for SqlxPgMigrate {
    type Target = PgPool;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SqlxPgMigrate {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
