use derrick::prelude::*;
use derrick::reexport::BoxFuture;
use derrick::sqlx_postgres::{SqlxPgHistoryTable, SqlxPgMigrate};
use derrick::{forward_migrate_to_field, Runtime};

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
    // The schema and table where the migration
    // history lives or should be created for the
    // first time.
    type History = SqlxPgHistoryTable;

    // The `GetEnvVar` type doesn't need
    // anything to initialize, so we need nothing
    // additional to initialize `ExampleMigrate`.
    //
    // This would be data needed to create other
    // things if what had the `Runtime` derive
    // macro were more complicated.
    type Init = ();

    fn initialize(
        db_url: String,
        history: Self::History,
        data: Self::Init,
    ) -> BoxFuture<'static, Result<Self, derrick::Error>> {
        Box::pin(async move {
            let migrate = <SqlxPgMigrate as Migrate>::initialize(db_url, history, data).await?;
            Ok(Self {
                migrate,
                env: GetEnvVar,
            })
        })
    }

    // Deferring the implementation of all methods to the
    // `SqlxPgMigrate` field, which implements `Migrate`
    // over in `derrick-migrate`.
    forward_migrate_to_field!(SqlxPgMigrate, migrate);
}
