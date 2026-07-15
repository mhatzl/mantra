use mantra_schema::{Origin, Properties, product::ProductId};

/// The frontmatter delimiter for Markdown
pub const MD_FRONTMATTER_DELIMITER: &str = "+++";

/// Frontmatter that may be set at the start of markup documents.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Frontmatter {
    /// The mantra schema version the markup document was written for.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "mantra_schema::serialize_schema_version")]
    pub mantra_schema: Option<String>,
    pub product_id: Option<ProductId>,
    pub properties: Option<Properties>,
    /// Optional origin that is applied to all collected items of the markup document
    pub origin: Option<Origin>,
}
