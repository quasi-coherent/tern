use proc_macro2::TokenStream;
use syn::Result;

use crate::internal::SourceFile;
use crate::internal::ast::{Container, SkipParseAttr};

/// Create the output token stream for deriving `Migration`.
pub fn expand_impl_migration(input: &syn::DeriveInput) -> Result<TokenStream> {
    let container = MigrationContainer::new(input)?;
    let tokens = container.quot_impl_migration()?;
    Ok(tokens)
}

/// Creates the token stream for expand_impl_migration.
type MigrationContainer<'a> = Container<'a, SkipParseAttr, SkipParseAttr>;

impl<'a> MigrationContainer<'a> {
    fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    fn quot_impl_migration(&self) -> Result<TokenStream> {
        let ident = &self.ty.ident;
        let source = SourceFile::from_spanned(ident)?;
        let output = source.quot_impl_migration_rs(ident);
        Ok(output)
    }
}
