use proc_macro2::TokenStream;
use syn::Result;

use crate::ast::*;

/// The combination of an up/down pair of .sql files.
pub(crate) struct SqlSource {
    ident: syn::Ident,
    module: syn::Ident,
    version: i64,
    description: syn::LitStr,
    up_content: syn::LitStr,
    down_content: Option<syn::LitStr>,
    modified: syn::LitInt,
}

impl SqlSource {
    fn new(up: SourceFile) -> Self {
        Self {
            ident: up.ident,
            module: up.module,
            version: up.version,
            description: up.description,
            up_content: up.content,
            down_content: None,
            modified: up.modified,
        }
    }

    pub(crate) fn combine(left: SourceFile, right: SourceFile) -> Result<Self> {
        if left.version != right.version
            || [left.typ, right.typ].contains(&SourceType::Simple)
        {
            return Err(syn::Error::new(
                left.ident.span(),
                "invalid up/down pair",
            ));
        }
        match (left.is_down(), right.is_down()) {
            (Some(true), Some(false)) => {
                let this = Self::new(right).set_down_content(left.content);
                Ok(this)
            },
            (Some(false), Some(true)) => {
                let this = Self::new(left).set_down_content(right.content);
                Ok(this)
            },
            _ => {
                Err(syn::Error::new(left.ident.span(), "missing up/down pair"))
            },
        }
    }

    fn set_down_content(mut self, content: syn::LitStr) -> Self {
        self.down_content.replace(content);
        self
    }

    pub(crate) fn quot_mod(&self, ident: &syn::Ident) -> TokenStream {
        let impl_mig = self.quot_impl_migration(ident);
        let SqlSource { modified, module, .. } = self;
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
                #impl_mig
            }
        }
    }
}

impl MigrationTokens for SqlSource {
    fn mig_ident(&self) -> &syn::Ident {
        &self.ident
    }

    fn quot_ctx(&self, ident: &syn::Ident) -> TokenStream {
        quote::quote! { super::#ident }
    }

    fn quot_migration_id(&self) -> TokenStream {
        let ver = self.version;
        let desc = &self.description;
        quote::quote! { ::tern::migration::MigrationId::new(#ver, #desc) }
    }

    fn quot_query_fn(&self, _: &syn::Ident) -> TokenStream {
        let up_cont = &self.up_content;
        quote::quote! {
            fn query<'a>(
                &'a self,
                ctx: &'a mut Self::Ctx,
            ) -> ::tern::private::BoxFuture<'a, ::tern::TernResult<::tern::migration::Query>>
            {
                let query = ::tern::migration::Query::from_sql(#up_cont);
                let fut = ::core::futures::ready(query);
                ::std::boxed::Box::pin(fut)
            }
        }
    }

    fn quot_revert_query_fn(&self, _: &syn::Ident) -> TokenStream {
        let Some(down_cont) = self.down_content.as_ref() else {
            return Default::default();
        };
        quote::quote! {
            fn revert_query<'a>(
                &'a self,
                ctx: &'a mut Self::Ctx,
            ) -> ::tern::private::BoxFuture<'a, ::tern::TernResult<Option<::tern::migration::Query>>>
            {
                let query = ::tern::migration::Query::from_sql(#down_cont);
                let fut = ::core::futures::ready(query);
                ::std::boxed::Box::pin(fut)
            }
        }
    }
}
