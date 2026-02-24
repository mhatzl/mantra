use crate::{Properties, product::ProductId};

pub mod short;

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ReportProduct {
    /// The product ID.
    ///
    /// TODO: map to requirement
    pub id: ProductId,
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
    /// Optional description of the product.
    ///
    /// TODO: map to requirement
    pub description: Option<String>,
    /// Optional properties of the product.
    ///
    /// TODO: map to requirement
    pub properties: Option<Properties>,
}
