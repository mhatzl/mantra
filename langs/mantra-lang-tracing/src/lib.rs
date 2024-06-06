use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use regex::Regex;
use tree_sitter::{Language, Node, Parser, Tree};

pub mod path;

pub type ReqId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LineSpan {
    start: u32,
    end: u32,
}

impl LineSpan {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
    pub fn start(&self) -> u32 {
        self.start
    }

    pub fn end(&self) -> u32 {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TraceEntry {
    ids: Vec<ReqId>,
    /// The line the trace is defined
    line: u32,
    /// Optional span of lines this entry affects in the source.
    ///
    /// e.g. lines of a function body for a trace set at start of the function.
    line_span: Option<LineSpan>,
}

impl TraceEntry {
    pub fn new(ids: Vec<ReqId>, line: u32, line_span: Option<LineSpan>) -> Self {
        Self {
            ids,
            line,
            line_span,
        }
    }

    pub fn ids(&self) -> &[ReqId] {
        &self.ids
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn line_span(&self) -> &Option<LineSpan> {
        &self.line_span
    }
}

impl TryFrom<(&str, usize, Option<LineSpan>)> for TraceEntry {
    type Error = String;

    fn try_from(value: (&str, usize, Option<LineSpan>)) -> Result<Self, Self::Error> {
        let ids = extract_req_ids_from_str(value.0).map_err(|err| err.to_string())?;
        let line = value
            .1
            .try_into()
            .map_err(|err: <u32 as std::convert::TryFrom<usize>>::Error| err.to_string())?;
        let line_span = value.2;

        Ok(Self {
            ids,
            line,
            line_span,
        })
    }
}

impl std::fmt::Display for TraceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "req({}) at '{}'", self.ids.join(","), self.line)?;

        if let Some(span) = self.line_span {
            write!(f, " spans lines '{}:{}'", span.start, span.end)?;
        }

        Ok(())
    }
}

pub trait TraceCollector<T> {
    fn collect(&mut self, collect_arg: &T) -> Option<Vec<TraceEntry>>;
}

pub struct PlainCollector<'a> {
    src: &'a str,
}

impl<'a> PlainCollector<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src }
    }
}

impl TraceCollector<()> for PlainCollector<'_> {
    fn collect(&mut self, _collect_arg: &()) -> Option<Vec<TraceEntry>> {
        let trace_matcher = req_trace_matcher();
        let mut traces = Vec::new();
        let lines = self.src.lines();

        for (i, line_content) in lines.enumerate() {
            for capture in trace_matcher.captures_iter(line_content) {
                traces.push(
                    TraceEntry::try_from((capture.name("ids")?.as_str(), (i + 1), None)).ok()?,
                )
            }
        }

        Some(traces)
    }
}

pub struct AstCollector<'a, T> {
    tree: Tree,
    src: &'a [u8],
    collect_fn: AstCollectorFn<'a, T>,
}

pub type AstCollectorFn<'a, T> = Box<dyn FnMut(&Node, &'a [u8], &T) -> Option<Vec<TraceEntry>>>;

impl<'a, T> AstCollector<'a, T> {
    pub fn new(src: &'a [u8], lang: &Language, collect_fn: AstCollectorFn<'a, T>) -> Option<Self> {
        let mut parser = Parser::new();

        parser.set_language(lang).ok()?;

        let tree = parser.parse(src, None)?;

        Some(Self {
            tree,
            src,
            collect_fn,
        })
    }
}

impl<'a, T> TraceCollector<T> for AstCollector<'a, T> {
    fn collect(&mut self, collect_arg: &T) -> Option<Vec<TraceEntry>> {
        let mut cursor = self.tree.walk();
        let mut traces = Vec::new();
        let mut traces_extracted = false;

        // top down traversal
        'outer: loop {
            let has_child = if traces_extracted {
                // to not further travers already extracted nodes
                false
            } else {
                cursor.goto_first_child()
            };
            traces_extracted = false;

            if !has_child {
                let has_sibling = cursor.goto_next_sibling();
                if !has_sibling {
                    let mut has_next_upper = false;

                    while !has_next_upper {
                        if !cursor.goto_parent() {
                            break 'outer;
                        }

                        has_next_upper = cursor.goto_next_sibling();
                    }
                }
            }

            let node = cursor.node();

            if let Some(mut extracted_traces) = (self.collect_fn)(&node, self.src, collect_arg) {
                traces.append(&mut extracted_traces);
                traces_extracted = true;
            }
        }

        if traces.is_empty() {
            None
        } else {
            Some(traces)
        }
    }
}

static REQ_TRACE_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

pub fn req_trace_matcher() -> &'static Regex {
    REQ_TRACE_MATCHER.get_or_init(|| {
        Regex::new(r"\[req\((?<ids>[^\)]+)\)\]")
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
