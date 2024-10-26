use syn::{parse_macro_input, DeriveInput};

mod derive;
mod embedding;
mod mangle;
mod parse;

#[proc_macro_derive(QueryBuilder, attributes(migration))]
pub fn query_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    derive::expand(derive_input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn embed_migrations(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let embed_input = parse_macro_input!(input as parse::EmbedInput);
    embedding::expand(embed_input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
