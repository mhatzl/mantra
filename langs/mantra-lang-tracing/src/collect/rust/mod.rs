use std::str::FromStr;

use anyhow::{anyhow, bail};
use mantra_schema::{
    FmtHash, Line, LineSpan,
    annotations::{
        Annotations, CodeBlock, Element, ElementKind, Trace, TraceKind, TraceRelatedCodeVariant,
    },
    product::ProductId,
    requirements::{ReqId, RequirementPk},
};
use serde_json::Map;
use tree_sitter::{Node, Parser, TreeCursor};

use crate::{
    collect::collector::AnnotationCollector,
    traces::variants::{AttributeTraceVariant, FnLikeTraceVariant},
};

#[cfg(test)]
mod tests;

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

            if (node_kind == "attribute_item" || node_kind == "inner_attribute_item")
                && let Some(attribute_node) = node.named_child(0)
            {
                if let Some(name_node) = attribute_node.named_child(0) {
                    if let Some(trace_kind) = get_attrb_macro_trace_kind(&name_node, content_bytes)
                        && let Some(args_node) = attribute_node.named_child(1)
                        && args_node.kind() == "token_tree"
                    {
                        let ids = get_req_ids(&args_node, content_bytes, start_line, false)?;
                        let traced_line: Line =
                            attribute_node.start_position().row.try_into().unwrap_or(-1)
                                + start_line; // TODO: handle bad line

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
                    } else if name_node.utf8_text(content_bytes) == Ok("cfg_attr")
                        && let Some(mut attr_traces) =
                            parse_cfg_attr_for_traces(&attribute_node, content_bytes, start_line)
                    {
                        // patch in related element
                        let element_def_line =
                            get_related_element_def_line(&mut cursor.clone(), start_line)?;
                        for trace in &mut attr_traces {
                            trace.related_code =
                                Some(TraceRelatedCodeVariant::ElementAtLine(element_def_line));
                        }

                        traces.extend(attr_traces);
                    }
                }

                reached_innermost_item = true;
            } else if node_kind == "macro_invocation"
                && let Some(name_node) = node.named_child(0)
                && let Some(trace_kind) = get_fn_macro_trace_kind(&name_node, content_bytes)
                && let Some(args_node) = node.named_child(1)
                && args_node.kind() == "token_tree"
            {
                let ids = get_req_ids(&args_node, content_bytes, start_line, true)?;
                let traced_line: Line =
                    node.start_position().row.try_into().unwrap_or(-1) + start_line;
                let end = node.end_position().row.try_into().unwrap_or(-1) + start_line;
                let node_hash = Some(FmtHash::new(node.utf8_text(content_bytes)?));

                traces.push(Trace {
                    ids,
                    line: traced_line,
                    related_code: Some(TraceRelatedCodeVariant::CodeBlock(CodeBlock {
                        kind: mantra_schema::annotations::CodeBlockKind::Other,
                        content_hash: node_hash,
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

fn parse_cfg_attr_for_traces(
    attrb_node: &Node<'_>,
    content_bytes: &[u8],
    start_line: Line,
) -> Option<Vec<Trace>> {
    let token_tree = attrb_node.named_child(1)?;

    traces_in_cfg_attr_token_tree(token_tree, content_bytes, start_line, &[])
}

fn traces_in_cfg_attr(
    identifier_node: Node<'_>,
    token_tree: Node<'_>,
    content_bytes: &[u8],
    start_line: Line,
    cfg_conditions: &[serde_json::Value],
) -> Option<Vec<Trace>> {
    if identifier_node.kind() != "identifier" || token_tree.kind() != "token_tree" {
        return None;
    }

    if let Some(trace_kind) = get_attrb_macro_trace_kind(&identifier_node, content_bytes) {
        if let Ok(ids) = get_req_ids(&token_tree, content_bytes, start_line, false) {
            let mut trace_props = Map::new();
            trace_props.insert(
                "cfg_attr".to_string(),
                serde_json::Value::Array(cfg_conditions.to_vec()),
            );

            Some(vec![Trace {
                ids,
                line: identifier_node
                    .start_position()
                    .row
                    .try_into()
                    .unwrap_or(-1)
                    + start_line,
                related_code: None,
                kind: trace_kind,
                properties: Some(trace_props),
            }])
        } else {
            log::warn!(
                "Could not extract requirements from mantra-compatible macro at line {}",
                identifier_node.start_position().row
                    + usize::try_from(start_line).unwrap_or_default()
            );

            None
        }
    } else if identifier_node.utf8_text(content_bytes) == Ok("cfg_attr") {
        traces_in_cfg_attr_token_tree(token_tree, content_bytes, start_line, cfg_conditions)
    } else {
        None
    }
}

fn traces_in_cfg_attr_token_tree(
    token_tree: Node<'_>,
    content_bytes: &[u8],
    start_line: Line,
    cfg_conditions: &[serde_json::Value],
) -> Option<Vec<Trace>> {
    let mut cursor = token_tree.walk();
    let mut children = token_tree.children(&mut cursor).skip(1).peekable(); // skip initial `(`

    let mut condition = String::new();
    while let Some(node) = children.next()
        && node.kind() != ","
        && let Ok(content) = node.utf8_text(content_bytes)
    {
        condition.push_str(content);
    }

    let mut conditions = cfg_conditions.to_vec();
    conditions.push(serde_json::Value::String(condition.clone()));
    let mut traces = Vec::new();

    loop {
        let next_node = children.next();

        if let Some(ident_node) = next_node
            && ident_node.kind() == "identifier"
            && let Some(inner_token_tree) = children.peek()
            && inner_token_tree.kind() == "token_tree"
        {
            let inner_tree = children.next().expect("Peek succeeded for next node");

            if let Some(inner_traces) = traces_in_cfg_attr(
                ident_node,
                inner_tree,
                content_bytes,
                start_line,
                &conditions,
            ) {
                traces.extend(inner_traces);
            }
        } else if let Some(node) = next_node
            && node.kind() == ")"
        {
            break;
        } else if next_node.is_none() {
            break;
        }
    }

    if traces.is_empty() {
        None
    } else {
        Some(traces)
    }
}

fn get_element(
    cursor: &mut TreeCursor<'_>,
    content: &[u8],
    start_line: Line,
) -> Result<Element, anyhow::Error> {
    let item = cursor.node();
    let element_start_node = get_element_start_node(cursor);
    let element_content_hash = Some(FmtHash::new(&String::from_utf8(
        content[element_start_node.start_byte()..=item.end_byte()].to_vec(),
    )?));

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
            .and_then(|n| n.utf8_text(content).ok())
            .unwrap_or("<unknown>")
            .to_string()
    } else if item.kind() == "use_declaration" {
        item.utf8_text(content)?.to_string()
    } else if item.kind() == "impl_item" || item.kind() == "foreign_mod_item" {
        if let Some(body) = item.child_by_field_name("body") {
            String::from_utf8(content[item.start_byte()..body.start_byte()].to_vec())?
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
        content_hash: element_content_hash,
    })
}

fn get_element_kind(kind: &str) -> ElementKind {
    match kind {
        "function_item" => ElementKind::Function,
        "function_signature_item" => ElementKind::FunctionSignature,
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

// Note: temporary fn before moving to custom parser
fn get_req_ids(
    args_node: &Node<'_>,
    content: &[u8],
    start_line: Line,
    fn_like: bool,
) -> Result<Vec<RequirementPk>, anyhow::Error> {
    let line = args_node.start_position().row + start_line.try_into().unwrap_or(0);

    if args_node.has_error() {
        bail!("Mantra trace at line '{line}' has syntax error");
    }

    let mut ids = Vec::new();

    let mut cursor = args_node.walk();
    let mut expected_kinds: &[&str] = &["("];
    let mut reached_end = false;

    for child in args_node.children(&mut cursor) {
        if expected_kinds.contains(&child.kind()) {
            if child.kind() == "token_tree" {
                let req_pk = get_req_pk(&child, content, start_line)?;
                ids.push(req_pk);
                expected_kinds = &[","];
            } else if child.kind() == "string_literal"
                && let Some(content_node) = child.named_child(0)
                && let Ok(id) = content_node.utf8_text(content)
            {
                ids.push(RequirementPk {
                    id: ReqId::from_str(id)?,
                    product_id: None,
                });
                expected_kinds = &[","];
            } else {
                expected_kinds = &["string_literal", "token_tree"];
            }
        } else if child.kind() == ")" || (child.kind() == "=>" && fn_like) {
            // Note: ")" is end of a simple macro and "=>" marks start of code block in fn-like macros
            reached_end = true;
            break;
        } else {
            bail!(
                "Mantra trace at line '{line}' must only consist of comma separated string literals or objects with fields 'id' and 'product_id'!"
            );
        }
    }

    if !reached_end {
        bail!("Mantra trace at line '{line}' is incomplete!");
    }

    Ok(ids)
}

fn get_req_pk(
    tree_node: &Node<'_>,
    content: &[u8],
    _start_line: Line,
) -> Result<RequirementPk, anyhow::Error> {
    const ID_IDENT: &str = "id";
    const PRODUCT_ID_IDENT: &str = "product_id";

    let mut id = None;
    let mut product_id = None;

    let mut cursor = tree_node.walk();
    let mut children = tree_node.children(&mut cursor);
    let curly_open = children.next().ok_or(anyhow!(
        "Expected '{{' as start of a requirement trace using the explicit object notation."
    ))?;

    if curly_open.kind() != "{" {
        bail!("Expected '{{' as start of a requirement trace using the explicit object notation.")
    }

    let first_field = children.next().ok_or(anyhow!(
        "Expected first field of a requirement trace using the explicit object notation."
    ))?;
    let first_field_separator = children.next().ok_or(anyhow!(
        "Expected field separator ':' of a requirement trace using the explicit object notation."
    ))?;

    if first_field_separator.kind() != ":" {
        bail!(
            "Expected field separator ':' of a requirement trace using the explicit object notation."
        );
    }

    let first_field_value = children.next().ok_or(anyhow!(
        "Expected field value of a requirement trace using the explicit object notation."
    ))?;

    let resolved_first_ident = if first_field.kind() == "identifier"
        && let Ok(ident) = first_field.utf8_text(content)
    {
        ident
    } else if first_field.kind() == "string_literal"
        && let Some(content_node) = first_field.named_child(0)
        && let Ok(key) = content_node.utf8_text(content)
    {
        key
    } else {
        bail!(
            "Expected either '{}' or '{}' as keys",
            ID_IDENT,
            PRODUCT_ID_IDENT
        )
    };

    if resolved_first_ident != ID_IDENT && resolved_first_ident != PRODUCT_ID_IDENT {
        bail!(
            "Found field '{}'. Allowed fields are '{}' and '{}'",
            resolved_first_ident,
            ID_IDENT,
            PRODUCT_ID_IDENT,
        );
    }

    if first_field_value.kind() != "string_literal" {
        bail!(
            "Expected string literal as value for field '{}' of a requirement trace using the explicit object notation.",
            resolved_first_ident
        );
    }

    let first_field_value_content = first_field_value
        .named_child(0)
        .ok_or(anyhow!("Expected string value"))?;
    let first_value = first_field_value_content.utf8_text(content)?;

    if resolved_first_ident == ID_IDENT {
        id = Some(ReqId::from_str(first_value)?);
    } else if resolved_first_ident == PRODUCT_ID_IDENT {
        product_id = Some(ProductId::from_str(first_value)?);
    }

    let mut next_child = children
        .next()
        .ok_or(anyhow!("Expected ',' or closing '}}'"))?;

    if next_child.kind() == "," {
        next_child = children
            .next()
            .ok_or(anyhow!("Expected identifier or closing '}}'"))?;

        if next_child.kind() == "identifier" || next_child.kind() == "string_literal" {
            let resolved_second_ident = if first_field.kind() == "identifier"
                && let Ok(ident) = next_child.utf8_text(content)
            {
                ident
            } else if next_child.kind() == "string_literal"
                && let Some(content_node) = next_child.named_child(0)
                && let Ok(key) = content_node.utf8_text(content)
            {
                key
            } else {
                bail!("Failed to extract the second key");
            };

            let second_field_separator = children.next().ok_or(anyhow!(
                "Expected field separator ':' of a requirement trace using the explicit object notation."
            ))?;

            if second_field_separator.kind() != ":" {
                bail!(
                    "Expected field separator ':' of a requirement trace using the explicit object notation."
                );
            }

            let second_field_value = children.next().ok_or(anyhow!(
                "Expected field value of a requirement trace using the explicit object notation."
            ))?;

            if second_field_value.kind() != "string_literal" {
                bail!(
                    "Expected string literal as value for field '{}' of a requirement trace using the explicit object notation.",
                    resolved_second_ident
                );
            }

            let second_field_value_content = second_field_value
                .named_child(0)
                .ok_or(anyhow!("Expected string value"))?;
            let second_value = second_field_value_content.utf8_text(content)?;

            if resolved_second_ident == ID_IDENT {
                if id.is_some() {
                    bail!("Duplicate field '{}'", ID_IDENT);
                } else {
                    id = Some(ReqId::from_str(second_value)?);
                }
            } else if resolved_second_ident == PRODUCT_ID_IDENT {
                if product_id.is_some() {
                    bail!("Duplicate field '{}'", PRODUCT_ID_IDENT);
                } else {
                    product_id = Some(ProductId::from_str(second_value)?);
                }
            }

            next_child = children
                .next()
                .ok_or(anyhow!("Expected either ',' or '}}'"))?;

            if next_child.kind() == "," {
                next_child = children.next().ok_or(anyhow!("Expected '}}'"))?;
            }
        }
    }

    if next_child.kind() != "}" {
        bail!("Expected closing '}}'");
    }

    if children.next().is_some() {
        bail!("Expected no more tokens after closing '}}'");
    }

    Ok(RequirementPk {
        id: id.ok_or_else(|| {
            anyhow!(
                "Missing field '{}' for the explicit requirement trace object notation",
                ID_IDENT
            )
        })?,
        product_id,
    })
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
        None
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
