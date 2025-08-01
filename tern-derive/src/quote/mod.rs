use proc_macro2::TokenStream;
use syn::Result;

mod migration;
mod migration_context;

pub(crate) fn expand_impl_migration(input: &syn::DeriveInput) -> Result<TokenStream> {
    let container = migration::MigrationContainer::new(input)?;
    let output = container.quote_impl_migration();
    Ok(output)
}

pub(crate) fn expand_impl_migration_context(input: &syn::DeriveInput) -> Result<TokenStream> {
    let container = migration_context::MigrationContextContainer::new(input)?;
    let output = container.quote_impl_migration_context()?;
    Ok(output)
}
