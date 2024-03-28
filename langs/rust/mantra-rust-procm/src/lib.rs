use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn, Stmt};

#[proc_macro_attribute]
pub fn req(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut wrapped_fn: ItemFn = parse_macro_input!(item);

    let mut req_ids = extract_req_ids(attr);
    req_ids.reverse();

    for req in req_ids {
        let req_literal = syn::LitStr::new(&req, proc_macro2::Span::call_site());
        let macro_stmt: Stmt = parse_quote!(mantra_rust_macros::mr_reqcov!(#req_literal););

        wrapped_fn.block.stmts.insert(0, macro_stmt);
    }

    quote!(#wrapped_fn).into()
}

#[proc_macro]
pub fn reqcov(input: TokenStream) -> TokenStream {
    let req_ids = extract_req_ids(input);

    let mut stream = TokenStream::new();

    for req in req_ids {
        let req_literal = syn::LitStr::new(&req, proc_macro2::Span::call_site());
        stream.extend::<TokenStream>(quote!(mantra_rust_macros::mr_reqcov!(#req_literal);).into())
    }

    stream
}

fn extract_req_ids(input: TokenStream) -> Vec<String> {
    let mut req_ids = Vec::new();
    let mut req_part = String::new();
    let mut prev_was_punct = false;

    for token in input.into_iter() {
        match token {
            proc_macro::TokenTree::Group(group) => panic!(
                "Invalid keyword '{}'. Grouping requirement IDs is not supported.",
                match group.delimiter() {
                    proc_macro::Delimiter::Parenthesis => "()",
                    proc_macro::Delimiter::Brace => "{}",
                    proc_macro::Delimiter::Bracket => "[]",
                    proc_macro::Delimiter::None => "invisible delimiter",
                }
            ),
            proc_macro::TokenTree::Ident(id) => {
                if !req_part.is_empty() && !prev_was_punct {
                    panic!("ID parts must be separated by '-' or '.'.");
                }
                prev_was_punct = false;
                req_part.push_str(&id.to_string());
            }
            proc_macro::TokenTree::Punct(punct) => {
                let c = punct.as_char();
                match c {
                    '.' | '-' => {
                        if req_part.is_empty() {
                            panic!("No requirement ID part found before '{c}'. IDs must not start with '-' or '.'.");
                        }
                        prev_was_punct = true;
                        req_part.push(c);
                    }
                    ',' => {
                        if req_part.is_empty() {
                            panic!("No requirement ID found before ','.");
                        } else if prev_was_punct {
                            panic!("ID must not end with '-' or '.'.");
                        }

                        req_ids.push(std::mem::take(&mut req_part));
                    }
                    _ => {
                        panic!("Invalid punctuation '{c}'. Use '.' for nested requirement IDs, or ',' to set multiple IDs.")
                    }
                }
            }
            proc_macro::TokenTree::Literal(literal) => {
                if !req_part.is_empty() && !prev_was_punct {
                    panic!("ID parts must be separated by '-' or '.'.");
                }
                let literal_str = literal.to_string();

                if literal_str.contains('.') {
                    panic!("Quoted strings or numbers must not contain '.', because '.' is used for nested requirements.");
                }

                prev_was_punct = false;
                req_part.push_str(&literal_str.replace('"', ""));
            }
        }
    }

    if !req_part.is_empty() {
        req_ids.push(req_part);
    }

    req_ids
}
