use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

use crate::internal::ast::{Container, ParseAttr, SkipParseAttr};

// A Rust migration needs to implements `Migration` and this helper macro
// contributes a method `no_tx` for that.
pub type MigrationContainer<'a> = Container<'a, MigrationAttr, SkipParseAttr>;

impl<'a> MigrationContainer<'a> {
    pub fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    pub fn quote_impl_migration(&self) -> TokenStream {
        let no_tx = &self.attrs.no_tx;

        quote! {
            impl TernMigration {
                pub fn no_tx(&self) -> bool {
                    #no_tx
                }
            }
        }
    }
}

#[derive(Default)]
pub struct MigrationAttr {
    no_tx: bool,
}

impl ParseAttr<syn::DeriveInput> for MigrationAttr {
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
                if meta.path.is_ident("no_transaction") {
                    self.no_tx = true;
                }

                Ok(())
            })?;
        }

        Ok(())
    }
}
