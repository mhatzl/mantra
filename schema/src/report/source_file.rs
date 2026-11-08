use relative_path::RelativePathBuf;

use crate::{
    Line,
    report::{
        product::ProductMetadata, review::ReviewReference, sources::SourceProductCoverageSummary,
        tests::TestReference,
    },
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SourceFileReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    pub product_coverage: Option<Vec<SourceFileProductCoverage>>,
    pub collected_by: Vec<ProductMetadata>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SourceFileProductCoverage {
    pub product: ProductMetadata,
    pub summary: SourceProductCoverageSummary,
    pub lines: Vec<SourceLineInfo>,
}

/// Coverage information of a line in a file.
/// [req("testcov.cov.lines")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct SourceLineInfo {
    /// The line number.
    pub number: Line,
    pub state: ResolvedLineState,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedLineState {
    Covered(Vec<TestReference>),
    /// Line was excluded from coverage analysis by a mantra annotation.
    Excluded(ExclusionAnnotationReference),
    Overriden(Vec<ReviewOverride>),
    Uncovered,
    Unknown,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ReviewOverride {
    pub review: ReviewReference,
    /// The original hits that were collected from the tests.
    pub original_hits: Option<i64>,
    /// The coverage set by the review.
    pub set_coverage: Option<bool>,
    pub rational: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ExclusionAnnotationReference {
    /// The line the exclude annotation was defined at.
    pub def_line: Line,
    pub rational: String,
}
