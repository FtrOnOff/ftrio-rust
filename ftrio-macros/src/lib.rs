//! Procedural attribute macros that render FtrIO's attribute-based toggle natively in Rust.
//!
//! Rust has no runtime reflection or IL weaving, but it has procedural attribute macros, which are
//! compile-time code transformation. This is the *closest* analogue of the .NET AspectInjector
//! `[Toggle]`/`[ToggleAsync]` attributes of any target language — closer than Python's runtime
//! decorator — so the "attribute-based toggle is a must" requirement is met natively rather than by
//! substitution.
//!
//! `#[toggle]` wraps the original function body so it only runs when the toggle is on, and otherwise
//! returns `Default::default()` (the faithful equivalent of the woven aspect returning `default(T)`
//! and the Python wrapper returning `None`). The gating check panics on a misconfiguration error,
//! mirroring the woven aspect throwing an exception out of the decorated method; `ftrio lint` is the
//! build-time step meant to catch that before it can happen.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Expr, ExprLit, ItemFn, Lit, MetaNameValue, ReturnType, Type};

/// Extract the toggle key from the attribute arguments.
///
/// With no arguments the key defaults to the function's own name (Rust fns are `snake_case`, so the
/// derived key is `snake_case`, matching the Rust method-naming convention). An explicit override is
/// written `#[toggle(key = "SendWelcomeEmail")]`; an explicit string is a JSON key, not an
/// identifier, so it is used verbatim.
fn resolve_toggle_key(attr: TokenStream, function_name: &str) -> Result<String, syn::Error> {
    if attr.is_empty() {
        return Ok(function_name.to_string());
    }
    let name_value: MetaNameValue = syn::parse(attr)?;
    if !name_value.path.is_ident("key") {
        return Err(syn::Error::new_spanned(
            name_value.path,
            "expected `key = \"...\"`",
        ));
    }
    match name_value.value {
        Expr::Lit(ExprLit {
            lit: Lit::Str(key), ..
        }) => Ok(key.value()),
        other => Err(syn::Error::new_spanned(
            other,
            "the `key` value must be a string literal",
        )),
    }
}

/// `#[toggle]` on a synchronous function.
///
/// Expands, at compile time, to wrap the original body so the function runs only when
/// `get_toggle_status(<key>)` is on, and otherwise yields `Default::default()`. This imposes one
/// documented constraint: the return type must implement `Default`.
#[proc_macro_attribute]
pub fn toggle(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as ItemFn);
    let function_name = function.sig.ident.to_string();
    let toggle_key = match resolve_toggle_key(attr, &function_name) {
        Ok(key) => key,
        Err(error) => return error.to_compile_error().into(),
    };

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;
    let original_statements = &block.stmts;

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            if ::ftrio::toggle_parser_provider::instance()
                .get_toggle_status(#toggle_key)
                .unwrap()
            {
                #(#original_statements)*
            } else {
                ::core::default::Default::default()
            }
        }
    };
    expanded.into()
}

/// `#[toggle_async]` on an `async fn`.
///
/// The gating check must run synchronously *at call time* (before any async work begins), matching
/// the .NET woven `Around` advice and the Python async wrapper: a missing key or unparseable value
/// surfaces at the call site, not as a faulted future. So this rewrites `async fn f(..) -> T` into a
/// synchronous `fn f(..) -> impl Future<Output = T>` that performs the check eagerly and returns a
/// future resolving to either the original body's value or `T::default()`. Callers can `.await` the
/// result whether the toggle is on or off.
#[proc_macro_attribute]
pub fn toggle_async(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = parse_macro_input!(item as ItemFn);
    let function_name = function.sig.ident.to_string();
    let toggle_key = match resolve_toggle_key(attr, &function_name) {
        Ok(key) => key,
        Err(error) => return error.to_compile_error().into(),
    };

    let ItemFn {
        attrs,
        vis,
        mut sig,
        block,
    } = function;

    if sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            sig.fn_token,
            "#[toggle_async] can only be applied to an `async fn`",
        )
        .to_compile_error()
        .into();
    }

    // Reconstruct the signature as a synchronous fn returning `impl Future<Output = T>` so the
    // toggle check runs eagerly at call time rather than on first poll.
    let output_type: Type = match &sig.output {
        ReturnType::Default => syn::parse_quote!(()),
        ReturnType::Type(_, boxed) => (**boxed).clone(),
    };
    sig.asyncness = None;
    sig.output = syn::parse_quote!(
        -> impl ::core::future::Future<Output = #output_type>
    );

    let original_statements = &block.stmts;

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            let __ftrio_toggle_is_on = ::ftrio::toggle_parser_provider::instance()
                .get_toggle_status(#toggle_key)
                .unwrap();
            async move {
                if __ftrio_toggle_is_on {
                    #(#original_statements)*
                } else {
                    ::core::default::Default::default()
                }
            }
        }
    };
    expanded.into()
}
