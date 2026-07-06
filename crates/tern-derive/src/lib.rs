use syn::{DeriveInput, parse_macro_input};

mod internal;
mod migration;
mod tern_app;
mod testing;

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

#[proc_macro_attribute]
pub fn test(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(attr as testing::ParsedArgs);
    let item = parse_macro_input!(item as syn::ItemFn);
    testing::expand_test(args, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn test_suite(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(input as testing::ParsedArgs);
    testing::expand_test_suite(args)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
