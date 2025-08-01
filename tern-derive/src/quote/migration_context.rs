use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Result;
use syn::spanned::Spanned;

use crate::internal::ast::{Container, ParseAttr};
use crate::internal::parse;

// Derive `MigrationContext`.  This assumes that the type implements
// `MigrationSource`, which can be done with the macro for it in this crate,
// but it's more flexible to split them into two macros.
pub type MigrationContextContainer<'a> =
    Container<'a, MigrationContextDeriveAttr, MigrationContextFieldAttr>;

impl<'a> MigrationContextContainer<'a> {
    pub fn new(input: &'a syn::DeriveInput) -> Result<Self> {
        Container::from_ast(input)
    }

    pub fn quote_impl_migration_context(&self) -> Result<TokenStream> {
        let ident = &self.ty.ident;
        let source = &self.attrs.source;

        let migration_set = MigrationSetContainer::new(ident, source)?;

        let quote_migration_mods = migration_set.quote_migration_modules();
        let quote_migration_impls = migration_set.quote_migration_impls();
        let quote_migrations = quote! {
            #quote_migration_mods
            #[doc(hidden)]
            #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
            const _: () = {
                #quote_migration_impls
            };
        };

        let quote_migration_ctx_body = self.quote_migration_context_body(&migration_set)?;

        let quote_impl_migration_ctx = quote! {
            #quote_migrations
            #[automatically_derived]
            impl ::tern::migration::MigrationContext for #ident {
                #quote_migration_ctx_body
            }
        };

        Ok(quote_impl_migration_ctx)
    }

    fn quote_migration_context_body(
        &self,
        migration_set: &MigrationSetContainer,
    ) -> Result<TokenStream> {
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

        let quote_migration_set_impl = migration_set.quote_migration_set_impl();

        // Construct the part of the impl body about the underlying query
        // executor type (i.e., database connection).
        let quote_ctx_body = match &exec_field[..] {
            [field] => {
                let accessor = &field.member;
                let ty = &field.ty;
                quote! {
                    type Exec = #ty;
                    #quote_assoc_const
                    #quote_migration_set_impl
                    fn executor(&mut self) -> &mut Self::Exec {
                        &mut self.#accessor
                    }
                }
            }
            _ => quote! {
                type Exec = Self;
                #quote_assoc_const
                #quote_migration_set_impl
                fn executor(&mut self) -> &mut Self::Exec {
                    self
                }
            },
        };

        Ok(quote_ctx_body)
    }
}

#[derive(Default, Clone)]
pub struct MigrationContextDeriveAttr {
    source: Option<syn::LitStr>,
    table: Option<syn::LitStr>,
}

impl ParseAttr<syn::DeriveInput> for MigrationContextDeriveAttr {
    fn init() -> Self {
        Self::default()
    }

    fn attrs(input: &syn::DeriveInput) -> impl Iterator<Item = &syn::Attribute> {
        input.attrs.iter()
    }

    fn update(&mut self, attr: &syn::Attribute) -> Result<()>
    where
        Self: Sized,
    {
        if attr.path().is_ident("tern") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("table") {
                    let parsed_table: syn::LitStr = meta.value()?.parse()?;
                    self.table = Some(parsed_table);
                } else if meta.path.is_ident("source") {
                    let parsed_source: syn::LitStr = meta.value()?.parse()?;
                    self.source = Some(parsed_source);
                } else {
                    Err(syn::Error::new(attr.span(), "unknown `tern` attribute"))?;
                }

                Ok(())
            })?;
        }

        Ok(())
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

// The sorted list of tokens parsed from all migration files.
struct MigrationSetContainer {
    ident: syn::Ident,
    migrations: Vec<MigrationContainer>,
}

// A single parsed migration file.
enum MigrationContainer {
    Sql(SqlSourceContainer),
    Rs(RustSourceContainer),
}

struct SqlSourceContainer {
    module: syn::Ident,
    version: syn::LitInt,
    description: syn::LitStr,
    content: syn::LitStr,
    no_tx: syn::LitBool,
}

struct RustSourceContainer {
    module: syn::Ident,
    version: syn::LitInt,
    description: syn::LitStr,
    content: syn::LitStr,
}

impl MigrationSetContainer {
    fn new(ident: &syn::Ident, source: &Option<syn::LitStr>) -> Result<Self> {
        let src = source.as_ref().map(|s| s.value()).ok_or_else(|| {
            syn::Error::new(
                ident.span(),
                "missing required `source` attribute containing the path to the migration files",
            )
        })?;
        let migration_dir = parse::cargo_manifest_dir().join(src);
        let migrations = parse::MigrationSource::from_migration_dir(migration_dir)
            .map_err(|e| {
                syn::Error::new(ident.span(), format!("error with migration source: {e:?}"))
            })?
            .into_iter()
            .map(MigrationContainer::from)
            .collect::<Vec<_>>();

        Ok(Self {
            ident: ident.clone(),
            migrations,
        })
    }

    // Use the expanded vector of `Box::new(module_name::TernMigration)`s
    // to build a `Vec<Box<dyn Migration<Ctx = Self>>>`.  This works because we
    // have already implemented `Migration` for all of them.
    fn quote_migration_set_impl(&self) -> TokenStream {
        let boxed_migrations = self.quote_boxed_qualified_migration_types();

        quote! {
            fn migration_set(&self, target_version: Option<i64>) -> ::tern::migration::MigrationSet<Self> {
                let all: Vec<Box<dyn ::tern::migration::Migration<Ctx = Self>>> = vec![#(#boxed_migrations),*];
                let Some(v) = target_version else {
                    return ::tern::migration::MigrationSet::new(all);
                };
                let migrations: Vec<Box<dyn ::tern::migration::Migration<Ctx = Self>>> = all
                    .into_iter()
                    .skip_while(|m| m.as_ref().version() <= v)
                    .collect::<Vec<_>>();

                ::tern::migration::MigrationSet::new(migrations)
            }
        }
    }

    // Iterate over the token containers for both Rust and SQL migrations that
    // we derived from the source files and declare for each the module that,
    // in both cases, has a type `TernMigration` that implements the
    // trait `QueryBuilder`.
    fn quote_migration_modules(&self) -> TokenStream {
        let ctx = &self.ident;

        self.migrations.iter().fold(quote! {}, |acc, src| {
            let quote_migration_mod = src.quote_migration_module(ctx);

            quote! {
                #acc
                #quote_migration_mod
            }
        })
    }

    // Iterate over the token containers and collect all the implementations of
    // `Migration` for `module_name::TernMigration`.
    fn quote_migration_impls(&self) -> TokenStream {
        let ctx = &self.ident;

        self.migrations.iter().fold(quote! {}, |acc, src| {
            let quote_impl_migration = src.quote_impl_migration(ctx);

            quote! {
                #acc
                #quote_impl_migration
            }
        })
    }

    // A vector of all the `TernMigration`s for each module.
    fn quote_boxed_qualified_migration_types(&self) -> Vec<TokenStream> {
        self.migrations
            .iter()
            .map(|s| s.quote_boxed_qualified_migration_type())
            .collect::<Vec<_>>()
    }
}

impl MigrationContainer {
    // `Box<module_name::TernMigration>`.
    fn quote_boxed_qualified_migration_type(&self) -> TokenStream {
        let module = self.module();
        quote! {Box::new(#module::TernMigration)}
    }

    // Make a child module in this module.  Rust ones already exist so this just
    // declares it: `mod rust_migration_filename;`.  SQL ones we have to create,
    // and then declare `TernMigration` and impl `QueryBuilder` for it.
    //
    // Rust migrations get a `TernMigration` and the impl `QueryBuilder` from
    // the user being required to do that.
    fn quote_migration_module(&self, ctx: &syn::Ident) -> TokenStream {
        let module = self.module();

        match self {
            Self::Sql(s) => {
                let quote_impl_query_builder = s.quote_impl_query_builder(ctx);

                quote! {
                    mod #module {
                        use super::#ctx;
                        #[derive(Debug, Clone)]
                        pub struct TernMigration;
                        #quote_impl_query_builder
                    }
                }
            }
            Self::Rs(_) => {
                quote! {
                    mod #module;
                }
            }
        }
    }

    // The `Migration` impl for `migration_filename::TernMigration` where
    // it's expected that `TernMigration` implements the `QueryBuilder`
    // trait.  SQL migrations do it inside the ad-hoc module we create in
    // `quote_migration_module` and Rust migrations expect the user to do it.
    fn quote_impl_migration(&self, ctx: &syn::Ident) -> TokenStream {
        let module = self.module();
        let quote_common = self.quote_common_migration_fns();
        let no_tx_body = match self {
            Self::Sql(s) => {
                let no_tx = &s.no_tx;
                quote! { #no_tx }
            }
            _ => quote! { self.no_tx() },
        };

        quote! {
            impl ::tern::migration::Migration for #module::TernMigration {
                type Ctx = #ctx;

                #quote_common

                fn no_tx(&self) -> bool {
                    #no_tx_body
                }
            }
        }
    }

    // Identical SQL/Rust `Migration` method implementations.
    fn quote_common_migration_fns(&self) -> TokenStream {
        let description = self.description();
        let version = self.version();
        let content = self.content();

        quote! {
            fn migration_id(&self) -> ::tern::migration::MigrationId {
                let description = #description.to_string();
                ::tern::migration::MigrationId::new(#version, description)
            }

            fn content(&self) -> String {
                #content.to_string()
            }

            fn build<'a>(
                &'a self,
                ctx: &'a mut Self::Ctx,
            ) -> ::tern::future::BoxFuture<'a, ::tern::error::TernResult<::tern::migration::Query>>
            {
                Box::pin(<Self as ::tern::migration::QueryBuilder>::build(self, ctx))
            }
        }
    }

    // Fields that both Rust/SQL migrations have.
    fn module(&self) -> &syn::Ident {
        match self {
            Self::Sql(s) => &s.module,
            Self::Rs(s) => &s.module,
        }
    }
    fn version(&self) -> &syn::LitInt {
        match self {
            Self::Sql(s) => &s.version,
            Self::Rs(s) => &s.version,
        }
    }
    fn description(&self) -> &syn::LitStr {
        match self {
            Self::Sql(s) => &s.description,
            Self::Rs(s) => &s.description,
        }
    }
    fn content(&self) -> &syn::LitStr {
        match self {
            Self::Sql(s) => &s.content,
            Self::Rs(s) => &s.content,
        }
    }
}

impl SqlSourceContainer {
    // A SQL migration has an impl `QueryBuilder` that just returns the contents
    // of the file.  It is needed in the module we create named after the .sql
    // file to unify the treatment of Rust and SQL migrations.
    fn quote_impl_query_builder(&self, ctx: &syn::Ident) -> TokenStream {
        let content = &self.content;

        quote! {
            #[automatically_derived]
            impl ::tern::migration::QueryBuilder for TernMigration {
                type Ctx = #ctx;

                async fn build(
                    &self,
                    ctx: &mut Self::Ctx,
                ) -> ::tern::error::TernResult<::tern::migration::Query>
                {
                    let sql = #content.to_string();
                    Ok(::tern::migration::Query::new(sql))
                }
            }
        }
    }
}

impl From<parse::SqlSource> for SqlSourceContainer {
    fn from(value: parse::SqlSource) -> Self {
        Self {
            module: syn::Ident::new(&value.module, Span::call_site()),
            version: syn::LitInt::new(&format!("{}", value.version), Span::call_site()),
            description: syn::LitStr::new(&value.description, Span::call_site()),
            content: syn::LitStr::new(&value.content, Span::call_site()),
            no_tx: syn::LitBool::new(value.no_tx, Span::call_site()),
        }
    }
}

impl From<parse::RustSource> for RustSourceContainer {
    fn from(value: parse::RustSource) -> Self {
        Self {
            module: syn::Ident::new(&value.module, Span::call_site()),
            version: syn::LitInt::new(&format!("{}", value.version), Span::call_site()),
            description: syn::LitStr::new(&value.description, Span::call_site()),
            content: syn::LitStr::new(&value.content, Span::call_site()),
        }
    }
}

impl From<parse::MigrationSource> for MigrationContainer {
    fn from(value: parse::MigrationSource) -> Self {
        match value {
            parse::MigrationSource::Sql(s) => Self::Sql(SqlSourceContainer::from(s)),
            parse::MigrationSource::Rs(s) => Self::Rs(RustSourceContainer::from(s)),
        }
    }
}
