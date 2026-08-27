use proc_macro2::{Span, TokenStream};
use syn::Result;
use syn::spanned::Spanned as _;

use crate::ast::{Container, ParseAttr};
use crate::quot::SourceTokens;

// Token stream for expand_impl_tern.
pub(super) type TernAppContainer<'a> =
    Container<'a, TernAppDeriveAttr, TernAppFieldAttr>;

impl<'a> TernAppContainer<'a> {
    pub(super) fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    pub(super) fn quot_impl_tern_app(&self) -> Result<TokenStream> {
        let ident = &self.ty.ident;
        let source = self.attrs.source(ident)?;
        let tokens = SourceTokens::from_source(source, ident)?;

        let mods = &tokens.mods;
        let impl_tern_app = tokens.quot_impl_tern_app(ident);

        let impl_ctx = self.quot_impl_migration_context()?;

        let output = quote::quote! {
            #mods
            #[doc(hidden)]
            #[allow(
                non_upper_case_globals,
                unused_attributes,
                unused_qualifications,
                clippy::absolute_paths,
            )]
            const _: () = {
                #impl_tern_app
                #impl_ctx
            };
        };
        Ok(output)
    }

    fn quot_impl_migration_context(&self) -> Result<TokenStream> {
        let ident = &self.ty.ident;
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
        let exec = match &exec_field[..] {
            [field] => {
                let accessor = &field.member;
                let ty = &field.ty;
                quote::quote! {
                    type Executor = #ty;
                    fn executor_mut(&mut self) -> &mut Self::Executor {
                        &mut self.#accessor
                    }
                }
            },
            _ => quote::quote! {
                type Executor = Self;
                fn executor_mut(&mut self) -> &mut Self::Executor {
                    self
                }
            },
        };
        let hist = self.attrs.quot_history_table_fn();
        let output = quote::quote! {
            #[automatically_derived]
            impl ::tern::migration::MigrationContext for #ident {
                #exec
                #hist
            }
        };

        Ok(output)
    }
}

#[derive(Default)]
pub(super) struct TernAppDeriveAttr {
    source: Option<syn::LitStr>,
    table: Option<syn::LitStr>,
    schema: Option<syn::LitStr>,
}

impl TernAppDeriveAttr {
    fn source(&self, ident: &syn::Ident) -> Result<&syn::LitStr> {
        self.source.as_ref().ok_or_else(|| {
            syn::Error::new(ident.span(), "missing required `source` attribute")
        })
    }

    fn quot_history_table_fn(&self) -> TokenStream {
        let quot_tbl = self
            .table
            .as_ref()
            .map(|t| quote::quote! { ::tern::migration::HistoryRelid::new(#t) })
            .unwrap_or(
                quote::quote! { ::tern::migration::HistoryRelid::default() },
            );
        let quot_hist_new = match self.schema.as_ref() {
            Some(s) => quote::quote! { #quot_tbl.set_relschema(#s) },
            _ => quot_tbl,
        };

        quote::quote! {
            fn history_table(&self) -> ::tern::migration::HistoryRelid {
                #quot_hist_new
            }
        }
    }
}

impl ParseAttr<syn::DeriveInput> for TernAppDeriveAttr {
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

#[derive(Clone, Copy, Default)]
pub(super) struct TernAppFieldAttr {
    executor_via: bool,
}

impl ParseAttr<syn::Field> for TernAppFieldAttr {
    fn attrs(input: &syn::Field) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()> {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                // If the field has `#[tern(executor_via)]` then the parsed
                // `internal::ast::Field<'a, TernAppFieldAttr>` will
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
