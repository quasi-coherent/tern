use proc_macro2::TokenStream;

mod container;
pub(crate) use container::*;

mod parse;
pub(crate) use parse::*;

/// Type that can produce the token stream of an implementation of `Migration`.
///
/// For a .rs migration `ident` in the method signatures is the type that
/// derives `Migration`, as this is constructed by that derive macro rather than
/// `TernApp`.  For .sql, `ident` is the type deriving `TernApp` because there
/// is no other type.
pub(crate) trait MigrationTokens {
    /// The name of the type that implements `Migration`.
    fn mig_ident(&self) -> &syn::Ident;

    /// Token stream with an expression creating a `MigrationId` value.
    fn quot_migration_id(&self) -> TokenStream;

    /// The `Ctx` type expression in `MigrationBox<Ctx>`.
    fn quot_ctx(&self, ident: &syn::Ident) -> TokenStream;

    /// The token stream implementing `Migration::query`.
    fn quot_query_fn(&self, ident: &syn::Ident) -> TokenStream;

    /// The token stream implementing `Migration::revert_query`.
    fn quot_revert_query_fn(&self, ident: &syn::Ident) -> TokenStream;

    /// Token stream with the expression calling the `migration` method exported
    /// for this migration's module.
    fn quot_boxed(&self) -> TokenStream {
        let this = self.mig_ident();
        quote::quote! { #this::migration() }
    }

    /// Provided by putting the others together.
    fn quot_impl_migration(&self, ident: &syn::Ident) -> TokenStream {
        let this = self.mig_ident();
        let mid = self.quot_migration_id();
        let ctx = self.quot_ctx(ident);
        let up = self.quot_query_fn(ident);
        let down = self.quot_revert_query_fn(ident);
        quote::quote! {
            pub(super) struct #this;

            impl #this {
                pub(super) fn migration() -> ::tern::migration::MigrationBox<#ctx> {
                    ::tern::migration::MigrationBox::new(#this)
                }
            }

            static ___MIGRATION_ID: ::std::sync::LazyLock<::tern::migration::MigrationId> =
                ::std::sync::LazyLock::new(|| #mid);

            #[automatically_derived]
            impl ::tern::Migration for #this {
                type Ctx = #ctx;
                fn migration_id(&self) -> &::tern::migration::MigrationId {
                    &*___MIGRATION_ID
                }
                #up
                #down
            }
        }
    }
}
