use std::str::FromStr;

use mantra_schema::{requirements::ReqId, traces::TraceEntry, Line};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use regex::Regex;

use crate::RawTraceEntry;

impl TryFrom<RawTraceEntry<'_>> for TraceEntry {
    type Error = String;

    fn try_from(value: RawTraceEntry) -> Result<Self, Self::Error> {
        let ids = extract_req_ids_from_str(value.ids).map_err(|err| err.to_string())?;
        let line = value
            .line
            .try_into()
            .map_err(|err: <Line as std::convert::TryFrom<usize>>::Error| err.to_string())?;
        let line_span = value.line_span;

        Ok(Self {
            ids,
            line,
            line_span,
            item_name: None,
        })
    }
}

static REQ_TRACE_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

pub fn req_trace_matcher() -> &'static Regex {
    REQ_TRACE_MATCHER.get_or_init(|| {
        Regex::new(r"\[(?:[^\(]+::)?(?:req|requirements)\((?<ids>[^\)]+)\)\]")
            .expect("Regex to match a requirement trace could **not** be created.")
    })
}

pub fn extract_req_ids_from_str(s: &str) -> Result<Vec<ReqId>, String> {
    let tokens = TokenStream::from_str(s).map_err(|_| {
        format!("Given requirement IDs '{s}' contain one or more invalid characters.")
    })?;
    extract_req_ids(tokens)
}

pub fn extract_req_ids(input: TokenStream) -> Result<Vec<ReqId>, String> {
    let mut req_ids = Vec::new();
    let mut req_part = String::new();

    for token in input.into_iter() {
        match token {
            TokenTree::Group(group) => {
                return Err(format!(
                    "Keyword '{}' not allowed as part of a requirement ID.",
                    match group.delimiter() {
                        Delimiter::Parenthesis => "()",
                        Delimiter::Brace => "{}",
                        Delimiter::Bracket => "[]",
                        Delimiter::None => "invisible delimiter",
                    }
                ))
            }
            TokenTree::Ident(id) => {
                req_part.push_str(&id.to_string());
            }
            TokenTree::Punct(punct) => {
                let c = punct.as_char();
                match c {
                    '.' => {
                        if req_part.is_empty() {
                            return Err("No requirement ID part found before '.'. IDs must not start with '.'.".to_string());
                        }

                        req_part.push(c);
                    }
                    ',' => {
                        if req_part.is_empty() {
                            return Err("No requirement ID found before ','.".to_string());
                        } else if req_part.ends_with('.') {
                            return Err(format!(
                                "Requirement ID '{}' must not end with '.'.",
                                req_part
                            ));
                        }

                        req_ids.push(std::mem::take(&mut req_part));
                    }
                    '"' | '`' => {
                        return Err("Requirement IDs must not contain '\"', or '`'.".to_string())
                    }
                    _ => {
                        req_part.push(c);
                    }
                }
            }
            TokenTree::Literal(literal) => {
                let mut literal_str = literal.to_string();

                literal_str = literal_str
                    .strip_prefix('"')
                    .map(|s| s.strip_suffix('"').unwrap_or(s).to_string())
                    .unwrap_or(literal_str);

                if literal_str.contains(['"', '`']) {
                    return Err("Requirement IDs must not contain '\"', or '`'.".to_string());
                } else if literal_str.ends_with('.') {
                    return Err("Quoted IDs must not end with '.'.".to_string());
                }

                req_part.push_str(&literal_str);
            }
        }
    }

    if !req_part.is_empty() {
        if req_part.ends_with('.') {
            return Err("Quoted IDs must not end with '.'.".to_string());
        }
        req_ids.push(req_part);
    }

    Ok(req_ids)
}
