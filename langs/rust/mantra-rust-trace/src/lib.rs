use mantra_lang_tracing::{
    collect::{AstNode, Line, LineSpan, TraceEntry},
    lsif_graph::LsifGraph,
    RawTraceEntry,
};

pub fn collect_traces_in_rust(
    node: &AstNode,
    src: &[u8],
    filepath: &str,
    lsif_graphs: &Option<Vec<LsifGraph>>,
) -> Option<Vec<TraceEntry>> {
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
            && ident.utf8_text(src).map(is_req_macro).unwrap_or(false)
            && macro_content.kind() == "token_tree"
        {
            let span = if may_span {
                associated_item_span(*node)
            } else {
                None
            };

            return Some(vec![TraceEntry::try_from(RawTraceEntry::new(
                macro_content
                    .utf8_text(src)
                    .ok()?
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))?,
                ident.start_position().row + 1,
                span,
                get_ident(filepath, span, lsif_graphs.as_deref()),
            ))
            .ok()?]);
        } else if ident.kind() == "identifier" && ident.utf8_text(src) == Ok("cfg_attrb") {
            let mut traces = Vec::new();

            let span = if may_span {
                associated_item_span(*node)
            } else {
                None
            };
            let start_line = ident.start_position().row + 1;

            for child in macro_content.named_children(&mut macro_content.walk()) {
                if child.kind() == "identifier"
                    && child.utf8_text(src).map_or(false, is_req_macro)
                    && child
                        .next_named_sibling()
                        .map_or(false, |n| n.kind() == "token_tree")
                {
                    let ids = child
                        .next_named_sibling()
                        .expect("Sibling checked in condition")
                        .utf8_text(src)
                        .ok()?
                        .strip_prefix('(')
                        .and_then(|s| s.strip_suffix(')'))?;
                    if let Ok(entry) = TraceEntry::try_from(RawTraceEntry::new(
                        ids,
                        start_line,
                        span,
                        get_ident(filepath, span, lsif_graphs.as_deref()),
                    )) {
                        traces.push(entry);
                    }
                }
            }

            return if traces.is_empty() {
                None
            } else {
                Some(traces)
            };
        }
    } else if node_kind == "line_comment" && is_doc_comment(node) {
        let trace_matcher = mantra_lang_tracing::extract::req_trace_matcher();
        let comment_content = node.utf8_text(src).ok()?;

        let captures: Vec<_> = trace_matcher.captures_iter(comment_content).collect();

        if !captures.is_empty() {
            let span = associated_item_span(*node);

            let mut traces = Vec::new();
            for capture in captures {
                traces.push(
                    TraceEntry::try_from(RawTraceEntry::new(
                        capture.name("ids")?.as_str(),
                        node.start_position().row + 1,
                        span,
                        get_ident(filepath, span, lsif_graphs.as_deref()),
                    ))
                    .ok()?,
                )
            }

            return Some(traces);
        }
    }

    None
}

fn get_ident(
    filepath: &str,
    span: Option<LineSpan>,
    lsif_graphs: Option<&[LsifGraph]>,
) -> Option<String> {
    let start_line = span?.start;
    let graphs = lsif_graphs?;

    for graph in graphs {
        if graph.contains_doc(filepath) {
            // "-1" because lsif starts with line nr 0
            if let Some(ident) = graph.get_identifier(filepath, start_line - 1) {
                return Some(ident);
            }
        }
    }

    None
}

fn associated_item_span(mut node: AstNode) -> Option<LineSpan> {
    while let Some(sibling) = node.next_named_sibling() {
        let sibling_kind = sibling.kind();

        if sibling_kind.ends_with("_item") {
            let start = Line::try_from(sibling.start_position().row + 1).ok()?;
            let end = Line::try_from(sibling.end_position().row + 1).ok()?;

            return Some(LineSpan { start, end });
        } else if sibling_kind.ends_with("comment") && !is_doc_comment(&sibling) {
            return None;
        }

        node = sibling;
    }

    None
}

fn is_doc_comment(node: &AstNode) -> bool {
    if let Some(doc_node) = node.named_child(1) {
        doc_node.kind() == "doc_comment"
    } else {
        false
    }
}

fn is_req_macro(content: &str) -> bool {
    matches!(content, "req" | "requirements")
        || content
            .rsplit_once("::")
            .map(|(_, name)| matches!(name, "req" | "requirements"))
            .unwrap_or(false)
}
