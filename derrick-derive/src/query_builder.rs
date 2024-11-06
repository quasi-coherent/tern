use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, Error, Result};

pub fn expand(input: syn::DeriveInput) -> Result<TokenStream> {
    let token = DeriveToken::new(input)?;
    let quoted_builder_impl = token.quote_query_builder_impl();
    let quoted_future_migration_fn = token.quote_future_migration_fn();

    let output = quote! {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate derrick as ___derrick;
        #quoted_builder_impl
        #quoted_future_migration_fn
    };

    Ok(output)
}

struct DeriveToken {
    name: syn::Ident,
    no_tx: syn::LitBool,
    runtime: syn::Ident,
}

impl DeriveToken {
    fn new(input: syn::DeriveInput) -> Result<Self> {
        let name = &input.ident;
        let (no_tx, runtime) = Self::migration_attr(&input)?;

        Ok(Self {
            name: name.clone(),
            no_tx,
            runtime,
        })
    }

    fn quote_query_builder_impl(&self) -> TokenStream {
        let name = &self.name;
        let runtime = &self.runtime;
        let no_tx = &self.no_tx;
        let output = quote! {
            impl ___derrick::macros::QueryBuilder for #name {
                type Runtime = #runtime;

                fn build_query<'a>(
                    &'a self,
                    runtime: &'a mut Self::Runtime,
                ) -> ___derrick::macros::BoxFuture<'a, Result<___derrick::macros::MigrationQuery, ___derrick::macros::Error>>
                {
                    Box::pin(async move {
                        let sql = build_query(runtime).await?;
                        Ok(___derrick::macros::MigrationQuery::new(sql, #no_tx))
                    })
                }
            }
        };

        output
    }

    fn quote_future_migration_fn(&self) -> TokenStream {
        let name = &self.name;
        let runtime = &self.runtime;
        let output = quote! {
            pub fn future_migration<'a>(
                runtime: &'a mut #runtime,
                source: &'a ___derrick::macros::MigrationSource,
            ) -> ___derrick::macros::BoxFuture<'a, Result<___derrick::macros::Migration, ___derrick::macros::Error>>
            {
                let query_builder = #name;
                #name.resolve(runtime, source)
            }
        };

        output
    }

    fn migration_attr(input: &syn::DeriveInput) -> Result<(syn::LitBool, syn::Ident)> {
        let mut no_tx_arg = None::<bool>;
        let mut runtime_arg = None::<syn::Ident>;
        for attr in &input.attrs {
            if attr.path().is_ident("migration") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("no_transaction") {
                        no_tx_arg = Some(true);
                    }

                    if meta.path.is_ident("runtime") {
                        let ident: syn::Ident = meta.value()?.parse()?;
                        runtime_arg = Some(ident);
                    }

                    Ok(())
                })?;
            }
        }
        let no_tx = syn::LitBool::new(no_tx_arg.unwrap_or_default(), Span::call_site());
        let runtime = runtime_arg.ok_or(Error::new(
            input.span(),
            "arg `runtime = ...` not found for `migration` attr",
        ))?;

        Ok((no_tx, runtime))
    }
}
