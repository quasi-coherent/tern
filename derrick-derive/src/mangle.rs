use proc_macro2::TokenStream;
use quote::quote;

pub fn wrap_in_mod(module: &syn::Ident, code: TokenStream) -> TokenStream {
    let path = quote! {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate derrick as _derrick;
        #[allow(unused_imports)]
        use super::*;
    };
    quote! {
        pub mod #module {
            #path
            #code
        }
    }
}
