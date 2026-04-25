use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Migration, attributes(tern))]
pub fn migration(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _ = parse_macro_input!(input as DeriveInput);
    (quote::quote! { }).into()
}

#[proc_macro_derive(MigrationContext, attributes(tern))]
pub fn migration_context(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _ = parse_macro_input!(input as DeriveInput);
    (quote::quote! { }).into()
}
