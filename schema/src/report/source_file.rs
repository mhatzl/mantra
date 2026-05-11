use relative_path::RelativePathBuf;

use crate::{
    FmtHash, Line,
    report::{
        annotations::ResolvedAnnotations, product::ProductMetadata, review::ReviewReference,
        sources::SourceProductCoverageSummary, tests::TestReference,
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
    pub hashed_info: Vec<HashedSourceFileInfo>,
    pub product_coverage: Option<Vec<SourceFileProductCoverage>>,
    pub collected_by: Vec<ProductMetadata>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct HashedSourceFileInfo {
    pub file_hash: FmtHash,
    pub content: Option<String>,
    pub annotations: Option<ResolvedAnnotations>,
    pub product_coverage: Option<Vec<SourceFileProductCoverage>>,
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
    pub nr: Line,
    pub state: ResolvedLineState,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedLineState {
    Covered(Vec<CoveredLineTestReference>),
    /// Line was excluded from coverage analysis.
    /// An optional reference to an exclusion annotation is given,
    /// if the exclusion was based on a mantra annotation.
    Excluded(Option<ExclusionAnnotationReference>),
    Overriden(Vec<ReviewOverride>),
    Uncovered,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoveredLineTestReference {
    pub test: TestReference,
    pub hits: i64,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ReviewOverride {
    pub review: ReviewReference,
    /// The original hits that were collected from the tests.
    pub original_hits: Option<i64>,
    /// The hits set by the review.
    pub set_hits: Option<i64>,
    pub comment: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ExclusionAnnotationReference {
    /// The line the exclude annotation was defined at.
    pub def_line: Line,
    pub comment: String,
}
