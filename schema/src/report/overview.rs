use relative_path::RelativePathBuf;
use time::OffsetDateTime;

use crate::{
    FmtHash, Line, LineSpan, Origin, Properties,
    annotations::{CoverageExclude, ElementKind, TraceKind, TraceRelatedCodeVariant},
    product::{Product, ProductId},
    report::{
        Aggregated, RequirementReference, RequirementState, ReviewReference, TestCaseReference,
        TestRunReference, TraceReference,
    },
    requirements::ReqId,
    reviews::OverrideTestRun,
    test_runs::{TestCaseLocation, TestState},
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ProductsOverviewReport {
    pub summary: ProductsSummary,
    pub product_reports: Vec<ProductOverviewReport>,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ProductsSummary {
    pub requirements: RequirementsSummary,
    pub test_cases: TestCasesSummary,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ProductOverviewReport {
    pub product: Product,
    pub annotations: AnnotationsOverview,
    pub requirements: RequirementsOverview,
    pub test_runs: TestRunsOverview,
    pub reviews: ReviewsOverview,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AnnotationsOverview {
    pub traces: TracesOverview,
    pub elements: ElementsOverview,
    pub coverage_excludes: CoverageExcludesOverview,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ElementsOverview {
    pub files: Vec<ElementsPerFile>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ElementsPerFile {
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    pub file_hash: FmtHash,
    pub elements: Vec<ElementOverview>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ElementOverview {
    /// The fully qualified identifier of the element.
    /// [req("trace.element.ident")]
    pub ident: Option<String>,
    /// The element name.
    ///
    /// **Note:** This is not the fully qualified identifier.
    pub name: String,
    /// The line the element is defined at.
    ///
    /// **Note:** This might differ from `span.start`,
    /// because in Rust for example, attributes & doc-comments are part of the span,
    /// but the definition of an element starts below them.
    ///
    /// TODO: trace req
    pub definition_line: Line,
    /// The line span of the element.
    /// [req("trace.element.span")]
    pub span: LineSpan,
    /// The kind of the element.
    /// [req("trace.element.kind")]
    pub kind: ElementKind,
    pub covered_by: Option<CoveredByTestsOverview>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CoverageExcludesOverview {
    pub files: Vec<CoverageExcludesPerFile>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoverageExcludesPerFile {
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    pub file_hash: FmtHash,
    pub excludes: Vec<CoverageExclude>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TracesOverview {
    pub summary: TracesSummary,
    pub files: Vec<TracesPerFile>,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TracesSummary {
    pub total: i64,
    pub satisfies: Aggregated,
    pub verifies: Aggregated,
    pub clarifies: Aggregated,
    pub links: Aggregated,
}

impl TracesSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.satisfies.cnt += other.satisfies.cnt;
        self.verifies.cnt += other.verifies.cnt;
        self.clarifies.cnt += other.clarifies.cnt;
        self.links.cnt += other.links.cnt;

        self.satisfies.percentage = percentage!(self.satisfies.cnt, self.total);
        self.verifies.percentage = percentage!(self.verifies.cnt, self.total);
        self.clarifies.percentage = percentage!(self.clarifies.cnt, self.total);
        self.links.percentage = percentage!(self.links.cnt, self.total);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TracesPerFile {
    pub summary: TracesSummary,
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    pub file_hash: FmtHash,
    pub traces: Vec<TraceOverview>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TraceOverview {
    /// The requirement IDs that are referenced by the trace.
    /// [req("trace.id", "trace.mult_reqs")]
    pub ids: Vec<RequirementReference>,
    /// The line the trace is defined at.
    /// [req("trace.origin")]
    pub line: Line,
    /// Optional related code block or element that is linked to the trace.
    /// [req("trace.code_block", "trace.element")]
    pub related_code: Option<TraceRelatedCodeVariant>,
    /// Trace kind.
    /// [req("trace.kind`")]
    pub kind: TraceKind,
    /// List of custom properties that may be set on a trace.
    /// [req("trace.properties")]
    pub properties: Option<Properties>,
    pub covered_by: Option<CoveredByTestsOverview>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoveredByTestsOverview {
    pub test_runs: Vec<TestRunReference>,
    pub test_cases: Vec<TestCaseReference>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementsOverview {
    pub summary: RequirementsSummary,
    pub roots: Vec<RequirementOverview>,
    pub all: Vec<RequirementOverview>,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct RequirementsSummary {
    pub total: i64,
    /// Metric for how many non-optional requirements are verified.
    pub mandatory_verified: Aggregated,
    pub verified: Aggregated,
    pub failed: Aggregated,
    pub skipped: Aggregated,
    pub unverified: Aggregated,
    pub deprecated: Aggregated,
    pub ignored: Aggregated,
    pub manual_verification: Aggregated,
}

#[macro_export]
macro_rules! percentage {
    ($val:expr, $total:expr) => {
        ($val as f32 / $total as f32) * 100.0
    };
}
pub use percentage;

impl RequirementsSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.verified.cnt += other.verified.cnt;
        self.mandatory_verified.cnt += other.mandatory_verified.cnt;
        self.failed.cnt += other.failed.cnt;
        self.skipped.cnt += other.skipped.cnt;
        self.unverified.cnt += other.unverified.cnt;
        self.deprecated.cnt += other.deprecated.cnt;
        self.ignored.cnt += other.ignored.cnt;
        self.manual_verification.cnt += other.manual_verification.cnt;

        self.verified.percentage = percentage!(self.verified.cnt, self.total);
        self.mandatory_verified.percentage = percentage!(self.mandatory_verified.cnt, self.total);
        self.failed.percentage = percentage!(self.failed.cnt, self.total);
        self.skipped.percentage = percentage!(self.skipped.cnt, self.total);
        self.unverified.percentage = percentage!(self.unverified.cnt, self.total);
        self.deprecated.percentage = percentage!(self.deprecated.cnt, self.total);
        self.ignored.percentage = percentage!(self.ignored.cnt, self.total);
        self.manual_verification.percentage = percentage!(self.manual_verification.cnt, self.total);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RequirementOverview {
    pub id: ReqId,
    pub title: String,
    pub state: RequirementState,
    pub optional: bool,
    pub manual_verification: bool,
    pub description: Option<String>,
    pub base_origin: Option<Origin>,
    pub origin: Option<Origin>,
    pub parents: Option<Vec<RequirementReference>>,
    pub children: Option<Vec<RequirementReference>>,
    pub traces: Option<RequirementTracesOverview>,
    pub covered_by: Option<CoveredByTestsOverview>,
    pub reviewed_in: Option<Vec<ReviewReference>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementTracesOverview {
    pub summary: TracesSummary,
    pub all: Vec<TraceReference>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestRunsOverview {
    pub test_cases_summary: TestCasesSummary,
    pub all: Vec<TestRunOverview>,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestCasesSummary {
    pub total: i64,
    pub passed: Aggregated,
    pub failed: Aggregated,
    pub skipped: Aggregated,
    pub unknown: Aggregated,
    pub obsolete: Aggregated,
}

impl TestCasesSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.passed.cnt += other.passed.cnt;
        self.failed.cnt += other.failed.cnt;
        self.skipped.cnt += other.skipped.cnt;
        self.unknown.cnt += other.unknown.cnt;
        self.obsolete.cnt += other.obsolete.cnt;

        self.passed.percentage = percentage!(self.passed.cnt, self.total);
        self.failed.percentage = percentage!(self.failed.cnt, self.total);
        self.skipped.percentage = percentage!(self.skipped.cnt, self.total);
        self.unknown.percentage = percentage!(self.unknown.cnt, self.total);
        self.obsolete.percentage = percentage!(self.obsolete.cnt, self.total);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestRunOverview {
    pub name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub state: TestState,
    pub test_cases: Option<TestCasesOverview>,
    pub parents: Option<Vec<TestRunReference>>,
    pub children: Option<Vec<TestRunReference>>,
    pub related_reqs: Option<Vec<TestRelatedRequirementOverview>>,
    pub coverage: Option<TestCoverageOverview>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TestCoverageOverview {
    pub summary: TestCoverageSummary,
    pub covered_files: Vec<ResolvedCoveredFile>,
    pub covered_traces: Option<Vec<TraceReference>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedCoveredFile {
    /// File the coverage information is for.
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    /// Optional hash of the file content to detect changes.
    /// Coverage formats may not provide the file hash, therefore it must be optional.
    pub file_hash: Option<FmtHash>,
    /// Coverage information for a line in the file.
    /// [req("testcov.cov.lines")]
    pub lines: ResolvedCoveredLines,
}

/// Coverage information of a line in a file.
/// [req("testcov.cov.lines")]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResolvedCoveredLines {
    pub summary: CoveredLinesSummary,
    pub lines: Vec<ResolvedCoveredLine>,
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

/// Coverage information of a line in a file.
/// [req("testcov.cov.lines")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ResolvedCoveredLine {
    /// The line number.
    pub nr: Line,
    pub state: ResolvedCoveredLineState,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedCoveredLineState {
    Covered(i64),
    /// Line was excluded from coverage analysis.
    /// An optional reference to an exclusion annotation is given,
    /// if the exclusion was based on a mantra annotation.
    Excluded(Option<ExclusionAnnotationReference>),
    Overriden {
        review: ReviewReference,
        /// The original hits that were collected from the tests.
        original_hits: Option<i64>,
        /// The hits set by the review.
        set_hits: Option<i64>,
        comment: String,
    },
    Uncovered,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ExclusionAnnotationReference {
    /// The line the exclude annotation was defined at.
    pub def_line: Line,
    pub comment: String,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestCoverageSummary {
    pub lines: CoveredLinesSummary,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestCasesOverview {
    pub summary: TestCasesSummary,
    pub all: Vec<TestCaseOverview>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TestCaseOverview {
    pub name: String,
    pub state: TestState,
    pub location: Option<TestCaseLocation>,
    pub related_reqs: Option<Vec<TestRelatedRequirementOverview>>,
    pub coverage: Option<TestCoverageOverview>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestRelatedRequirementOverview {
    pub product_id: Option<ProductId>,
    pub id: ReqId,
    pub kind: TestRelatedRequirementKind,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TestRelatedRequirementKind {
    Direct,
    Traced(Vec<TraceReference>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ReviewsOverview {
    pub summary: ReviewsSummary,
    pub all: Vec<ReviewOverview>,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReviewsSummary {
    pub total: i64,
    pub valid: Aggregated,
    pub obsolete: Aggregated,
    pub mandatory_requirements_verified: Aggregated,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReviewOverview {
    pub name: String,
    /// The UTC date and time the review was started.
    /// [req("review.id")]
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: OffsetDateTime,
    pub authors: Vec<String>,
    pub description: String,
    pub requirements: Option<Vec<VerifiedRequirementOverview>>,
    pub test_run_overrides: Option<Vec<OverrideTestRun>>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct VerifiedRequirementOverview {
    pub id: ReqId,
    pub comment: String,
}
