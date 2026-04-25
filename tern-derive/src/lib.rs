use syn::{DeriveInput, parse_macro_input};

mod internal;
mod migration;
mod tern_migrate;

#[proc_macro_derive(Migration, attributes(tern))]
pub fn migration(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    migration::expand_impl_migration(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(TernMigrate, attributes(tern))]
pub fn migration_context(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    tern_migrate::expand_impl_tern_migrate(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
