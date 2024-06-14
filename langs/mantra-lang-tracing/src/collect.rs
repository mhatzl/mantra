use tree_sitter::{Language, Parser, Tree};

use crate::RawTraceEntry;

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
        let trace_matcher = crate::extract::req_trace_matcher();
        let mut traces = Vec::new();
        let lines = self.src.lines();

        for (i, line_content) in lines.enumerate() {
            for capture in trace_matcher.captures_iter(line_content) {
                traces.push(
                    TraceEntry::try_from(RawTraceEntry::new(
                        capture.name("ids")?.as_str(),
                        i + 1,
                        None,
                    ))
                    .ok()?,
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

// re-export types used in collector_fn for fewer dependencies for implementors
pub use mantra_schema::traces::LineSpan;
pub use mantra_schema::traces::TraceEntry;
pub use mantra_schema::Line;
pub use tree_sitter::Node as AstNode;

pub type AstCollectorFn<'a, T> = Box<dyn FnMut(&AstNode, &'a [u8], &T) -> Option<Vec<TraceEntry>>>;

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
