use std::str::FromStr;

use anyhow::{anyhow, bail};
use mantra_schema::{
    Line, LineSpan,
    annotations::{
        Annotations, CodeBlock, Element, ElementKind, Trace, TraceKind, TraceRelatedCodeVariant,
    },
    requirements::ReqId,
};
use tree_sitter::{Node, Parser, TreeCursor};

use crate::{
    collect::collector::AnnotationCollector,
    traces::variants::{AttributeTraceVariant, FnLikeTraceVariant},
};

pub struct RustCodeCollector;

impl AnnotationCollector for RustCodeCollector {
    fn collect_relative(content: &str, start_line: Line) -> Result<Annotations, anyhow::Error> {
        let mut traces = Vec::new();
        let mut elements = Vec::new();

        let content_bytes = content.as_bytes();
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
        let tree = parser
            .parse(content, None)
            .ok_or(anyhow::anyhow!("Failed to parse Rust code"))?;
        let mut cursor = tree.walk();

        let mut reached_innermost_item = false;

        // top down traversal
        loop {
            let go_next_sibling_or_parent = reached_innermost_item || !cursor.goto_first_child();
            if go_next_sibling_or_parent {
                reached_innermost_item = false;

                if goto_next_sibling_or_parent(&mut cursor).is_none() {
                    break;
                }
            }

            let node = cursor.node();
            let node_kind = node.kind();

            if node_kind == "attribute_item"
                && let Some(attribute_node) = node.named_child(0)
            {
                if let Some(name_node) = attribute_node.named_child(0)
                    && let Some(trace_kind) = get_attrb_macro_trace_kind(&name_node, content_bytes)
                    && let Some(args_node) = attribute_node.named_child(1)
                    && args_node.kind() == "token_tree"
                {
                    let ids = get_req_ids(&args_node, content_bytes, start_line)?;
                    let traced_line: Line =
                        attribute_node.start_position().row.try_into().unwrap_or(-1) + start_line; // TODO: handle bad line

                    let element_def_line =
                        get_related_element_def_line(&mut cursor.clone(), start_line)?;

                    traces.push(Trace {
                        ids,
                        line: traced_line,
                        related_code: Some(TraceRelatedCodeVariant::ElementAtLine(
                            element_def_line,
                        )),
                        kind: trace_kind,
                        properties: None,
                    });
                }

                reached_innermost_item = true;
            } else if node_kind == "macro_invocation"
                && let Some(name_node) = node.named_child(0)
                && let Some(trace_kind) = get_fn_macro_trace_kind(&name_node, content_bytes)
                && let Some(args_node) = node.named_child(1)
                && args_node.kind() == "token_tree"
            {
                let ids = get_req_ids(&args_node, content_bytes, start_line)?;
                let traced_line: Line =
                    node.start_position().row.try_into().unwrap_or(-1) + start_line;
                let end = node.end_position().row.try_into().unwrap_or(-1) + start_line;

                traces.push(Trace {
                    ids,
                    line: traced_line,
                    related_code: Some(TraceRelatedCodeVariant::CodeBlock(CodeBlock {
                        kind: mantra_schema::annotations::CodeBlockKind::Other,
                        content: None,
                        span: LineSpan {
                            start: traced_line,
                            end,
                        },
                    })),
                    kind: trace_kind,
                    properties: None,
                });
            } else if node_kind.ends_with("_item")
                || node_kind == "extern_crate_declaration"
                || node_kind == "use_declaration"
            {
                elements.push(get_element(&mut cursor.clone(), content_bytes, start_line)?);
            }
        }

        Ok(Annotations {
            traces,
            elements,
            coverage_excludes: vec![],
        })
    }
}

fn get_element(
    cursor: &mut TreeCursor<'_>,
    content: &[u8],
    start_line: Line,
) -> Result<Element, anyhow::Error> {
    let item = cursor.node();
    let element_start_node = get_element_start_node(cursor);
    let element_content = Some(String::from_utf8(
        content[element_start_node.start_byte()..=item.end_byte()]
            .iter()
            .copied()
            .collect(),
    )?);

    let name = if item.kind() == "function_item"
        || item.kind() == "function_signature_item"
        || item.kind() == "mod_item"
        || item.kind() == "const_item"
        || item.kind() == "static_item"
        || item.kind() == "extern_crate_declaration"
        || item.kind() == "struct_item"
        || item.kind() == "enum_item"
        || item.kind() == "union_item"
        || item.kind() == "type_item"
        || item.kind() == "trait_item"
    {
        item.child_by_field_name("name")
            .map(|n| n.utf8_text(content).ok())
            .flatten()
            .unwrap_or("<unknown>")
            .to_string()
    } else if item.kind() == "use_declaration" {
        item.utf8_text(content)?.to_string()
    } else if item.kind() == "impl_item" || item.kind() == "foreign_mod_item" {
        if let Some(body) = item.child_by_field_name("body") {
            String::from_utf8(
                content[item.start_byte()..body.start_byte()]
                    .iter()
                    .copied()
                    .collect(),
            )?
        } else {
            item.utf8_text(content)?.to_string()
        }
    } else {
        bail!("Unknown item kind '{}'", item.kind());
    };

    let kind = get_element_kind(item.kind());

    Ok(Element {
        ident: None,
        name,
        definition_line: item.start_position().row.try_into().unwrap_or(-1) + start_line,
        span: LineSpan {
            start: element_start_node
                .start_position()
                .row
                .try_into()
                .unwrap_or(-1)
                + start_line,
            end: item.end_position().row.try_into().unwrap_or(-1) + start_line,
        },
        kind,
        content: element_content,
    })
}

fn get_element_kind(kind: &str) -> ElementKind {
    match kind {
        "function_item" => ElementKind::Function,
        "function_signature_item" => ElementKind::Function,
        "mod_item" => ElementKind::Module,
        "const_item" => ElementKind::Const,
        "static_item" => ElementKind::Variable,
        "extern_crate_declaration" => ElementKind::Other,
        "struct_item" => ElementKind::Type,
        "enum_item" => ElementKind::Type,
        "union_item" => ElementKind::Type,
        "type_item" => ElementKind::Type,
        "trait_item" => ElementKind::Trait,
        "use_declaration" => ElementKind::Other,
        "foreign_mod_item" => ElementKind::Other,
        "impl_item" => ElementKind::Other,
        _ => unreachable!(),
    }
}

fn get_element_start_node<'a>(cursor: &mut TreeCursor<'a>) -> Node<'a> {
    let mut curr_node = cursor.node();

    while cursor.goto_previous_sibling()
        && (cursor.node().kind() == "attribute_item"
            || (cursor.node().kind() == "line_comment"
                && cursor
                    .node()
                    .named_child(1)
                    .map(|n| n.kind() == "doc_comment")
                    .unwrap_or(false)))
    {
        curr_node = cursor.node();
    }

    curr_node
}

fn get_related_element_def_line(
    cursor: &mut TreeCursor<'_>,
    start_line: Line,
) -> Result<Line, anyhow::Error> {
    let trace_node = cursor.node();

    if !cursor.goto_next_sibling() {
        bail!(
            "Missing related element for trace at line {}",
            trace_node.start_position().row + start_line.try_into().unwrap_or(0)
        );
    }

    let mut next_node = cursor.node();
    println!("kind='{}' node='{:?}'", next_node.kind(), next_node);

    while next_node.kind() == "attribute_item"
        || (next_node.kind() == "line_comment"
            && next_node
                .named_child(1)
                .map(|n| n.kind() == "doc_comment")
                .unwrap_or(false))
    {
        if !cursor.goto_next_sibling() {
            bail!(
                "Missing related element for trace at line {}",
                trace_node.start_position().row + start_line.try_into().unwrap_or(0)
            );
        }

        next_node = cursor.node();
    }

    if next_node.kind().ends_with("_item") {
        Ok(next_node.start_position().row.try_into().unwrap_or(-1) + start_line)
    } else {
        Err(anyhow!("No Rust item found after attribute trace"))
    }
}

fn get_req_ids(
    args_node: &Node<'_>,
    content: &[u8],
    start_line: Line,
) -> Result<Vec<ReqId>, anyhow::Error> {
    let line = args_node.start_position().row + start_line.try_into().unwrap_or(0);

    if args_node.has_error() {
        bail!("Mantra trace at line '{line}' has syntax error");
    }

    let mut ids = Vec::new();

    let mut cursor = args_node.walk();
    let mut expected_kind = "(";
    let mut reached_end = false;

    for child in args_node.children(&mut cursor) {
        if expected_kind == child.kind() {
            if expected_kind == "(" {
                expected_kind = "string_literal";
            } else if expected_kind == "string_literal"
                && let Some(content_node) = child.named_child(0)
                && let Ok(id) = content_node.utf8_text(content)
            {
                ids.push(id.to_string());
                expected_kind = ",";
            } else {
                expected_kind = "string_literal";
            }
        } else if child.kind() == ")" {
            reached_end = true;
        } else {
            bail!(
                "Mantra trace at line '{line}' must only consist of comma separated string literals!"
            );
        }
    }

    if !reached_end {
        bail!("Mantra trace at line '{line}' is incomplete!");
    }

    Ok(ids)
}

fn get_attrb_macro_trace_kind(node: &Node<'_>, content: &[u8]) -> Option<TraceKind> {
    let ident_node = get_ident_node(node)?;
    let macro_name = ident_node.utf8_text(content).ok()?;

    Some(AttributeTraceVariant::from_str(macro_name).ok()?.into())
}

fn get_fn_macro_trace_kind(node: &Node<'_>, content: &[u8]) -> Option<TraceKind> {
    let ident_node = get_ident_node(node)?;
    let macro_name = ident_node.utf8_text(content).ok()?;

    Some(FnLikeTraceVariant::from_str(macro_name).ok()?.into())
}

fn get_ident_node<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "identifier" {
        Some(*node)
    } else if node.kind() == "scoped_identifier" {
        node.named_child(1)
    } else {
        return None;
    }
}

fn goto_next_sibling_or_parent(cursor: &mut TreeCursor<'_>) -> Option<()> {
    let has_sibling = cursor.goto_next_sibling();
    if !has_sibling {
        let mut has_next_upper = false;

        while !has_next_upper {
            if !cursor.goto_parent() {
                return None;
            }

            has_next_upper = cursor.goto_next_sibling();
        }
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use crate::collect::{collector::AnnotationCollector, rust::RustCodeCollector};

    #[test]
    fn simple_attrb() {
        let content = r#"
            #[req("ID"1)]
            fn foo() {}
            "#;
        let annotations = RustCodeCollector::collect(content).unwrap();

        println!("{annotations:?}");
    }
}
