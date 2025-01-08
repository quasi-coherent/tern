use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Result;

use super::TernDeriveAttr;
use crate::internal::ast::{Container, ParseAttr};

/// Derive `MigrationContext`.  This assumes that the type implements
/// `MigrationSource`, which can be done with the macro for it in this crate,
/// but it's more flexible to split them into two macros.
pub type MigrationContextContainer<'a> = Container<'a, TernDeriveAttr, MigrationContextFieldAttr>;

impl<'a> MigrationContextContainer<'a> {
    pub fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    pub fn quote_impl_migration_context(&self) -> Result<TokenStream> {
        let ident = &self.ty.ident;
        let quote_migration_ctx_body = self.quote_migration_context_body()?;
        let quote_impl_migration_ctx = quote! {
            #[automatically_derived]
            impl ::tern::migration::MigrationContext for #ident {
                #quote_migration_ctx_body
            }
        };

        Ok(quote_impl_migration_ctx)
    }

    fn quote_migration_context_body(&self) -> Result<TokenStream> {
        let table = &self.attrs.table;
        let exec_field = self
            .fields
            .fields
            .clone()
            .into_iter()
            .filter(|f| f.attrs.executor_via)
            .collect::<Vec<_>>();

        if exec_field.len() > 1 {
            Err(syn::Error::new(
                Span::call_site(),
                "at most one field may have the annotation `#[tern(executor_via)]`",
            ))?
        }
        // The target table for schema migration history defaults to
        // `_tern_migrations`.
        let quote_assoc_const = match table {
            Some(t) => quote! {const HISTORY_TABLE: &str = #t;},
            _ => quote! {const HISTORY_TABLE: &str = "_tern_migrations";},
        };
        // Construct the part of the impl body about the underlying query
        // executor type (i.e., database connection).
        let quote_exec_body = match &exec_field[..] {
            [field] => {
                let accessor = &field.member;
                let ty = &field.ty;
                quote! {
                    #quote_assoc_const
                    type Exec = #ty;
                    fn executor(&mut self) -> &mut Self::Exec {
                        &mut self.#accessor
                    }
                }
            }
            _ => quote! {
                #quote_assoc_const
                type Exec = Self;
                fn executor(&mut self) -> &mut Self::Exec {
                    self
                }
            },
        };

        Ok(quote_exec_body)
    }
}

#[derive(Default, Clone)]
pub struct MigrationContextFieldAttr {
    executor_via: bool,
}

impl ParseAttr<syn::Field> for MigrationContextFieldAttr {
    fn init() -> Self {
        Self::default()
    }

    fn attrs(input: &syn::Field) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()>
    where
        Self: Sized,
    {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                // If the field has `#[tern(executor_via)]` then the parsed
                // `internal::ast::Field<'a, MigrationContextFieldAttr>` will
                // have `field.attrs.executor_via = true`.
                if meta.path.is_ident("executor_via") {
                    self.executor_via = true;
                }

                Ok(())
            })?;
        }

        Ok(())
    }
}
