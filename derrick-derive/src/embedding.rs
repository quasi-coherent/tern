use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Error, Result};

use super::mangle;
use super::parse;

pub fn expand(input: parse::EmbedInput) -> Result<TokenStream> {
    let loc = &input.loc;
    let location = parse::cargo_manifest_dir().join(loc.value());
    let migrate = &input.migrate;
    let parsed = parse::ParsedSource::from_migration_dir(location).map_err(|e| {
        Error::new(
            loc.span(),
            format!("could not parse any migration sources {:?}", e),
        )
    })?;

    let mut future_migration_fns = Vec::new();
    let mut migration_sources = Vec::new();
    let mut migration_modules = Vec::new();
    let mut use_modules = Vec::new();

    for src in parsed.iter() {
        let token = SourceToken::new(src, &migrate);
        let module = &token.module;

        let (quote_mod, future_mig_fn) = match src.source_type {
            parse::SourceType::Sql => (
                token.quote_sql_migration_mod(),
                token.quote_sql_future_migration_fn(),
            ),
            _ => (
                quote! {pub mod #module;},
                token.quote_rust_future_migration_fn(),
            ),
        };

        let migration_source = token.quote_migration_source();
        let version = token.version;

        migration_sources.push(migration_source);
        migration_modules.push(quote_mod);
        use_modules.push(quote! {pub use super::#module;});
        future_migration_fns.push(quote! {(#version, #future_mig_fn)});
    }
    let quoted_migration_mod_fn = quote_migration_mod_fn(
        migrate,
        migration_sources,
        future_migration_fns,
        use_modules,
    );
    let output = quote! {
        #(#migration_modules)*
        #quoted_migration_mod_fn
    };

    Ok(output)
}

/// "Main" mod containing the `ready` method. which
/// interacts with a `Runner` by preparing
/// migration sources, and resolving those that
/// are unapplied and need a query, which
/// pass back to the `Runner` to apply.
fn quote_migration_mod_fn<T: ToTokens>(
    migrate: &syn::Type,
    sources: Vec<T>,
    migration_fns: Vec<T>,
    use_modules: Vec<T>,
) -> TokenStream {
    let output = quote! {
        #(#use_modules)*
        async fn ready<'a>(
            runner: _derrick::macros::Runner<'a>,
            migrate: &mut #migrate,
        ) -> Result<_derrick::macros::Runner<'a>, _derrick::macros::Error> {
            let sources: Vec<_derrick::macros::MigrationSource> = vec![#(#sources),*];

            let resolvers: Vec<(
                i64,
                Box<dyn for<'c> FnOnce(&'c mut #migrate) -> _derrick::macros::BoxFuture<'c, _derrick::macros::FutureMigration<'c>>>,
            )> = vec![#(#migration_fns),*];

            let latest = runner.ready(migrate, sources).await?;

            let mut migrations: Vec<_derrick::macros::Migration> = Vec::new();

            for (version, mut resolver) in resolvers.into_iter() {
                if version <= latest {
                    continue;
                }

                let fut = resolver(migrate).as_mut().await;
                let migration = fut.migration.await?;

                migrations.push(migration);
            }

            let runner = runner.set_unapplied(migrations);

            Ok(runner)
        }
    };

    mangle::wrap_in_mod(&syn::Ident::new("migrate", Span::call_site()), output)
}

#[derive(Debug)]
struct SourceToken {
    module: syn::Ident,
    migrate: syn::Type,
    version: syn::LitInt,
    description: syn::LitStr,
    content: syn::LitStr,
}

impl SourceToken {
    fn new(source: &parse::ParsedSource, migrate: &syn::Type) -> Self {
        let module = syn::Ident::new(&source.module, Span::call_site());
        let migrate = migrate.clone();
        let version = syn::LitInt::new(&source.version.to_string(), Span::call_site());
        let description = syn::LitStr::new(&source.description, Span::call_site());
        let content = syn::LitStr::new(&source.content, Span::call_site());

        Self {
            module,
            migrate,
            version,
            description,
            content,
        }
    }

    fn quote_migration_source(&self) -> TokenStream {
        let version = &self.version;
        let description = &self.description;
        let content = &self.content;
        let output = quote! {
            _derrick::macros::MigrationSource {
                version: #version,
                description: #description.to_string(),
                content: #content.to_string(),
            }
        };

        output
    }

    fn quote_sql_future_migration_fn(&self) -> TokenStream {
        let module = &self.module;
        let output = quote! {Box::new(#module::future_migration)};

        output
    }

    fn quote_rust_future_migration_fn(&self) -> TokenStream {
        let module = &self.module;
        let source = self.quote_migration_source();
        let output = quote! {#module::future_migration(&#source)};

        output
    }

    fn quote_sql_migration_mod(&self) -> TokenStream {
        let module = &self.module;
        let migrate = &self.migrate;
        let version = &self.version;
        let description = &self.description;
        let content = &self.content;
        let sql = &self.content;
        let code = quote! {
            const SQL: &str = #sql;

            pub fn future_migration(
                _: &mut #migrate,
            ) -> _derrick::macros::BoxFuture<'_, _derrick::macros::FutureMigration<'_>>
            {
                Box::pin(async move {
                    let migration_source = _derrick::macros::MigrationSource {
                        version: #version,
                        description: #description.to_string(),
                        content: #content.to_string(),
                    };
                    let no_tx = SQL
                        .lines()
                        .take(1)
                        .next()
                        .map(|l| l.contains("derrick:noTransaction"))
                        .unwrap_or_default();
                    let query = _derrick::macros::MigrationQuery::new(SQL, no_tx);
                    let migration = Box::pin(async move {
                        Ok(_derrick::macros::Migration::new(
                            &migration_source,
                            &query,
                        ))
                    });
                    _derrick::macros::FutureMigration {
                        version: #version,
                        migration,
                    }
                })
            }
        };

        mangle::wrap_in_mod(module, code)
    }
}
