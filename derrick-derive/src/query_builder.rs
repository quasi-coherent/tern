use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, Error, Result};

pub fn expand(input: syn::DeriveInput) -> Result<TokenStream> {
    let token = DeriveToken::new(input)?;
    let builder_impl = token.quoted_query_builder_impl();
    let quoted_future_migration_fn = token.quoted_future_migration_fn();

    let output = quote! {
        extern crate derrick as _derrick;
        #builder_impl
        #quoted_future_migration_fn
    };

    Ok(output)
}

#[derive(Debug)]
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

    fn quoted_query_builder_impl(&self) -> TokenStream {
        let name = &self.name;
        let runtime = &self.runtime;
        let no_tx = &self.no_tx;
        let output = quote! {
            impl _derrick::macros::QueryBuilder for #name {
                type Runtime = #runtime;

                fn build_query<'a>(
                    &'a self,
                    runtime: &'a mut Self::Runtime,
                ) -> _derrick::macros::BoxFuture<'a, Result<_derrick::macros::MigrationQuery, _derrick::macros::Error>>
                {
                    Box::pin(async move {
                        let sql = build_query(runtime).await?;
                        Ok(_derrick::macros::MigrationQuery::new(sql, #no_tx))
                    })
                }
            }
        };

        output
    }

    fn quoted_future_migration_fn(&self) -> TokenStream {
        let name = &self.name;
        let runtime = &self.runtime;
        let output = quote! {
            pub fn future_migration<'a>(
                runtime: &'a mut #runtime,
                source: &'a _derrick::macros::MigrationSource,
            ) -> _derrick::macros::BoxFuture<'a, Result<_derrick::macros::Migration, _derrick::macros::Error>>
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