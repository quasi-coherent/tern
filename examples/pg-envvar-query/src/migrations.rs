use derrick::prelude::*;
use derrick::reexport::BoxFuture;
use derrick::sqlx_postgres::{SqlxPgHistoryTable, SqlxPgMigrate};
use derrick::types::{AppliedMigration, HistoryRow, Migration};
use derrick::{forward_migrate_to_field, Error, Runtime};

/// Migration runtime having the DB connection
/// and in addition a way to get environment
/// variables while building the migration query.
#[derive(Runtime)]
#[migration(path = "src/migrations/")]
pub struct ExampleMigrate {
    pub migrate: SqlxPgMigrate,
    pub env: GetEnvVar,
}

/// Could have any side effect.
pub struct GetEnvVar;

impl GetEnvVar {
    pub fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

impl Migrate for ExampleMigrate {
    type MigrateInfo = SqlxPgHistoryTable;

    fn initialize(
        db_url: String,
        info: Self::MigrateInfo,
    ) -> BoxFuture<'static, Result<Self, Error>> {
        Box::pin(async move {
            let migrate = <SqlxPgMigrate as Migrate>::initialize(db_url, info).await?;
            Ok(Self {
                migrate,
                env: GetEnvVar,
            })
        })
    }

    forward_migrate_to_field!(SqlxPgMigrate, migrate);
}
