use derrick::prelude::*;
use derrick::reexport::BoxFuture;
use derrick::sqlx_postgres::{SqlxPgHistoryTable, SqlxPgMigrate};
use derrick::types::{AppliedMigration, Migration};
use derrick::Error;

pub mod migrations;

/// Migration runtime having the DB connection
/// and in addition a way to get environment
/// variables while building the migration query.
pub struct ExampleMigrate {
    pub migrate: SqlxPgMigrate,
    pub env: GetEnvVar,
}

pub struct GetEnvVar;

impl GetEnvVar {
    pub fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

impl Migrate for ExampleMigrate {
    type Table = SqlxPgHistoryTable;
    type Conn = SqlxPgMigrate;

    fn acquire(&mut self) -> &mut SqlxPgMigrate {
        &mut self.migrate
    }

    fn apply_no_tx<'a, 'c: 'a>(
        &'c mut self,
        table_name: &'a Self::Table,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        let conn = self.acquire();
        conn.apply_no_tx(table_name, migration)
    }

    fn apply_tx<'a, 'c: 'a>(
        &'c mut self,
        table_name: &'a Self::Table,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        let conn = self.acquire();
        conn.apply_tx(table_name, migration)
    }
}
