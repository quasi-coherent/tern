use proc_macro2::TokenStream;
use std::collections::{BTreeMap, HashMap};
use syn::Result;

use crate::ast::{MigrationTokens as _, SourceExt, SourceFile};
use crate::rs::RustSource;
use crate::sql::SqlSource;

mod migration;
mod tern_app;
mod tern_test;

/// Collecting the whole token stream for `TernApp`.
pub(super) fn expand_impl_tern_app(
    input: &syn::DeriveInput,
) -> Result<TokenStream> {
    let container = tern_app::TernAppContainer::new(input)?;
    container.quot_impl_tern_app()
}

/// Collect the whole token stream for `TernTest`.
pub(super) fn expand_impl_tern_test(
    input: &syn::DeriveInput,
) -> Result<TokenStream> {
    let container = tern_test::TernTestContainer::new(input)?;
    container.quot_impl_tern_test()
}

/// Create the output token stream for deriving `Migration`.
pub(super) fn expand_impl_migration(
    input: &syn::DeriveInput,
) -> Result<TokenStream> {
    let container = migration::MigrationContainer::new(input)?;
    container.quot_impl_migration()
}

#[derive(Default)]
pub(self) struct SourceTokens {
    pub(self) set: BTreeMap<i64, TokenStream>,
    pub(self) mods: TokenStream,
}

impl SourceTokens {
    fn from_source(source: &syn::LitStr, ident: &syn::Ident) -> Result<Self> {
        let it = SourceFile::from_source_dir(source)?;

        let mut unpaired: HashMap<i64, SourceFile> = Default::default();
        let mut set: BTreeMap<i64, TokenStream> = Default::default();

        let mods =
            it.into_iter().try_fold(TokenStream::default(), |mods, res| {
                let src = res?;
                let ver = src.version;
                let ext = src.ext;

                match ext {
                    SourceExt::Rs => {
                        let rs_src = RustSource::from(src);
                        let boxed = rs_src.quot_boxed();

                        if set.insert(ver, boxed).is_some() {
                            return Err(syn::Error::new(
                                ident.span(),
                                "duplicate found",
                            ));
                        }

                        let rs_mod = rs_src.quot_mod();
                        Ok(quote::quote! {
                            #mods
                            #rs_mod
                        })
                    },
                    SourceExt::Sql => match unpaired.remove(&ver) {
                        Some(other) => {
                            let sql_src = SqlSource::combine(src, other)?;
                            let boxed = sql_src.quot_boxed();

                            if set.insert(ver, boxed).is_some() {
                                return Err(syn::Error::new(
                                    ident.span(),
                                    "duplicate found",
                                ));
                            }

                            let sql_mod = sql_src.quot_mod(ident);
                            Ok(quote::quote! {
                                #mods
                                #sql_mod
                            })
                        },
                        _ => {
                            unpaired.insert(ver, src);
                            return Ok(mods);
                        },
                    },
                }
            })?;

        if let Some((maxv, _)) = set.last_key_value()
            && let Some((minv, _)) = set.first_key_value()
            && let all_paired = unpaired.is_empty()
            && let count = *maxv == set.len() as i64
            && *minv == 1
            && !(all_paired && count)
        {
            let msg = format!(
                "bad migration state: up/down unpaired: {all_paired}, correct count: {count}"
            );
            Err(syn::Error::new(ident.span(), msg))
        } else {
            Ok(SourceTokens { set, mods })
        }
    }

    fn quot_impl_tern_app(&self, ident: &syn::Ident) -> TokenStream {
        let set: Vec<&TokenStream> = self.set.values().collect();

        quote::quote! {
            #[automatically_derived]
            impl ::tern::TernApp for #ident {
                type Set = ::tern::migration::MigrationBoxSet<#ident>;

                fn try_new_migration_set(&mut self) -> ::tern::TernResult<Self::Set> {
                    let set: Vec<::tern::migration::MigrationBox<#ident>> = vec![#(#set),*];
                    ::tern::migration::MigrationBoxSet::try_new(set)
                }
            }
        }
    }
}
