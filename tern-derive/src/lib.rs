use syn::{parse_macro_input, DeriveInput};

mod internal;
mod quote;

/// A Rust migration requires a struct called `TernMigration` which derives
/// `Migration`.  This is used in concert with [`MigrationSource`] to finish the
/// implementation of [`Migration`] for it.
///
/// With the macro attribute `no_transaction`, the `Migration` implementation
/// is constructed to not run the migration in a database transaction.
///
/// ## Usage
///
/// ```rust,no_run
/// use tern::Migration;
///
/// /// Then implement `tern::QueryBuilder` for this type.
/// #[derive(Migration)]
/// #[tern(no_transaction)]
/// pub struct TernMigration;
/// ```
///
/// [`MigrationSource`]: crate::MigrationSource
/// [`Migration`]: https://docs.rs/tern/latest/tern/trait.Migration.html
#[proc_macro_derive(Migration, attributes(tern))]
pub fn migration(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quote::expand_impl_migration(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// `MigrationContext` implements the trait [`MigrationContext`], which is
/// required of a type to be suitable for use in a migration [`Runner`].  A
/// bound of [`MigrationSource`][source-core] exists, which can be satisfied by
/// hand or by using the derive macro provided here, [`MigrationSource`].
/// Custom, dynamic behavior for a migration can be defined for the context,
/// which is available to [`QueryBuilder`].
///
/// The macro exposes one optional macro attribute and one optional field
/// attribute:
///
/// * `table` is the optional macro attribute.  With it enabled, the migration
///   history will be stored in this table, located in the default schema for
///   the database driver, instead of the default table, `_tern_migrations`.
/// * `executor_via` decorates the field holding an [`Executor`], which is
///   required of the type to be a context.  If not specified then it is
///   expected that the type itself implements `Executor`.
///
/// ## Usage
///
/// ```rust,no_run
/// use tern::{SqlxPgExecutor, MigrationContext};
///
/// #[derive(MigrationContext)]
/// #[tern(table = "_my_migration_history")]
/// pub struct MyContext {
///     #[tern(executor_via)]
///     executor: SqlxPgExecutor,
/// }
/// ```
///
/// [`Runner`]: https://docs.rs/tern/latest/tern/struct.Runner.html
/// [`QueryBuilder`]: https://docs.rs/tern/latest/tern/trait.QueryBuilder.html
/// [`MigrationContext`]: https://docs.rs/tern/latest/tern/trait.MigrationContext.html
/// [source-core]: https://docs.rs/tern/latest/tern/trait.MigrationSource.html
/// [`MigrationSource`]: crate::MigrationSource
/// [`Executor`]: https://docs.rs/tern/latest/tern/trait.Executor.html
#[proc_macro_derive(MigrationContext, attributes(tern))]
pub fn migration_context(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quote::expand_impl_migration_context(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// `MigrationSource` does the work of collecting all of the migrations, sorting
/// them, unifying SQL and Rust migrations under a common interface by
/// implementing [`Migration`], and then exposing methods to return an ordered
/// subset of them to be used in a given operation.  It has one required
/// attribute and one optional attribute.
///
/// * `source` is a required macro attribute.  It is the location of the
///   migration files relative to the project root (i.e., CARGO_MANIFEST_DIR).
///
/// ## Usage
///
/// ```rust,no_run
/// use tern::{SqlxPgExecutor, MigrationSource};
///
/// #[derive(MigrationSource)]
/// #[tern(source = "src/migrations")]
/// pub struct MyContext {
///     #[tern(executor_via)]
///     executor: SqlxPgExecutor,
/// }
/// ```
///
/// [`Migration`]: https://docs.rs/tern/latest/tern/trait.Migration.html
#[proc_macro_derive(MigrationSource, attributes(tern))]
pub fn migration_source(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quote::expand_impl_migration_source(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
