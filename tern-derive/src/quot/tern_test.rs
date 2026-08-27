#![allow(unused)]
use proc_macro2::{Span, TokenStream};
use syn::Result;
use syn::spanned::Spanned as _;

use crate::ast::{Container, ParseAttr};

// Token stream for expand_impl_tern_test.
pub(super) type TernTestContainer<'a> =
    Container<'a, TernTestDeriveAttr, TernTestFieldAttr>;

impl<'a> TernTestContainer<'a> {
    pub(super) fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    pub(super) fn quot_impl_tern_test(&self) -> Result<TokenStream> {
        todo!()
    }
}

#[derive(Default)]
pub(super) struct TernTestDeriveAttr {
    properties: Option<syn::LitStr>,
}

impl TernTestDeriveAttr {
    fn quot_properties_fn(&self) -> Result<TokenStream> {
        todo!()
    }
}

impl ParseAttr<syn::DeriveInput> for TernTestDeriveAttr {
    fn attrs(
        input: &syn::DeriveInput,
    ) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()> {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("properties") {
                    let parsed: syn::LitStr = meta.value()?.parse()?;
                    self.properties = Some(parsed);
                } else {
                    Err(syn::Error::new(
                        attr.span(),
                        "unknown `tern` attribute",
                    ))?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct TernTestFieldAttr {
    app: bool,
}

impl ParseAttr<syn::Field> for TernTestFieldAttr {
    fn attrs(input: &syn::Field) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()> {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("app") {
                    self.app = true;
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}
