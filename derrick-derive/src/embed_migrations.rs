use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Error, Result};

use super::parse;

pub fn expand(input: parse::EmbedInput) -> Result<TokenStream> {
    let loc = &input.loc;
    let location = parse::cargo_manifest_dir().join(loc.value());
    let runtime = &input.runtime;
    let parsed = parse::ParsedSource::from_migration_dir(location).map_err(|e| {
        Error::new(
            loc.span(),
            format!("could not parse any migration sources {:?}", e),
        )
    })?;

    let mut use_modules = Vec::new();
    let mut migration_modules = Vec::new();
    let mut migration_sources = Vec::new();
    let mut migration_data = Vec::new();

    for src in parsed.iter() {
        let token = SourceToken::new(src);

        // To bring each migration module's members into scope
        // in the `migrate` module that has the main function.
        let module = &token.module;
        use_modules.push(quote! {use super::#module;});

        // Each migration has a module containing the necessary tokens.
        let quote_mod = match src.source_type {
            parse::SourceType::Sql => token.quote_sql_migration_mod(),
            _ => quote! {pub mod #module;},
        };
        migration_modules.push(quote_mod);

        // Each migration has a static `MigrationSource` built from the file.
        let migration_source = token.quote_migration_source();
        migration_sources.push(migration_source);

        // Data necessary for either sql or rust to build a `Migration` that
        // we send back to the runner.
        let migration_datum = token.quote_migration_datum(src.source_type);
        migration_data.push(migration_datum);
    }

    let quoted_migrate_mod =
        quote_migrate_mod(runtime, migration_sources, migration_data, use_modules);
    let output = quote! {
        pub use migrate::*;

        #(#migration_modules)*
        #quoted_migrate_mod
    };

    Ok(output)
}

/// "Main" mod containing the `ready` method. which interacts with
/// an instance of `derrick_backends::Runner` through validating
/// and preparing migration sources, and resolving those that are
/// unapplied and need a query, which pass back to the `Runner` to apply.
fn quote_migrate_mod<T: ToTokens>(
    runtime: &syn::Type,
    sources: Vec<T>,
    data: Vec<T>,
    use_modules: Vec<T>,
) -> TokenStream {
    let output = quote! {
        mod migrate {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate derrick as _derrick;
            #[allow(unused_imports)]
            use super::*;

            #(#use_modules)*

            type FnResolveMig = Box<
                dyn for<'c> FnOnce(
                    &'c mut #runtime,
                    &'c _derrick::macros::MigrationSource,
                ) -> _derrick::macros::BoxFuture<
                    'c,
                    Result<_derrick::macros::Migration, _derrick::macros::Error>,
                >,
            >;

            enum MigrationData {
                SqlData(_derrick::macros::Migration),
                RsData(_derrick::macros::MigrationSource, FnResolveMig),
            }

            pub async fn ready<'a>(
                runner: _derrick::macros::Runner<'a>,
                runtime: &mut #runtime,
            ) -> Result<_derrick::macros::Runner<'a>, _derrick::macros::Error> {
                let sources: Vec<_derrick::macros::MigrationSource> = vec![#(#sources),*];

                let latest = runner.ready(runtime, sources).await?;

                let migration_data: Vec<(i64, MigrationData)> = vec![#(#data),*];

                let mut migrations: Vec<_derrick::macros::Migration> = Vec::new();

                for (version, data) in migration_data.into_iter() {
                    if version <= latest {
                        continue;
                    }

                    match data {
                        MigrationData::SqlData(migration) => migrations.push(migration),
                        MigrationData::RsData(source, resolver) => {
                            let migration = resolver(runtime, &source).await?;
                            migrations.push(migration)
                        }
                    }
                }

                let runner = runner.set_unapplied(migrations);

                Ok(runner)
            }
        }
    };

    output
}

#[derive(Debug)]
struct SourceToken {
    module: syn::Ident,
    version: syn::LitInt,
    description: syn::LitStr,
    content: syn::LitStr,
}

impl SourceToken {
    fn new(source: &parse::ParsedSource) -> Self {
        let module = syn::Ident::new(&source.module, Span::call_site());
        let version = syn::LitInt::new(&source.version.to_string(), Span::call_site());
        let description = syn::LitStr::new(&source.description, Span::call_site());
        let content = syn::LitStr::new(&source.content, Span::call_site());

        Self {
            module,
            version,
            description,
            content,
        }
    }

    fn quote_migration_datum(&self, source_type: parse::SourceType) -> TokenStream {
        let module = &self.module;
        let version = &self.version;
        let source = self.quote_migration_source();

        match source_type {
            parse::SourceType::Sql => {
                quote! {(#version, MigrationData::SqlData(#module::migration()))}
            }
            parse::SourceType::Rust => {
                quote! {(#version, MigrationData::RsData(#source, Box::new(#module::future_migration)))}
            }
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

    fn quote_sql_migration_mod(&self) -> TokenStream {
        let module = &self.module;
        let version = &self.version;
        let description = &self.description;
        let content = &self.content;
        let sql = &self.content;

        let output = quote! {
            mod #module {
                #[allow(unused_extern_crates, clippy::useless_attribute)]
                extern crate derrick as _derrick;
                #[allow(unused_imports)]
                use super::*;

                const SQL: &str = #sql;

                pub fn migration() -> _derrick::macros::Migration {
                    let source = _derrick::macros::MigrationSource {
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
                    let query = _derrick::macros::MigrationQuery::new(SQL.to_string(), no_tx);

                    _derrick::macros::Migration::new(&source, query)
                }
            }
        };

        output
    }
}
