use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, Stmt};

#[proc_macro_attribute]
pub fn req(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut req_ids = mantra_lang_tracing::extract::extract_req_ids(attr.into())
        .map_err(|err| panic!("{err}"))
        .unwrap();

    let mut attrbs: Vec<syn::Attribute> = vec![parse_quote!(#[doc = "# Requirements"])];

    for req in &req_ids {
        let req_literal = syn::LitStr::new(req, proc_macro2::Span::call_site());
        let attrb: syn::Attribute;

        if let Ok(url) = std::env::var("MANTRA_REQUIREMENT_BASE_URL") {
            let url_literal = syn::LitStr::new(&url, proc_macro2::Span::call_site());
            attrb = parse_quote!(#[doc = concat!("- [", #req_literal, "](", #url_literal, #req_literal, ")")]);
        } else {
            attrb = parse_quote!(#[doc = concat!("- ", #req_literal)]);
        }
        attrbs.push(attrb);
    }

    if let Ok(parsed_item) = syn::parse::<syn::Item>(item) {
        match parsed_item {
            syn::Item::Const(mut const_item) => {
                const_item.attrs.append(&mut attrbs);
                quote!(#const_item).into()
            }
            syn::Item::Enum(mut enum_item) => {
                enum_item.attrs.append(&mut attrbs);
                quote!(#enum_item).into()
            }
            syn::Item::ExternCrate(mut extern_item) => {
                extern_item.attrs.append(&mut attrbs);
                quote!(#extern_item).into()
            }
            syn::Item::Fn(mut fn_item) => {
                // reversed, because statements are inserted at block start
                req_ids.reverse();

                for req in req_ids {
                    let req_literal = syn::LitStr::new(&req, proc_macro2::Span::call_site());
                    let macro_stmt: Stmt =
                        parse_quote!(mantra_rust_macros::mr_reqcov!(#req_literal););

                    fn_item.block.stmts.insert(0, macro_stmt);
                }

                fn_item.attrs.append(&mut attrbs);

                quote!(#fn_item).into()
            }
            syn::Item::ForeignMod(mut foreign_item) => {
                foreign_item.attrs.append(&mut attrbs);
                quote!(#foreign_item).into()
            }
            syn::Item::Impl(mut impl_item) => {
                impl_item.attrs.append(&mut attrbs);
                quote!(#impl_item).into()
            }
            syn::Item::Macro(mut macro_item) => {
                macro_item.attrs.append(&mut attrbs);
                quote!(#macro_item).into()
            }
            syn::Item::Mod(mut mod_item) => {
                mod_item.attrs.append(&mut attrbs);
                quote!(#mod_item).into()
            }
            syn::Item::Static(mut static_item) => {
                static_item.attrs.append(&mut attrbs);
                quote!(#static_item).into()
            }
            syn::Item::Struct(mut struct_item) => {
                struct_item.attrs.append(&mut attrbs);
                quote!(#struct_item).into()
            }
            syn::Item::Trait(mut trait_item) => {
                trait_item.attrs.append(&mut attrbs);
                quote!(#trait_item).into()
            }
            syn::Item::TraitAlias(mut talias_item) => {
                talias_item.attrs.append(&mut attrbs);
                quote!(#talias_item).into()
            }
            syn::Item::Type(mut type_item) => {
                type_item.attrs.append(&mut attrbs);
                quote!(#type_item).into()
            }
            syn::Item::Union(mut union_item) => {
                union_item.attrs.append(&mut attrbs);
                quote!(#union_item).into()
            }
            syn::Item::Use(mut use_item) => {
                use_item.attrs.append(&mut attrbs);
                quote!(#use_item).into()
            }
            syn::Item::Verbatim(token) => {
                if let Ok(mut trait_item_type) = syn::parse::<syn::TraitItemType>(token.into()) {
                    trait_item_type.attrs.append(&mut attrbs);
                    quote!(#trait_item_type).into()
                } else {
                    panic!("`req` macro may only be used on Rust items. See: https://doc.rust-lang.org/reference/items.html")
                }
            }
            _ => panic!("`req` macro may only be used on Rust items. See: https://doc.rust-lang.org/reference/items.html"),
        }
    } else {
        panic!("`req` macro may only be used on Rust items. See: https://doc.rust-lang.org/reference/items.html")
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
