use syn::{DeriveInput, parse_macro_input};

mod internal;
mod migration;
mod tern_app;

#[proc_macro_derive(Migration, attributes(tern))]
pub fn migration(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    migration::expand_impl_migration(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(TernApp, attributes(tern))]
pub fn tern_app(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    tern_app::expand_impl_tern(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
