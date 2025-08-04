use syn::{parse_macro_input, DeriveInput};

mod internal;
mod quote;

#[proc_macro_derive(Migration, attributes(tern))]
pub fn migration(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quote::expand_impl_migration(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(MigrationContext, attributes(tern))]
pub fn migration_context(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quote::expand_impl_migration_context(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
