use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::Result;

use crate::internal::ast::ParseAttr;

mod migration;
mod migration_context;
mod migration_source;

pub fn expand_impl_migration(input: &syn::DeriveInput) -> Result<TokenStream> {
    let container = migration::MigrationContainer::new(input)?;
    let output = container.quote_impl_migration();
    Ok(output)
}

pub fn expand_impl_migration_context(input: &syn::DeriveInput) -> Result<TokenStream> {
    let container = migration_context::MigrationContextContainer::new(input)?;
    let output = container.quote_impl_migration_context()?;
    Ok(output)
}

pub fn expand_impl_migration_source(input: &syn::DeriveInput) -> Result<TokenStream> {
    let container = migration_source::MigrationSourceContainer::new(input)?;
    let output = container.quote_impl_migration_source()?;
    Ok(output)
}

/// The derive macros `MigrationSource` and `MigrationContext` share the `tern`
/// attribute, so they have to share the same parsed representation of it.
#[derive(Default, Clone)]
pub struct TernDeriveAttr {
    source: Option<syn::LitStr>,
    table: Option<syn::LitStr>,
}

impl ParseAttr<syn::DeriveInput> for TernDeriveAttr {
    fn init() -> Self {
        Self::default()
    }

    fn attrs(input: &syn::DeriveInput) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()>
    where
        Self: Sized,
    {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("table") {
                    let parsed_table: syn::LitStr = meta.value()?.parse()?;
                    self.table = Some(parsed_table);
                } else if meta.path.is_ident("source") {
                    let parsed_source: syn::LitStr = meta.value()?.parse()?;
                    self.source = Some(parsed_source);
                } else {
                    Err(syn::Error::new(attr.span(), "unknown `tern` attribute"))?;
                }

                Ok(())
            })?;
        }

        Ok(())
    }
}
