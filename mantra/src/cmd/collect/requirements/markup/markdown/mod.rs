use std::str::FromStr;

use anyhow::anyhow;
use comrak::{
    Node, Options,
    nodes::{ListType, NodeHeading, NodeValue},
};
use mantra_schema::{
    Origin, SCHEMA_VERSION,
    product::ProductId,
    requirements::{ReqId, Requirement, RequirementSchema},
};

use crate::cmd::collect::{
    collector::CollectableFile,
    markup::{Frontmatter, MD_FRONTMATTER_DELIMITER},
};

#[cfg(test)]
mod tests;

pub fn collect_requirements(
    product_id: &ProductId,
    file: &CollectableFile,
) -> Result<Option<RequirementSchema>, anyhow::Error> {
    let arena = comrak::Arena::default();
    let mut comrak_options = comrak::Options::default();
    comrak_options.extension.front_matter_delimiter = Some(MD_FRONTMATTER_DELIMITER.to_owned());

    let root_node = comrak::parse_document(&arena, file.content, &comrak_options);

    if root_node.data().value != NodeValue::Document {
        return Err(anyhow::anyhow!("Expected Markdown root to be a document"));
    }

    let mut top_nodes = root_node.children().peekable();

    let peeked_child = top_nodes.peek();

    let (properties, origin) = if let Some(peeked_node) = peeked_child
        && let NodeValue::FrontMatter(frontmatter_content) = &peeked_node.data().value
    {
        let _ = top_nodes.next();
        let frontmatter = resolve_frontmatter(frontmatter_content)?;

        if let Some(pid) = frontmatter.product_id
            && &pid != product_id
        {
            log::info!(
                "Found explicit product ID that differs to current. Skipping requirement collection in file '{}'.",
                file.filepath
            );

            return Ok(None);
        }

        let origin = if frontmatter.origin.is_some() {
            frontmatter.origin
        } else {
            Some(serde_json::json!({
                "file": &file.filepath
            }))
        };

        (frontmatter.properties, origin)
    } else {
        (
            None,
            Some(serde_json::json!({
                "file": &file.filepath
            })),
        )
    };

    let requirements = extract_requirements(top_nodes)?;

    if requirements.is_empty() {
        Ok(None)
    } else {
        Ok(Some(RequirementSchema {
            schema_version: Some(SCHEMA_VERSION.to_string()),
            product_id: Some(product_id.clone()),
            requirements,
            properties,
            origin,
        }))
    }
}

fn resolve_frontmatter(content: &str) -> Result<Frontmatter, anyhow::Error> {
    let content = content.replacen(MD_FRONTMATTER_DELIMITER, "{", 1);
    let (content, _) = content
        .rsplit_once(MD_FRONTMATTER_DELIMITER)
        .ok_or(anyhow!("Expected closing frontmatter delimiter"))?;
    let mut content = content.to_string();
    content.push('}');

    Ok(json5::from_str(&content)?)
}

fn extract_requirements(
    mut top_nodes: std::iter::Peekable<
        comrak::arena_tree::Children<'_, std::cell::RefCell<comrak::nodes::Ast>>,
    >,
) -> Result<Vec<Requirement>, anyhow::Error> {
    while top_nodes
        .peek()
        .map(|n| !matches!(n.data().value, NodeValue::Heading(_)))
        .unwrap_or(false)
    {
        let _ = top_nodes.next();
    }

    let mut requirements = Vec::new();

    while let Some(node) = top_nodes.next() {
        let line = node.data().sourcepos.start.line;

        if matches!(node.data().value, NodeValue::Heading(_))
            && let Some(req_heading) = extract_requirement_heading(node)?
        {
            let mut requirement = Requirement::new_minimal(
                req_heading.id,
                req_heading.title,
                serde_json::json!({
                    "line": line
                }),
            );

            fill_req_body(&mut requirement, &mut top_nodes)?;

            requirements.push(requirement);
        } else {
            log::debug!(
                "Ignoring section outside a requirement definition starting at line '{}'",
                line
            );

            while top_nodes
                .peek()
                .map(|n| !matches!(n.data().value, NodeValue::Heading(_)))
                .unwrap_or(false)
            {
                let _ = top_nodes.next();
            }
        }
    }

    Ok(requirements)
}

struct RequirementHeading {
    id: ReqId,
    title: String,
}

fn extract_requirement_heading(
    heading_node: Node,
) -> Result<Option<RequirementHeading>, anyhow::Error> {
    let mut children = heading_node.children();

    if let Some(req_id_part) = children.next()
        && let NodeValue::Code(enclosed_id) = &req_id_part.data().value
    {
        let req_id = ReqId::from_str(&enclosed_id.literal)?;

        let title_part = children.fold(String::new(), |mut base, n| {
            let mut s = String::new();
            let _ = comrak::format_commonmark(n, &Options::default(), &mut s);
            base.push_str(&s);
            base
        });

        let trimmed = title_part.trim();

        if let Some(title) = trimmed.strip_prefix(": ") {
            if trimmed != title_part.trim_end() {
                log::warn!(
                    "Remove space before ':' in requirement heading at line '{}'",
                    heading_node.data().sourcepos.start.line
                );
            }

            return Ok(Some(RequirementHeading {
                id: req_id,
                title: title.to_string(),
            }));
        }
    }

    Ok(None)
}

fn fill_req_body(
    requirement: &mut Requirement,
    top_nodes: &mut std::iter::Peekable<
        comrak::arena_tree::Children<'_, std::cell::RefCell<comrak::nodes::Ast>>,
    >,
) -> Result<(), anyhow::Error> {
    let peeked_node = top_nodes.peek();

    if peeked_node
        .map(|n| {
            if let NodeValue::List(list) = &n.data().value
                && list.list_type == ListType::Bullet
            {
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
    {
        let node = top_nodes
            .next()
            .expect("Peek above ensures that there is a node");

        // TODO: extract requirement properties
        log::info!("List entry: '{:#?}'", node);
    }

    let mut description: Option<String> = None;

    while let Some(peeked) = top_nodes.peek()
        && (!matches!(peeked.data().value, NodeValue::Heading(_))
            || extract_requirement_heading(peeked).ok().flatten().is_none())
    {
        let next = top_nodes
            .next()
            .expect("Peek above ensures that there is a node");

        let mut s = String::new();
        comrak::format_commonmark(next, &Options::default(), &mut s)?;

        if let Some(desc) = description.as_mut() {
            desc.push_str(&s);
        } else {
            description = Some(s)
        }
    }

    requirement.description = description;

    Ok(())
}
