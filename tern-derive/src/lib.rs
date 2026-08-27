use syn::{DeriveInput, parse_macro_input};

mod ast;
mod rs;
mod sql;
mod quot;

#[proc_macro_derive(Migration, attributes(tern))]
pub fn migration(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quot::expand_impl_migration(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(TernApp, attributes(tern))]
pub fn tern_app(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quot::expand_impl_tern_app(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(TernTest, attributes(tern))]
pub fn tern_test(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    quot::expand_impl_tern_test(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
