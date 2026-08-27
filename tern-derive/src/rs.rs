use proc_macro2::{Span, TokenStream};
use syn::Result;
use syn::spanned::Spanned;

use crate::ast::*;

/// An .rs file with the necessary implementations for both up/down migrations.
pub(crate) struct RustSource {
    ident: syn::Ident,
    module: syn::Ident,
    version: i64,
    description: syn::LitStr,
    path: syn::LitStr,
    // https://github.com/quasi-coherent/tern/issues/46
    _sql_pair: Option<syn::LitStr>,
    typ: SourceType,
}

impl RustSource {
    pub(crate) fn new<S: Spanned>(ident: &S) -> Result<Self> {
        let file = SourceFile::from_spanned(ident)?;
        if !matches!(file.ext, SourceExt::Rs) {
            return Err(syn::Error::new(
                ident.span(),
                "not found in source tree",
            ));
        }
        Ok(Self::from(file))
    }

    pub(crate) fn quot_mod(&self) -> TokenStream {
        let module = &self.module;
        let path = &self.path;
        quote::quote! {
            #[doc(hidden)]
            #[allow(
                non_upper_case_globals,
                unused_attributes,
                unused_qualifications,
                clippy::absolute_paths,
            )]
            #[path = #path]
            mod #module;
        }
    }

    fn is_updown(&self) -> bool {
        self.typ == SourceType::UpDown
    }

    fn _set_sql(&mut self, content: syn::LitStr) -> Result<()> {
        if self._sql_pair.replace(content).is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("found duplicate pair for {}", self.version),
            ));
        }
        Ok(())
    }

    // Some(false) means _this_ is the down migration.
    fn _has_sql_inverse(&self) -> Option<bool> {
        if self._sql_pair.is_none() {
            return None;
        }
        Some(self.typ == SourceType::Down)
    }
}

impl From<SourceFile> for RustSource {
    fn from(value: SourceFile) -> Self {
        let SourceFile {
            ident, path, version, description, module, typ, ..
        } = value;
        Self { ident, module, version, description, path, typ, _sql_pair: None }
    }
}

impl MigrationTokens for RustSource {
    fn mig_ident(&self) -> &syn::Ident {
        &self.ident
    }

    fn quot_ctx(&self, ident: &syn::Ident) -> TokenStream {
        quote::quote! { <#ident as ::tern::migration::ResolveQuery>::Ctx }
    }

    fn quot_migration_id(&self) -> TokenStream {
        let ver = self.version;
        let desc = &self.description;
        quote::quote! { ::tern::migration::MigrationId::new(#ver, #desc) }
    }

    fn quot_query_fn(&self, ident: &syn::Ident) -> TokenStream {
        quote::quote! {
            fn query<'a>(
                &'a self,
                ctx: &'a mut Self::Ctx,
            ) -> ::tern::private::BoxFuture<'a, ::tern::TernResult<::tern::migration::Query>>
            {
                ::std::boxed::Box::pin(async move {
                    let migration = <#ident as ::tern::migration::ResolveQuery>::init(ctx).await?;
                    <#ident as ::tern::migration::ResolveQuery>::resolve_query(&migration, ctx).await
                })
            }
        }
    }

    fn quot_revert_query_fn(&self, ident: &syn::Ident) -> TokenStream {
        // TODO(qcoh): https://github.com/quasi-coherent/tern/issues/46
        if !self.is_updown() {
            return Default::default();
        }
        quote::quote! {
            fn revert_query<'a>(
                &'a self,
                ctx: &'a mut Self::Ctx,
            ) -> ::tern::private::BoxFuture<'a, ::tern::TernResult<Option<::tern::migration::Query>>>
            {
                ::std::boxed::Box::pin(async move {
                    let migration = <#ident as ::tern::migration::ResolveQuery>::init(ctx).await?;
                    <#ident as ::tern::migration::ResolveRevertQuery>::resolve_revert_query(&migration, ctx).await
                })
            }
        }
    }
}
