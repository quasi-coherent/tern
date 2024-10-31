pub use derrick_core::error::Error;
pub use derrick_core::prelude::{Migrate, QueryBuilder};
pub use derrick_core::reexport::BoxFuture;
pub use derrick_core::types::{AppliedMigration, Migration, MigrationQuery, MigrationSource};
pub use derrick_migrate::{MigrationRuntime, Runner, RunnerArgs};

pub use derrick_derive::{QueryBuilder, Runtime};

/// Helper to implement [`Migrate`](self::Migrate) if
/// a struct field implements it.  Provides all methods
/// except `initialize` and the associated type.  Use as in
/// `forward_migrate_to!(ForwardToType, forward_to_field)`.
///
#[macro_export]
macro_rules! forward_migrate_to_field {
    ($from:ty, $field:ident) => {
        fn check_history_table(&mut self) -> BoxFuture<'_, Result<(), Error>> {
            <$from as Migrate>::check_history_table(&mut self.$field)
        }
        fn get_history_rows(&mut self) -> BoxFuture<'_, Result<Vec<HistoryRow>, Error>> {
            <$from as Migrate>::get_history_rows(&mut self.$field)
        }
        fn insert_new_applied<'a, 'c: 'a>(
            &'c mut self,
            applied: &'a AppliedMigration,
        ) -> BoxFuture<'a, Result<(), Error>> {
            <$from as Migrate>::insert_new_applied(&mut self.$field, applied)
        }
        fn apply_no_tx<'a, 'c: 'a>(
            &'c mut self,
            migration: &'a Migration,
        ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
            <$from as Migrate>::apply_no_tx(&mut self.$field, migration)
        }
        fn apply_tx<'a, 'c: 'a>(
            &'c mut self,
            migration: &'a Migration,
        ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
            <$from as Migrate>::apply_tx(&mut self.$field, migration)
        }
    };
}
