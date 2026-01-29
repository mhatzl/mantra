/// Type alias for a product ID.
///
/// TODO: map to requirement
pub type ProductId = String;

/// Defines the schema to exchange product related information.
/// [req("exchange.requirements.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ProductSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    /// List of products.
    pub products: Vec<Product>,
}

/// This struct defines the information *mantra* stores about a product.
///
/// TODO: map to requirement
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Product {
    /// The product ID.
    ///
    /// TODO: map to requirement
    pub id: Option<ProductId>,
    /// Baseline of the product. e.g. git branch or commit hash
    ///
    /// TODO: map to requirement
    pub base: String,
    /// The name of the product.
    ///
    /// TODO: map to requirement
    pub name: String,
    /// Optional version of the product.
    ///
    /// TODO: map to requirement
    pub version: Option<String>,
    /// Optional link to the homepage of the product.
    ///
    /// TODO: map to requirement
    pub homepage: Option<String>,
    /// Optional link to the repository of the product.
    ///
    /// TODO: map to requirement
    pub repository: Option<String>,
    /// Optional license of the product.
    ///
    /// TODO: map to requirement
    pub license: Option<String>,
    /// Optional metadata of the product.
    ///
    /// TODO: map to requirement
    pub metadata: Option<serde_json::Value>,
}
