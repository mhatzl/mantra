use relative_path::RelativePathBuf;

use crate::{
    FmtHash, Line, LineSpan, Properties,
    annotations::{CoverageExclude, ElementKind, TraceKind, TraceRelatedCodeVariant},
    product::ProductId,
    report::{Aggregated, requirement::RequirementReference},
    requirements::ReqId,
};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TraceReference {
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    pub file_hash: FmtHash,
    pub line: Line,
    pub kind: TraceKind,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
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

        self.update_percentages();
    }

    pub fn update_percentages(&mut self) {
        self.satisfies.update_percentage(self.total);
        self.verifies.update_percentage(self.total);
        self.clarifies.update_percentage(self.total);
        self.links.update_percentage(self.total);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedAnnotations {
    pub traces: Option<Vec<ResolvedTrace>>,
    pub elements: Option<Vec<ResolvedElement>>,
    pub coverage_excludes: Option<Vec<CoverageExclude>>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedTrace {
    /// The requirement IDs that are referenced by the trace.
    /// [req("trace.id", "trace.mult_reqs")]
    pub resolved_ids: Vec<RequirementReference>,
    pub unknown_ids: Vec<ReqId>,
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
    pub base_properties: Option<Properties>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedElement {
    /// The fully qualified identifier of the element.
    /// [req("trace.element.ident")]
    pub idents: Option<Vec<ResolvedElementIdent>>,
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
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedElementIdent {
    pub ident: String,
    pub product_ids: Vec<ProductId>,
}
