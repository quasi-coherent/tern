use proc_macro2::TokenStream;
use syn::Result;

use crate::rs::RustSource;
use crate::ast::{Container, MigrationTokens as _, SkipParseAttr};

/// Creates the token stream for expand_impl_migration.
pub(super) type MigrationContainer<'a> =
    Container<'a, SkipParseAttr, SkipParseAttr>;

impl<'a> MigrationContainer<'a> {
    pub(super) fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    pub(super) fn quot_impl_migration(&self) -> Result<TokenStream> {
        let ident = self.ty.ident;
        let source = RustSource::new(ident)?;
        Ok(source.quot_impl_migration(&ident))
    }
}
