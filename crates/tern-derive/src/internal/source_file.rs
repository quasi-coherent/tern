use proc_macro2::{Span, TokenStream};
use regex::Regex;
use std::ffi::OsStr;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::SystemTime;
use syn::Result;
use syn::spanned::Spanned;

use super::{IntoResult as _, SourceExt, SourceType};

const PAT: &str = r#"^(V|U|D)(\d+)__(\w+)\.(sql|rs)$"#;

fn filename_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(PAT).unwrap())
}

/// Migration metadata parsed from the filename.
pub struct SourceFile {
    pub(super) this: syn::Ident,
    pub(super) pbuf: PathBuf,
    pub(super) modified: Option<syn::LitStr>,
    pub(super) version: i64,
    pub(super) description: syn::LitStr,
    pub(super) module: syn::Ident,
    pub(super) ext: SourceExt,
    pub(super) typ: SourceType,
}

impl SourceFile {
    /// For the type deriving `TernMigrate`, get the `SourceFile` from one entry
    /// in traversing the directory that the `source` attribute points to.
    pub fn from_entry(entry: DirEntry) -> Result<Self> {
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| {
                syn::LitStr::new(&d.as_millis().to_string(), Span::call_site())
            });
        let pbuf = entry.path();
        Self::from_path(pbuf, modified)
    }

    /// For the type deriving `Migration`, get the `SourceFile` from its span.
    pub fn from_spanned<S: Spanned>(val: &S) -> Result<Self> {
        let span = val.span();
        let pbuf =
            span.local_file().result_msg("error resolving source path")?;
        Self::from_path(pbuf, None)
    }

    /// Module declaration.
    ///
    /// For .rs this is just `mod the_filename;`.  For .sql it contains the
    /// whole implementation to get to `Up/DownMigration`.
    pub fn quot_mod(&self, ident: &syn::Ident) -> Result<TokenStream> {
        let module = &self.module;
        let output = match self.ext {
            SourceExt::Rs => {
                let path_str =
                    self.pbuf.to_str().result_msg("non-utf8 path")?;
                let path = syn::LitStr::new(path_str, Span::call_site());
                quote::quote! {
                    #[path = #path]
                    mod #module;
                }
            },
            _ => {
                let modified = &self.modified;
                let content = std::fs::read_to_string(&self.pbuf)
                    .map_err(|e| {
                        syn::Error::new(Span::call_site(), e.to_string())
                    })
                    .map(|s| syn::LitStr::new(&s, Span::call_site()))?;
                let quot_mig = self.quot_impl_migration_sql(ident, &content);

                quote::quote! {
                    #[doc(hidden)]
                    #[allow(
                        non_upper_case_globals,
                        unused_attributes,
                        unused_qualifications,
                        clippy::absolute_paths,
                    )]
                    mod #module {
                        #[doc = #modified]
                        use super::#ident;

                        #quot_mig
                    }
                }
            },
        };
        Ok(output)
    }

    /// Expression referencing the value exported by a module.
    pub fn quot_migration_expr(&self) -> TokenStream {
        let this = &self.this;
        let module = &self.module;
        quote::quote! { #module::#this::migration() }
    }

    /// Expression initializing `MigrationId`.
    pub fn quot_migration_id(&self) -> TokenStream {
        let ver = self.version;
        let desc = &self.description;
        quote::quote! { ::tern::migration::MigrationId::new(#ver, #desc) }
    }

    /// The expression `Up/DownMigration`.
    pub fn quot_dyn_migration(&self) -> TokenStream {
        match self.typ {
            SourceType::Down => {
                quote::quote! { ::tern::migration::DownMigration }
            },
            _ => quote::quote! { ::tern::migration::UpMigration },
        }
    }

    /// For .sql we directly implement `ResolveQuery` and `HasMigrationId`, hence
    /// `Migration`, for 0-sized type #this using the filename and contents.
    pub fn quot_impl_migration_sql(
        &self,
        ident: &syn::Ident,
        content: &syn::LitStr,
    ) -> TokenStream {
        let this = &self.this;
        let mig = self.quot_dyn_migration();
        let mid = self.quot_migration_id();

        quote::quote! {
            static __MIGRATION_ID: ::std::sync::LazyLock<::tern::migration::MigrationId> =
                ::std::sync::LazyLock::new(|| #mid);

            pub(super) struct #this;

            impl #this {
                pub(super) fn migration() -> #mig<#ident> {
                    #mig::new(#this)
                }
            }

            impl ::tern::ResolveQuery for #this {
                type Ctx = #ident;

                async fn init(_: &mut Self::Ctx) -> ::tern::TernResult<Self> {
                    Ok(#this)
                }

                async fn resolve(&self, _: &mut Self::Ctx) -> ::tern::TernResult<::tern::Query> {
                    ::tern::Query::from_sql(#content)
                }
            }

            impl ::tern::migration::types::HasMigrationId for #this {
                fn id_ref(&self) -> &::tern::migration::MigrationId {
                    &*__MIGRATION_ID
                }
            }
        }
    }

    /// For .rs, implementing `Migration` is a little more challenging because
    /// this is in a child module of the module containing the user's deriving
    /// `TernMigrate` type.  So this token stream does not have the symbol for
    /// it, and we must:
    ///
    /// 1. Use the migration ID from the filename to impl `HasMigrationId` for
    ///    the user's type.  This implies it's a `Migration` too.  Now we can
    ///    write `<#ident as Migration>::Ctx`, which is one requirement.
    /// 2. The user's `ResolveQuery` means it can init from just the ctx.  We
    ///    use this to define Migration::query for #this.
    ///
    /// This allows us to write the parameterless `#this::migration()` with
    /// correct return type expression.
    pub fn quot_impl_migration_rs(&self, ident: &syn::Ident) -> TokenStream {
        let this = &self.this;
        let mig = self.quot_dyn_migration();
        let mid = self.quot_migration_id();

        quote::quote! {
            static __MIGRATION_ID: ::std::sync::LazyLock<::tern::migration::MigrationId> =
                ::std::sync::LazyLock::new(|| #mid);

            pub(super) struct #this;

            impl #this {
                pub(super) fn migration() -> #mig<<#ident as ::tern::Migration>::Ctx> {
                    #mig::new(#this)
                }
            }

            impl ::tern::migration::types::HasMigrationId for #ident {
                fn id_ref(&self) -> &::tern::migration::MigrationId {
                    &*__MIGRATION_ID
                }
            }

            impl ::tern::Migration for #this {
                type Ctx = <#ident as ::tern::Migration>::Ctx;

                fn migration_id(&self) -> &::tern::migration::MigrationId {
                    &*__MIGRATION_ID
                }

                fn query<'a>(
                    &'a self,
                    ctx: &'a mut Self::Ctx,
                ) -> ::tern::private::BoxFuture<'a, ::tern::TernResult<::tern::Query>>
                {
                    ::std::boxed::Box::pin(async move {
                        let mig = <#ident as ::tern::ResolveQuery>::init(ctx).await?;
                        <#ident as ::tern::ResolveQuery>::resolve(&mig, ctx).await
                    })
                }
            }
        }
    }

    fn from_path(pbuf: PathBuf, modified: Option<syn::LitStr>) -> Result<Self> {
        let re = filename_re();
        let file_name = pbuf
            .as_path()
            .file_name()
            .and_then(OsStr::to_str)
            .result_msg("invalid path")?;

        let capt = re
            .captures(file_name)
            .result_msg(format!("invalid name: expected {PAT}"))?;

        let version = capt
            .get(2)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .result_msg("invalid version")?;

        let description = capt
            .get(3)
            .map(|d| syn::LitStr::new(d.as_str(), Span::call_site()))
            .result_msg("invalid description")?;

        let ext = capt
            .get(4)
            .and_then(|m| SourceExt::new(m.as_str()))
            .result_msg("invalid extension")?;

        let typ = capt
            .get(1)
            .and_then(|m| SourceType::new(m.as_str()))
            .result_msg("invalid source type")?;

        let module = pbuf
            .as_path()
            .file_stem()
            .and_then(OsStr::to_str)
            .map(|s| syn::Ident::new(s, Span::call_site()))
            .result_msgv(version, "error extracting file stem")?;

        let this_str = format!("__Resolve{}{}", typ, version);
        let this = syn::Ident::new(&this_str, Span::call_site());

        Ok(Self {
            this,
            pbuf,
            modified,
            version,
            description,
            module,
            ext,
            typ,
        })
    }
}
