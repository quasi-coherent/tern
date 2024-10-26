use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, Error, Result};

pub fn expand(input: syn::DeriveInput) -> Result<TokenStream> {
    let token = DeriveToken::new(input)?;
    let builder_impl = token.quoted_query_builder_impl();
    let quoted_future_migration_fn = token.quoted_future_migration_fn();

    let output = quote! {
        extern crate derrick as _derrick;
        #quoted_future_migration_fn
        #builder_impl
    };

    Ok(output)
}

#[derive(Debug)]
struct DeriveToken {
    name: syn::Ident,
    no_tx: syn::LitBool,
    migrate: syn::Type,
}

impl DeriveToken {
    fn new(input: syn::DeriveInput) -> Result<Self> {
        let name = &input.ident;
        let no_tx = Self::attr_find_no_tx(&input)?;
        let migrate = Self::fields_find_migrate(&input)?;

        Ok(Self {
            name: name.clone(),
            no_tx,
            migrate,
        })
    }

    fn quoted_query_builder_impl(&self) -> TokenStream {
        let name = &self.name;
        let migrate = &self.migrate;
        let no_tx = &self.no_tx;
        let output = quote! {
            impl _derrick::macros::QueryBuilder for #name {
                type Runtime = #migrate;

                fn build_query(
                    &self,
                    migrate: &mut Self::Runtime,
                ) -> _derrick::macros::BoxFuture<'_, Result<_derrick::macros::MigrationQuery<'_>, _derrick::macros::Error>>
                {
                    Box::pin(async move {
                        let sql = #name::build_query(migrate).await?;
                        Ok(_derrick::macros::MigrationQuery::new(sql, #no_tx))
                    })
                }
            }
        };

        output
    }

    fn quoted_future_migration_fn(&self) -> TokenStream {
        let name = &self.name;
        let migrate = &self.migrate;
        let no_tx = &self.no_tx;
        let output = quote! {
            pub fn future_migration(
                source: &_derrick::macros::MigrationSource,
            ) -> Box<
                dyn for<'c> FnOnce(
                    &'c mut #migrate,
                )
                    -> _derrick::macros::BoxFuture<'c, _derrick::macros::FutureMigration<'c>>,
            > {
                Box::new(|migrate| {
                    let builder = #name;
                    let version = source.version;
                    _derrick::macros::FutureMigration::build(&mut migrate, &builder, source)
                })
            }
        };

        output
    }

    fn attr_find_no_tx(input: &syn::DeriveInput) -> Result<syn::LitBool> {
        let mut attr_tx: Option<bool> = None;
        for attr in &input.attrs {
            if attr.path().is_ident("migration") {
                if let Err(e) = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("no_transaction") {
                        attr_tx = Some(true);
                    }
                    Ok(())
                }) {
                    return Err(Error::new(attr.span(), "error parsing meta attributes"));
                }
            }
        }
        let no_tx = syn::LitBool::new(attr_tx.unwrap_or_default(), Span::call_site());

        Ok(no_tx)
    }

    fn fields_find_migrate(input: &syn::DeriveInput) -> Result<syn::Type> {
        let syn::Data::Struct(d) = &input.data else {
            return Err(Error::new(
                input.span(),
                "`QueryBuilder` only supports structs with one field",
            ));
        };
        let fields = match &d.fields {
            syn::Fields::Named(fields) => fields.named.iter(),
            syn::Fields::Unnamed(fields) => fields.unnamed.iter(),
            _ => Err(Error::new(
                input.span(),
                "A unit struct does not need `QueryBuilder`",
            ))?,
        }
        .collect::<Vec<_>>();

        match &fields[..] {
            [field] => Ok(field.ty.clone()),
            _ => Err(Error::new(
                input.span(),
                "`QueryBuilder` only supports structs with one field",
            )),
        }
    }
}
