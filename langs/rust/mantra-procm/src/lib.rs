use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn req(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "defmt")]
    return defmt_wrap(item);

    #[cfg(not(feature = "defmt"))]
    item
}

#[proc_macro_attribute]
pub fn req_satisfied(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "defmt")]
    return defmt_wrap(item);

    #[cfg(not(feature = "defmt"))]
    item
}

#[proc_macro_attribute]
pub fn req_verified(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "defmt")]
    return defmt_wrap(item);

    #[cfg(not(feature = "defmt"))]
    item
}

#[proc_macro_attribute]
pub fn req_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "defmt")]
    return defmt_wrap(item);

    #[cfg(not(feature = "defmt"))]
    item
}

#[proc_macro_attribute]
pub fn req_note(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "defmt")]
    return defmt_wrap(item);

    #[cfg(not(feature = "defmt"))]
    item
}

#[proc_macro_attribute]
pub fn req_link(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "defmt")]
    return defmt_wrap(item);

    #[cfg(not(feature = "defmt"))]
    item
}

#[cfg(feature = "defmt")]
/// Create a defmnt log to use as line coverage marker for embedded testing
fn defmt_wrap(item: TokenStream) -> TokenStream {
    if let Ok(parsed_item) = syn::parse::<syn::Item>(item.clone()) {
        match parsed_item {
            syn::Item::Fn(mut fn_item) => {
                let macro_stmt: syn::Stmt = syn::parse_quote!(mantra_macros::_line_coverage!(););

                fn_item.block.stmts.insert(0, macro_stmt);

                quote::quote!(#fn_item).into()
            }
            _ => item,
        }
    } else {
        item
    }
}
