use proc_macro2::{Span, TokenStream};
use syn::Result;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

use crate::internal::SourceMap;

pub fn expand_test(args: ParsedArgs, item: syn::ItemFn) -> Result<TokenStream> {
    if args.source.is_some() || args.properties.is_some() {
        return Err(syn::Error::new(
            Span::call_site(),
            "`source` and `properties` are arguments of \
             `tern::test_suite!`, not `#[tern::test]`",
        ));
    }
    let app = args.require_app()?;
    let context = args.require_context()?;
    if item.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            item.sig.fn_token,
            "#[tern::test] requires an `async fn`",
        ));
    }
    if item.sig.inputs.len() != 1 {
        return Err(syn::Error::new_spanned(
            &item.sig.inputs,
            "#[tern::test] requires exactly one argument, \
             `&mut tern::test::TernTest<_>`",
        ));
    }
    let attrs = &item.attrs;
    let vis = &item.vis;
    let name = &item.sig.ident;
    let name_str = name.to_string();
    let cfg = args.quot_config(quote::quote! { #name_str })?;
    let inner = syn::ItemFn {
        attrs: Vec::new(),
        vis: syn::Visibility::Inherited,
        sig: syn::Signature {
            ident: syn::Ident::new("__tern_test_impl", Span::call_site()),
            ..item.sig.clone()
        },
        block: item.block.clone(),
    };
    Ok(quote::quote! {
        #(#attrs)*
        #[::core::prelude::v1::test]
        #vis fn #name() {
            #inner
            ::tern::private::run_test::<#app, _, _, _>(
                #cfg,
                #context,
                __tern_test_impl,
            )
        }
    })
}

pub fn expand_test_suite(args: ParsedArgs) -> Result<TokenStream> {
    let app = args.require_app()?;
    let context = args.require_context()?;
    let source = args.source.as_ref().ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "missing required argument `source = \"<migration dir>\"` \
             (the same path as the app's `#[tern(source = ...)]`)",
        )
    })?;
    // Runs the same validation as `#[derive(TernApp)]`: contiguous
    // versions, matched U/D pairs.
    let map = SourceMap::new(source)?;
    if !map.is_updown() && args.properties.is_some() {
        return Err(syn::Error::new(
            source.span(),
            "`properties` requires an up/down migration source; a simple \
             (V-prefixed) source has nothing to revert",
        ));
    }
    let props = args
        .properties
        .as_ref()
        .map(|p| quote::quote! { #p })
        .unwrap_or_else(|| quote::quote! { () });

    let apply_all = quote::quote! {
        #[::core::prelude::v1::test]
        fn apply_all() {
            ::tern::private::run_test_case::<#app, _, _, _, _>(
                __cfg("apply_all"),
                #context,
                ::tern::private::TestCase::ApplyAll,
                || #props,
            )
        }
    };
    let updown = map
        .updown_pairs()
        .map(|(version, desc)| {
            let test_name = quote::format_ident!("updown_v{version}_{desc}");
            let name_str = test_name.to_string();
            quote::quote! {
                #[::core::prelude::v1::test]
                fn #test_name() {
                    ::tern::private::run_test_case::<#app, _, _, _, _>(
                        __cfg(#name_str),
                        #context,
                        ::tern::private::TestCase::UpDown { version: #version },
                        || #props,
                    )
                }
            }
        })
        .collect::<TokenStream>();

    let cfg = args.quot_config(quote::quote! { name })?;
    let mod_ident = args
        .app
        .as_ref()
        .and_then(|p| p.segments.last())
        .map(|seg| quote::format_ident!("__tern_suite_{}", seg.ident))
        .ok_or_else(|| {
            syn::Error::new(Span::call_site(), "`app` must be a type path")
        })?;
    Ok(quote::quote! {
        #[allow(non_snake_case)]
        mod #mod_ident {
            use super::*;

            fn __cfg(name: &str) -> ::tern::private::TestConfig {
                #cfg
            }

            #apply_all
            #updown
        }
    })
}

/// The collected arguments of a test macro invocation.
#[derive(Default)]
pub struct ParsedArgs {
    app: Option<syn::Path>,
    context: Option<syn::Expr>,
    env: Option<syn::LitStr>,
    source: Option<syn::LitStr>,
    properties: Option<syn::Expr>,
    keep_db: bool,
}

impl Parse for ParsedArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Punctuated::<TestArg, syn::Token![,]>::parse_terminated(input).map(
            |args| {
                args.into_iter().fold(Self::default(), |acc, arg| acc.set(arg))
            },
        )
    }
}

impl ParsedArgs {
    fn set(self, arg: TestArg) -> Self {
        match arg {
            TestArg::App(v) => Self { app: Some(v), ..self },
            TestArg::Context(v) => Self { context: Some(v), ..self },
            TestArg::Env(v) => Self { env: Some(v), ..self },
            TestArg::Source(v) => Self { source: Some(v), ..self },
            TestArg::Properties(v) => Self { properties: Some(v), ..self },
            TestArg::KeepDb => Self { keep_db: true, ..self },
        }
    }

    fn require_app(&self) -> Result<&syn::Path> {
        self.app.as_ref().ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "missing required argument `app = <the TernApp type>`",
            )
        })
    }

    fn require_context(&self) -> Result<&syn::Expr> {
        self.context.as_ref().ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "missing required argument `context`, \
                 which is anything of type `FnMut(T::Exec) -> Fut`, \
                 for a future returning `TernResult<T>`",
            )
        })
    }

    /// Expression building the `TestConfig`, where `name` is an expression
    /// for the test's name.
    fn quot_config(&self, name: TokenStream) -> Result<TokenStream> {
        let key = match &self.env {
            Some(env) => quote::quote! { ::std::option::Option::Some(#env) },
            _ => quote::quote! { ::std::option::Option::None },
        };
        let keep = self
            .keep_db
            .then(|| quote::quote! { .keep_db() })
            .unwrap_or_default();
        Ok(quote::quote! {
            ::tern::private::TestConfig::new(#name, #key) #keep
        })
    }
}

enum TestArg {
    App(syn::Path),
    Context(syn::Expr),
    Env(syn::LitStr),
    Source(syn::LitStr),
    Properties(syn::Expr),
    KeepDb,
}

impl Parse for TestArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let name = ident.to_string();
        if name == "keep_db" {
            return Ok(Self::KeepDb);
        }
        input.parse::<syn::Token![=]>()?;
        match name.as_str() {
            "app" => input.parse().map(Self::App),
            "context" => input.parse().map(Self::Context),
            "env" => input.parse().map(Self::Env),
            "source" => input.parse().map(Self::Source),
            "properties" => input.parse().map(Self::Properties),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown argument `{other}`: expected one of `app`, \
                     `context`, `env`, source`, `properties`, `keep_db`"
                ),
            )),
        }
    }
}
