use syn::{parse_macro_input, DeriveInput};

mod parse;
mod query_builder;
mod runtime;

#[proc_macro_derive(QueryBuilder, attributes(migration))]
pub fn query_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    query_builder::expand(derive_input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Runtime, attributes(migration))]
pub fn migrate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let embed_input = parse_macro_input!(input as DeriveInput);
    runtime::expand(embed_input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
