use mantra_lang_tracing::{
    collect::{AstNode, ItemEntry, Line, LineSpan, TraceEntry, TraceInfo},
    lsif_graph::LsifGraph,
    RawTraceEntry,
};

pub fn collect_trace_info_in_rust(
    node: &AstNode,
    src: &[u8],
    filepath: &str,
    lsif_graphs: &Option<Vec<LsifGraph>>,
) -> Option<TraceInfo> {
    let node_kind = node.kind();

    if node_kind == "attribute_item" || node_kind == "macro_invocation" {
        let (macro_node, has_associated_item) = if node_kind == "macro_invocation" {
            (*node, false)
        } else {
            let attribute_node = node.named_child(0)?;
            (attribute_node, true)
        };

        let ident = macro_node.named_child(0)?;
        let macro_content = macro_node.named_child(1)?;

        if is_req_macro(ident, src) {
            let item_start = if has_associated_item {
                get_associated_item_start(*node)
            } else {
                None
            };
            let Ok(line) = (ident.start_position().row + 1).try_into() else {
                // TODO: log in case line nr exceeds u32
                return None;
            };

            return Some(TraceInfo {
                traces: vec![TraceEntry::try_from(RawTraceEntry::new(
                    macro_content
                        .utf8_text(src)
                        .ok()?
                        .strip_prefix('(')
                        .and_then(|s| s.strip_suffix(')'))?,
                    line,
                    item_start,
                    // get_ident(filepath, span, lsif_graphs.as_deref()),
                ))
                .ok()?],
                items: vec![],
            });
        } else if ident.kind() == "identifier" && ident.utf8_text(src) == Ok("cfg_attrb") {
            let mut traces = Vec::new();

            let item_start = if has_associated_item {
                get_associated_item_start(*node)
            } else {
                None
            };
            let Ok(line) = (ident.start_position().row + 1).try_into() else {
                // TODO: log in case line nr exceeds u32
                return None;
            };

            for child in macro_content.named_children(&mut macro_content.walk()) {
                if is_req_macro(child, src) {
                    let ids = child
                        .next_named_sibling()
                        .expect("Sibling checked in condition")
                        .utf8_text(src)
                        .ok()?
                        .strip_prefix('(')
                        .and_then(|s| s.strip_suffix(')'))?;
                    if let Ok(entry) =
                        TraceEntry::try_from(RawTraceEntry::new(ids, line, item_start))
                    {
                        traces.push(entry);
                    }
                }
            }

            return if traces.is_empty() {
                None
            } else {
                Some(TraceInfo {
                    traces,
                    items: vec![],
                })
            };
        }
    } else if node_kind == "line_comment" && is_doc_comment(node) {
        let trace_matcher = mantra_lang_tracing::extract::req_trace_matcher();
        let comment_content = node.utf8_text(src).ok()?;

        let captures: Vec<_> = trace_matcher.captures_iter(comment_content).collect();

        if !captures.is_empty() {
            let item_start = get_associated_item_start(*node);

            let mut traces = Vec::new();
            for capture in captures {
                if let Ok(line) = (node.start_position().row + 1).try_into() {
                    // TODO: log in case line nr exceeds u32
                    traces.push(
                        TraceEntry::try_from(RawTraceEntry::new(
                            capture.name("ids")?.as_str(),
                            line,
                            item_start,
                        ))
                        .ok()?,
                    )
                };
            }

            return Some(TraceInfo {
                traces,
                items: vec![],
            });
        }
    } else if node_kind.ends_with("_item") {
        let span = LineSpan {
            start: (node.start_position().row + 1).try_into().ok()?,
            end: (node.end_position().row + 1).try_into().ok()?,
        };
        let ident = if node_kind == "impl_item" {
            let mut impl_ident = String::from("impl");

            for named_child in node.named_children(&mut node.walk()) {
                if named_child.grammar_name() == "body" {
                    break;
                }

                if named_child.kind() == "type_identifier" {
                    impl_ident.push(' ');
                }

                if named_child.grammar_name() == "trait" {
                    impl_ident.push_str("for ");
                }

                let content = named_child.utf8_text(src).unwrap_or("-");
                impl_ident.push_str(content);
            }

            impl_ident
        } else {
            get_ident(filepath, span.start - 1, lsif_graphs.as_deref()).unwrap_or_else(|| {
                let Some(ident) = node.named_child(0) else {
                    return "-".to_string();
                };
                ident.utf8_text(src).unwrap_or("-").to_string()
            })
        };

        return Some(TraceInfo {
            traces: vec![],
            items: vec![ItemEntry {
                ident,
                span,
                is_test: false, // TODO: detect if its a test fn
            }],
        });
    }

    None
}

fn get_ident(
    filepath: &str,
    item_start_zero_based: Line,
    lsif_graphs: Option<&[LsifGraph]>,
) -> Option<String> {
    let graphs = lsif_graphs?;

    for graph in graphs {
        if graph.contains_doc(filepath) {
            // "-1" because lsif starts with line nr 0
            if let Some(ident) = graph.get_identifier(filepath, item_start_zero_based) {
                return Some(ident);
            }
        }
    }

    None
}

fn get_associated_item_start(mut node: AstNode) -> Option<Line> {
    while let Some(sibling) = node.next_named_sibling() {
        let sibling_kind = sibling.kind();

        if (sibling_kind.ends_with("_item") && sibling_kind != "attribute_item")
            || sibling_kind == "field_declaration"
            || sibling_kind == "enum_variant"
        {
            let start = Line::try_from(sibling.start_position().row + 1).ok()?;
            return Some(start);
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

fn is_req_macro(node: AstNode, src: &[u8]) -> bool {
    ((node.kind() == "identifier" && node.utf8_text(src).map_or(false, is_req_ident))
        || (node.kind() == "scoped_identifier"
            && node
                .named_child(1)
                .map_or(false, |n| n.utf8_text(src).map_or(false, is_req_ident))))
        && node
            .next_named_sibling()
            .map_or(false, |n| n.kind() == "token_tree")
}

fn is_req_ident(ident: &str) -> bool {
    matches!(ident, "req" | "reqcov" | "requirements")
}
