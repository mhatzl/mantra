use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, ItemFn, Stmt};

#[proc_macro_attribute]
pub fn req(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(mut wrapped_fn) = syn::parse::<ItemFn>(item.clone()) {
        let mut req_ids = mantra_lang_tracing::extract::extract_req_ids(attr.into())
            .map_err(|err| panic!("{err}"))
            .unwrap();
        req_ids.reverse();

        for req in req_ids {
            let req_literal = syn::LitStr::new(&req, proc_macro2::Span::call_site());
            let macro_stmt: Stmt = parse_quote!(mantra_rust_macros::mr_reqcov!(#req_literal););

            wrapped_fn.block.stmts.insert(0, macro_stmt);
        }

        quote!(#wrapped_fn).into()
    } else {
        // specifying `req` macro is possible, but only fns generate logs for now
        item
    }
}

#[proc_macro]
pub fn reqcov(input: TokenStream) -> TokenStream {
    let req_ids = mantra_lang_tracing::extract::extract_req_ids(input.into())
        .map_err(|err| panic!("{err}"))
        .unwrap();

    let mut stream = TokenStream::new();

    for req in req_ids {
        let req_literal = syn::LitStr::new(&req, proc_macro2::Span::call_site());
        stream.extend::<TokenStream>(quote!(mantra_rust_macros::mr_reqcov!(#req_literal);).into())
    }

    stream
}
