use relative_path::RelativePathBuf;

use crate::{
    product::ProductId,
    report::{product::ProductMetadata, sources::SourceProductCoverageSummary},
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SourceFolderReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    #[schemars(with = "String")]
    pub path: RelativePathBuf,
    pub folder: Option<Vec<SourceChildReference>>,
    pub files: Option<Vec<SourceChildReference>>,
    pub product_coverage: Option<Vec<SourceFolderProductCoverage>>,
    pub collected_by: Vec<ProductMetadata>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SourceFolderProductCoverage {
    pub product: ProductMetadata,
    pub summary: SourceProductCoverageSummary,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SourceChildReference {
    pub collected_by: Vec<ProductId>,
    pub name: String,
}
