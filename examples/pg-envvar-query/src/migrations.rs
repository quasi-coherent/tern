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
    type History = SqlxPgHistoryTable;
    // The `GetEnvVar` type doesn't need
    // anything to initialize, so we need nothing
    // additional to initialize `ExampleMigrate`.
    //
    // This would be data needed to create other
    // other things if what had the `Runtime` derive
    // macro were more complicated.
    type Init = ();

    fn initialize(
        db_url: String,
        history: Self::History,
        data: Self::Init,
    ) -> BoxFuture<'static, Result<Self, Error>> {
        Box::pin(async move {
            let migrate = <SqlxPgMigrate as Migrate>::initialize(db_url, history, data).await?;
            Ok(Self {
                migrate,
                env: GetEnvVar,
            })
        })
    }

    forward_migrate_to_field!(SqlxPgMigrate, migrate);
}
