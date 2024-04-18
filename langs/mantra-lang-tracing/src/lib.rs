use std::{path::Path, str::FromStr};

use proc_macro2::{Delimiter, TokenStream, TokenTree};

pub type ReqId = String;

pub struct ReqTrace {
    req_id: ReqId,
    line: u32,
}

impl ReqTrace {
    pub fn new(req_id: impl Into<ReqId>, line: u32) -> Self {
        Self {
            req_id: req_id.into(),
            line,
        }
    }

    pub fn req_id(&self) -> &str {
        &self.req_id
    }

    pub fn line(&self) -> &u32 {
        &self.line
    }
}

pub trait TraceCollector {
    fn collect(filepath: &Path) -> Vec<ReqTrace>;
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

#[cfg(test)]
mod test {
    use crate::extract_req_ids_from_str;

    #[test]
    fn single_req() {
        let req = "req_id";
        let reqs = extract_req_ids_from_str(req).unwrap();

        assert_eq!(
            &reqs.first().unwrap(),
            &req,
            "Single requirement ID not extracted correctly."
        );
        assert_eq!(
            reqs.len(),
            1,
            "More/Less than one requirement ID extracted."
        );
    }

    #[test]
    fn multiple_reqs() {
        let trace_content = "first_id, second_id";
        let reqs = extract_req_ids_from_str(trace_content).unwrap();

        assert_eq!(
            reqs.first().unwrap(),
            "first_id",
            "First requirement ID not extracted correctly."
        );
        assert_eq!(
            reqs.last().unwrap(),
            "second_id",
            "Second requirement ID not extracted correctly."
        );
        assert_eq!(
            reqs.len(),
            2,
            "More/Less than two requirement ID extracted."
        );
    }

    #[test]
    fn quoted_id() {
        let trace_content = "\"req-id.sub-req\"";
        let reqs = extract_req_ids_from_str(trace_content).unwrap();

        assert_eq!(
            reqs.first().unwrap(),
            "req-id.sub-req",
            "Quoted requirement ID not extracted correctly."
        );
        assert_eq!(
            reqs.len(),
            1,
            "More/Less than one requirement ID extracted."
        );
    }

    #[test]
    fn invalid_id() {
        let trace_content = "invalid`char";
        let reqs = extract_req_ids_from_str(trace_content);

        assert!(reqs.is_err(), "Invalid char in ID extracted without error.");
    }
}
