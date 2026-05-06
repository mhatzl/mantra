use relative_path::RelativePathBuf;

use crate::{product::ProductId, report::Aggregated};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct SourceReference {
    #[schemars(with = "String")]
    pub path: RelativePathBuf,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SourceProductCoverageSummary {
    pub product_id: ProductId,
    pub lines: CoveredLinesSummary,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct CoveredLinesSummary {
    pub total: i64,
    pub covered: Aggregated,
    pub excluded: Aggregated,
    pub overridden: Aggregated,
    pub uncovered: Aggregated,
}
