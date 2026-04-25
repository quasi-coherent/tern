use proc_macro2::{Span, TokenStream};
use syn::Result;
use syn::spanned::Spanned as _;

use crate::internal::ast::{Container, ParseAttr};
use crate::internal::{IntoResult as _, SourceMods};

const DEFAULT_HISTORY_TABLE: &str = "_tern_migrations";

/// Collecting the whole token stream for `TernMigrate`.
pub fn expand_impl_tern_migrate(
    input: &syn::DeriveInput,
) -> Result<TokenStream> {
    let container = TernMigrateContainer::new(input)?;
    let output = container.quot_impl_tern_migrate()?;
    Ok(output)
}

// Token stream for expand_impl_tern_migrate.
type TernMigrateContainer<'a> =
    Container<'a, TernMigrateDeriveAttr, TernMigrateFieldAttr>;

impl<'a> TernMigrateContainer<'a> {
    fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    fn quot_impl_tern_migrate(&self) -> Result<TokenStream> {
        let ident = &self.ty.ident;
        let src_mods = self.attrs.source_mods(ident)?;
        let impl_tern = src_mods.quot_impl_tern(ident);
        let impl_ctx = self.quot_impl_migration_context()?;
        let output = quote::quote! {
            #impl_tern
            #impl_ctx
        };
        Ok(output)
    }

    fn quot_impl_migration_context(&self) -> Result<TokenStream> {
        let ident = &self.ty.ident;
        let quot_hist = self.attrs.quot_history_table_fn();
        let quot_exec = self.quot_exec_fragment()?;
        let output = quote::quote! {
            impl ::tern::MigrationContext for #ident {
                #quot_exec
                #quot_hist
            }
        };
        Ok(output)
    }

    fn quot_exec_fragment(&self) -> Result<TokenStream> {
        let exec_field = self
            .fields
            .as_slice()
            .iter()
            .filter(|f| f.attrs.executor_via)
            .collect::<Vec<_>>();
        if exec_field.len() > 1 {
            Err(syn::Error::new(
                Span::call_site(),
                "found more than one field with `executor_via`",
            ))?
        }
        let output = match &exec_field[..] {
            [field] => {
                let accessor = &field.member;
                let ty = &field.ty;
                quote::quote! {
                    type Exec = #ty;
                    fn executor_mut(&mut self) -> &mut Self::Exec {
                        &mut self.#accessor
                    }
                }
            },
            _ => quote::quote! {
                type Exec = Self;
                fn executor(&mut self) -> &mut Self::Exec {
                    self
                }
            },
        };
        Ok(output)
    }
}

#[derive(Default)]
struct TernMigrateDeriveAttr {
    source: Option<syn::LitStr>,
    table: Option<syn::LitStr>,
    schema: Option<syn::LitStr>,
}

impl TernMigrateDeriveAttr {
    fn source_mods(&self, ident: &syn::Ident) -> Result<SourceMods> {
        let source =
            self.source.as_ref().result_msg("missing `source` attribute")?;
        SourceMods::new(ident, source)
    }

    fn quot_history_table_fn(&self) -> TokenStream {
        let quot_tbl = self.table.as_ref()
            .map(|t| quote::quote! { ::tern::HistoryTable::new(#t) })
            .unwrap_or(quote::quote! { ::tern::HistoryTable::new(#DEFAULT_HISTORY_TABLE) });
        let quot_hist_new = match self.schema.as_ref() {
            Some(s) => quote::quote! { #quot_tbl.in_namespace(#s) },
            _ => quot_tbl,
        };
        quote::quote! {
            fn history_table(&self) -> ::tern::HistoryTable {
                #quot_hist_new
            }
        }
    }
}

impl ParseAttr<syn::DeriveInput> for TernMigrateDeriveAttr {
    fn attrs(
        input: &syn::DeriveInput,
    ) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()> {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("schema") {
                    let parsed_schema: syn::LitStr = meta.value()?.parse()?;
                    self.schema = Some(parsed_schema);
                } else if meta.path.is_ident("table") {
                    let parsed_table: syn::LitStr = meta.value()?.parse()?;
                    self.table = Some(parsed_table);
                } else if meta.path.is_ident("source") {
                    let parsed_source: syn::LitStr = meta.value()?.parse()?;
                    self.source = Some(parsed_source);
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

#[derive(Default, Clone, Copy)]
struct TernMigrateFieldAttr {
    executor_via: bool,
}

impl ParseAttr<syn::Field> for TernMigrateFieldAttr {
    fn attrs(input: &syn::Field) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()> {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                // If the field has `#[tern(executor_via)]` then the parsed
                // `internal::ast::Field<'a, TernMigrateFieldAttr>` will
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
