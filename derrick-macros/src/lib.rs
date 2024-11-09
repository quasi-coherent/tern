pub use derrick_core::error::Error;
pub use derrick_core::prelude::{Migrate, QueryBuilder};
pub use derrick_core::reexport::BoxFuture;
pub use derrick_core::types::{AppliedMigration, Migration, MigrationQuery, MigrationSource};
pub use derrick_migrate::Runner;

pub use derrick_derive::{QueryBuilder, Runtime};

/// Helper to implement [`Migrate`](self::Migrate) if
/// a struct field implements it.  Provides all methods
/// except `initialize` and the associated type.  Use as in
/// `forward_migrate_to!(ForwardToType, forward_to_field)`.
#[macro_export]
macro_rules! forward_migrate_to_field {
    ($from:ty, $field:ident) => {
        fn check_history_table(
            &mut self,
        ) -> derrick::reexport::BoxFuture<'_, Result<(), derrick::Error>> {
            <$from as derrick::prelude::Migrate>::check_history_table(&mut self.$field)
        }
        fn get_history_table(
            &mut self,
        ) -> derrick::reexport::BoxFuture<
            '_,
            Result<Vec<derrick::types::ExistingMigration>, derrick::Error>,
        > {
            <$from as derrick::prelude::Migrate>::get_history_table(&mut self.$field)
        }
        fn insert_new_applied<'a, 'c: 'a>(
            &'c mut self,
            applied: &'a derrick::types::AppliedMigration,
        ) -> derrick::reexport::BoxFuture<'a, Result<(), derrick::Error>> {
            <$from as derrick::prelude::Migrate>::insert_new_applied(&mut self.$field, applied)
        }
        fn apply_no_tx<'a, 'c: 'a>(
            &'c mut self,
            migration: &'a derrick::types::Migration,
        ) -> derrick::reexport::BoxFuture<
            'a,
            Result<derrick::types::AppliedMigration, derrick::Error>,
        > {
            <$from as Migrate>::apply_no_tx(&mut self.$field, migration)
        }
        fn apply_tx<'a, 'c: 'a>(
            &'c mut self,
            migration: &'a derrick::types::Migration,
        ) -> derrick::reexport::BoxFuture<
            'a,
            Result<derrick::types::AppliedMigration, derrick::Error>,
        > {
            <$from as derrick::prelude::Migrate>::apply_tx(&mut self.$field, migration)
        }
    };
}
