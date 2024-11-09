pub mod error;
mod migrations;

pub mod prelude {
    pub use super::migrations::history::HistoryTable;
    pub use super::migrations::migrate::Migrate;
    pub use super::migrations::source::QueryBuilder;
}

pub mod types {
    pub use super::migrations::history::{ExistingMigration, HistoryTableOptions};
    pub use super::migrations::migrate::NoValidation;
    pub use super::migrations::migration::{AppliedMigration, Migration};
    pub use super::migrations::source::{MigrationQuery, MigrationSource};
}

pub mod reexport {
    pub use futures_core::future::BoxFuture;
}
