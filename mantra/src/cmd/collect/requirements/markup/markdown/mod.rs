use std::str::FromStr;

use anyhow::{Context, anyhow, bail};
use comrak::{
    Node, Options,
    nodes::{ListType, NodeValue},
};
use mantra_lang_tracing::collect::collector::AnnotationCollector;
use mantra_schema::{
    Properties, SCHEMA_VERSION,
    product::ProductId,
    requirements::{ReqId, Requirement, RequirementPk, RequirementSchema},
};

use crate::cmd::collect::{
    collector::CollectableFile,
    markup::{Frontmatter, MD_FRONTMATTER_DELIMITER},
    requirements::markup::markdown::origin::{BaseOrigin, RequirementOrigin},
};

#[cfg(test)]
mod tests;

pub mod origin {
    use mantra_schema::path::RelativePathBuf;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct BaseOrigin {
        pub file: RelativePathBuf,
    }

    impl BaseOrigin {
        pub fn to_value(self) -> Result<serde_json::Value, serde_json::Error> {
            serde_json::to_value(self)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct RequirementOrigin {
        pub line: usize,
    }

    impl RequirementOrigin {
        pub fn to_value(self) -> Result<serde_json::Value, serde_json::Error> {
            serde_json::to_value(self)
        }
    }
}

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
            Some(
                BaseOrigin {
                    file: file.filepath.clone(),
                }
                .to_value()?,
            )
        };

        (frontmatter.properties, origin)
    } else {
        (
            None,
            Some(
                BaseOrigin {
                    file: file.filepath.clone(),
                }
                .to_value()?,
            ),
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
                RequirementOrigin { line }.to_value()?,
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
        let children = node.children().peekable();

        let req_fields: ReqFields = extract_req_fields(children)?;

        requirement.deprecated = req_fields.deprecated.unwrap_or_default();
        requirement.exclude = req_fields.exclude.unwrap_or_default();
        requirement.manual_verification = req_fields.manual_verification.unwrap_or_default();
        requirement.optional = req_fields.optional.unwrap_or_default();
        requirement.parents = req_fields.parents;
        requirement.properties = req_fields.properties;
        requirement.replaces = req_fields.replaces;
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

#[derive(Default)]
struct ReqFields {
    parents: Option<Vec<RequirementPk>>,
    manual_verification: Option<bool>,
    deprecated: Option<bool>,
    exclude: Option<bool>,
    optional: Option<bool>,
    replaces: Option<Vec<ReqId>>,
    properties: Option<Properties>,
}

enum FieldKey {
    Parents,
    ManualVerification,
    Deprecated,
    Exclude,
    Optional,
    Replaces,
    Properties,
}

impl FieldKey {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "parents" => FieldKey::Parents,
            "manual verification" | "manual" => FieldKey::ManualVerification,
            "deprecated" => FieldKey::Deprecated,
            "exclude" => FieldKey::Exclude,
            "optional" => FieldKey::Optional,
            "replaces" => FieldKey::Replaces,
            _ => FieldKey::Properties,
        }
    }
}

impl std::fmt::Display for FieldKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldKey::Parents => write!(f, "Parents"),
            FieldKey::ManualVerification => write!(f, "Manual Verification"),
            FieldKey::Deprecated => write!(f, "Deprecated"),
            FieldKey::Exclude => write!(f, "Exclude"),
            FieldKey::Optional => write!(f, "Optional"),
            FieldKey::Replaces => write!(f, "Replaces"),
            FieldKey::Properties => write!(f, "Properties"),
        }
    }
}

fn extract_req_fields(
    mut children: std::iter::Peekable<
        comrak::arena_tree::Children<'_, std::cell::RefCell<comrak::nodes::Ast>>,
    >,
) -> Result<ReqFields, anyhow::Error> {
    let mut fields = ReqFields::default();

    while let Some(node) = children.next()
        && let NodeValue::Item(list_item) = &node.data().value
        && list_item.list_type == ListType::Bullet
        && let Some(key_paragraph) = node.first_child()
        && matches!(key_paragraph.data().value, NodeValue::Paragraph)
        && let Some(first_child) = key_paragraph.first_child()
    {
        let mut para_children = key_paragraph.children();

        let (raw_key, raw_value) = if first_child.data().value == NodeValue::Strong
            && let Some(key_node) = first_child.first_child()
            && let NodeValue::Text(key_text) = &key_node.data().value
            && let Some((key, after_delim)) = key_text.split_once(':')
            && after_delim.is_empty()
        {
            let _ = para_children.next(); // skip key child

            let value = para_children.fold(String::new(), |mut base, n| {
                let mut s = String::new();
                let _ = comrak::format_commonmark(n, &Options::default(), &mut s);
                base.push_str(&s);
                base
            });

            (key.to_string(), value)
        } else if matches!(first_child.data().value, NodeValue::Text(_)) {
            let kv = para_children.fold(String::new(), |mut base, n| {
                let mut s = String::new();
                let _ = comrak::format_commonmark(n, &Options::default(), &mut s);
                base.push_str(&s);
                base
            });

            if let Some((key, value)) = kv.split_once(':') {
                (key.to_string(), value.to_string())
            } else {
                bail!(
                    "List key-value entry is missing the ':' delimiter at line '{}'",
                    first_child.data().sourcepos.start.line
                );
            }
        } else {
            bail!(
                "Keys in the requirement list must either be plain text or bold formatted and separated by ':' from the value"
            );
        };

        // comrak seems to escape '[]', so we need to unescape
        let raw_value = raw_value.trim().replace("\\[", "[").replace("\\]", "]");
        let field_key = FieldKey::from_str(&raw_key);

        match field_key {
            FieldKey::Parents => {
                // TODO: replace with proper parents field parsing
                // This is a workaround that uses the similarity of Rust attrbiute macros and the Parents field syntax.
                // Since Rust traces do not currently support trace properties and parent requirements are references like traces,
                // the syntax is identical.

                // Faking trace syntax by turning '["req-id"]' into '[req("req-id")]'
                let val = raw_value.replacen("[", "[req(", 1);
                let Some((modified_list, _)) = val.rsplit_once(']') else {
                    bail!("Parents list must end with ']'");
                };

                let annotations =
                    mantra_lang_tracing::collect::rust::RustCodeCollector::collect_relative(
                        &format!("#{})] fn foo() {{}}", modified_list.replace('\\', "")),
                        key_paragraph.data().sourcepos.start.line.try_into()?,
                    )?;

                if annotations.traces.is_empty() {
                    bail!("No parent IDs found in '{}'", raw_value);
                } else if annotations.traces.len() > 1 {
                    bail!("More than one list of requirements was detected.");
                }

                let parent_refs = annotations
                    .traces
                    .into_iter()
                    .next()
                    .expect("Checked above that exactly one entry exists");

                if fields.parents.is_some() {
                    bail!("Field 'Parents' was set more than once");
                } else {
                    fields.parents = Some(parent_refs.ids);
                }
            }
            FieldKey::ManualVerification => {
                update_bool(&mut fields.manual_verification, &raw_value)
                    .context("Key: Manual Verification")?
            }
            FieldKey::Deprecated => {
                update_bool(&mut fields.deprecated, &raw_value).context("Key: Deprecated")?
            }
            FieldKey::Exclude => {
                update_bool(&mut fields.exclude, &raw_value).context("Key: Exclude")?
            }
            FieldKey::Optional => {
                update_bool(&mut fields.optional, &raw_value).context("Key: Optional")?
            }
            FieldKey::Replaces => {
                let req_ids: Vec<ReqId> = json5::from_str(&raw_value)?;

                if fields.replaces.is_some() {
                    bail!("Field 'Replaces' was set more than once");
                } else {
                    fields.replaces = Some(req_ids);
                }
            }
            FieldKey::Properties => {
                eprintln!("{raw_value}");
                let value: serde_json::Value = json5::from_str(&raw_value)?;

                if let Some(properties) = &mut fields.properties {
                    properties.insert(raw_key.to_string(), value);
                } else {
                    let mut map = serde_json::Map::new();
                    map.insert(raw_key.to_string(), value);
                    fields.properties = Some(map);
                }
            }
        }
    }

    Ok(fields)
}

fn match_bool(s: &str) -> Result<bool, anyhow::Error> {
    match s.trim().to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => bail!("Expected either 'true' or 'false', but found '{}'", s),
    }
}

fn update_bool(b: &mut Option<bool>, new: &str) -> Result<(), anyhow::Error> {
    let new_b = match_bool(new)?;

    if b.is_some() {
        bail!("Duplicate entry! Key has been set previously");
    } else {
        *b = Some(new_b);
    }

    Ok(())
}
