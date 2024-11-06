use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Error, Result};

use super::parse;

pub fn expand(input: syn::DeriveInput) -> Result<TokenStream> {
    let token = parse::SourceToken::new(input)?;
    let loc = &token.loc;
    let location = parse::cargo_manifest_dir().join(loc.value());
    let runtime = &token.runtime;
    let parsed = parse::ParsedSource::from_migration_dir(location).map_err(|e| {
        Error::new(
            loc.span(),
            format!("could not parse any migration sources {:?}", e),
        )
    })?;

    let mut use_modules = Vec::new();
    let mut declare_rs_modules = Vec::new();
    let mut migration_modules = Vec::new();
    let mut migration_sources = Vec::new();
    let mut migration_data = Vec::new();

    for src in parsed.iter() {
        let token = SourceToken::new(src);

        // To bring each migration module's members into scope
        // in the `migrate` module that has the main function.
        let module = &token.module;
        let module_orig = &token.module_orig;
        use_modules.push(quote! {use super::#module;});

        // Each migration has a module containing the necessary tokens.
        match src.source_type {
            parse::SourceType::Sql => {
                let statements = src.statements().map_err(|e| {
                    Error::new(
                        token.module_orig.span(),
                        format!("could not read raw sql: {e:?}"),
                    )
                })?;
                let quote_mod = token.quote_sql_migration_mod(statements);
                migration_modules.push(quote_mod);
            }
            _ => {
                let declare_rs_mod = quote! {mod #module_orig;};
                let quote_mod = token.quote_rs_migration_mod();
                declare_rs_modules.push(declare_rs_mod);
                migration_modules.push(quote_mod);
            }
        };

        // Each migration has a static `MigrationSource` built from the file.
        let migration_source = token.quote_migration_source();
        migration_sources.push(migration_source);

        // Data necessary for either a sql or rust build of a `Migration`.
        let migration_datum = token.quote_migration_datum(src.source_type);
        migration_data.push(migration_datum);
    }

    let quoted_types_mod = quote_types_mod(runtime);
    let quoted_runtime_impl = quote_runtime_impl(runtime, migration_sources, migration_data);

    let output = quote! {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate derrick as ___derrick;
        #(#declare_rs_modules)*
        #(#migration_modules)*
        #quoted_runtime_impl
        #quoted_types_mod
    };

    Ok(output)
}

fn quote_runtime_impl<T: ToTokens>(
    runtime: &syn::Ident,
    sources: Vec<T>,
    data: Vec<T>,
) -> TokenStream {
    let output = quote! {
        impl ___derrick::macros::Runner for #runtime {
            fn sources() -> Vec<___derrick::macros::MigrationSource> {
                let sources: Vec<___derrick::macros::MigrationSource> = vec![#(#sources),*];

                sources
            }

            fn unapplied<'a, 'c: 'a>(
                &'c mut self,
            ) -> ___derrick::macros::BoxFuture<'a, Result<Vec<___derrick::macros::Migration>, ___derrick::macros::Error>>
            {
                Box::pin(async move {
                    let migration_data: Vec<(i64, ___migration_types::MigrationData)> = vec![#(#data),*];
                    let current = self.current_version().await?;
                    let mut migrations: Vec<___derrick::macros::Migration> = Vec::new();

                    for (version, datum) in migration_data.into_iter() {
                        if matches!(current, Some(v) if version <= v) {
                            continue;
                        }

                        match datum {
                            ___migration_types::MigrationData::SqlData(migration) => migrations.push(migration),
                            ___migration_types::MigrationData::RsData(source, resolver) => {
                                let migration = resolver(self, &source).await?;
                                migrations.push(migration);
                            }
                        }
                    }

                    Ok(migrations)
                })
            }
        }
    };

    output
}

fn quote_types_mod(runtime: &syn::Ident) -> TokenStream {
    let output = quote! {
        pub mod ___migration_types {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate derrick as ___derrick;
            use super::#runtime;

            pub type ResolveMigrationFn = Box<
                dyn for<'c> FnOnce(
                    &'c mut #runtime,
                    &'c ___derrick::macros::MigrationSource,
                ) ->
                ___derrick::macros::BoxFuture<'c,
                    Result<___derrick::macros::Migration, ___derrick::macros::Error>>
                + Send
                + Sync
            >;

            pub enum MigrationData {
                SqlData(___derrick::macros::Migration),
                RsData(___derrick::macros::MigrationSource, ResolveMigrationFn),
            }
        }
    };

    output
}

struct SourceToken {
    module_orig: syn::Ident,
    module: syn::Ident,
    version: syn::LitInt,
    description: syn::LitStr,
    content: syn::LitStr,
}

impl SourceToken {
    fn new(source: &parse::ParsedSource) -> Self {
        let module_orig = syn::Ident::new(&source.module, Span::call_site());
        let module = syn::Ident::new(&format!("___{module_orig}"), Span::call_site());
        let version = syn::LitInt::new(&source.version.to_string(), Span::call_site());
        let description = syn::LitStr::new(&source.description, Span::call_site());
        let content = syn::LitStr::new(&source.content, Span::call_site());

        Self {
            module_orig,
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
                quote! {(#version, ___migration_types::MigrationData::SqlData(#module::migration()))}
            }
            parse::SourceType::Rust => {
                quote! {(#version, ___migration_types::MigrationData::RsData(#source, Box::new(#module::future_migration)))}
            }
        }
    }

    fn quote_migration_source(&self) -> TokenStream {
        let version = &self.version;
        let description = &self.description;
        let content = &self.content;
        let output = quote! {
            ___derrick::macros::MigrationSource {
                version: #version,
                description: #description.to_string(),
                content: #content.to_string(),
            }
        };

        output
    }

    fn quote_rs_migration_mod(&self) -> TokenStream {
        let module_orig = &self.module_orig;
        let module = &self.module;
        let output = quote! {
            mod #module {
                pub use super::#module_orig::future_migration;
            }
        };

        output
    }

    fn quote_sql_migration_mod(&self, statements: Vec<String>) -> TokenStream {
        let module = &self.module;
        let version = &self.version;
        let description = &self.description;
        let content = &self.content;
        let sql = &self.content;

        let output = quote! {
            mod #module {
                #[allow(unused_extern_crates, clippy::useless_attribute)]
                extern crate derrick as ___derrick;
                use super::*;

                const SQL: &str = #sql;

                pub fn migration() -> ___derrick::macros::Migration {
                    let ss: Vec<&str> = vec![#(#statements),*];
                    ___derrick::macros::Migration {
                        version: #version,
                        description: std::borrow::Cow::Owned(#description.to_string()),
                        content: std::borrow::Cow::Owned(#content.to_string()),
                        sql: std::borrow::Cow::Owned(#sql.to_string()),
                        statements: std::borrow::Cow::Owned(ss.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
                        no_tx: SQL
                            .lines()
                            .take(1)
                            .next()
                            .map(|l| l.contains("derrick:noTransaction"))
                            .unwrap_or_default(),
                    }
                }
            }
        };

        output
    }
}
