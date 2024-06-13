use mantra_lang_tracing::{Line, LineSpan, TraceEntry};
use tree_sitter::Node;

pub fn collect_traces_in_rust(node: &Node, src: &[u8], _args: &()) -> Option<Vec<TraceEntry>> {
    let node_kind = node.kind();

    if node_kind == "attribute_item" || node_kind == "macro_invocation" {
        let (macro_node, may_span) = if node_kind == "macro_invocation" {
            (*node, false)
        } else {
            let attribute_node = node.named_child(0)?;
            (attribute_node, true)
        };

        let ident = macro_node.named_child(0)?;
        let macro_content = macro_node.named_child(1)?;

        if ident.kind() == "identifier"
            && ident.utf8_text(src) == Ok("req")
            && macro_content.kind() == "token_tree"
        {
            let span = if may_span {
                associated_item_span(*node)
            } else {
                None
            };

            return Some(vec![TraceEntry::try_from((
                macro_content
                    .utf8_text(src)
                    .ok()?
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))?,
                (ident.start_position().row + 1),
                span,
            ))
            .ok()?]);
        }
    } else if node_kind == "line_comment" && is_doc_comment(node) {
        let trace_matcher = mantra_lang_tracing::req_trace_matcher();
        let comment_content = node.utf8_text(src).ok()?;

        let captures: Vec<_> = trace_matcher.captures_iter(comment_content).collect();

        if !captures.is_empty() {
            let span = associated_item_span(*node);

            let mut traces = Vec::new();
            for capture in captures {
                traces.push(
                    TraceEntry::try_from((
                        capture.name("ids")?.as_str(),
                        (node.start_position().row + 1),
                        span,
                    ))
                    .ok()?,
                )
            }

            return Some(traces);
        }
    }

    None
}

fn associated_item_span(mut node: Node) -> Option<LineSpan> {
    while let Some(sibling) = node.next_named_sibling() {
        let sibling_kind = sibling.kind();

        if sibling_kind.ends_with("_item") {
            let start = Line::try_from(sibling.start_position().row + 1).ok()?;
            let end = Line::try_from(sibling.end_position().row + 1).ok()?;

            return Some(LineSpan::new(start, end));
        } else if sibling_kind.ends_with("comment") && !is_doc_comment(&sibling) {
            return None;
        }

        node = sibling;
    }

    None
}

fn is_doc_comment(node: &Node) -> bool {
    if let Some(doc_node) = node.named_child(1) {
        doc_node.kind() == "doc_comment"
    } else {
        false
    }
}
