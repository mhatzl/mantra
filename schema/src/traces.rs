use std::path::PathBuf;

use crate::{Line, LineSpan};

use super::requirements::ReqId;

/// Defines the schema to exchange trace related information.
/// [req("exchange.traces.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TraceSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    /// List of files that contain trace and element information.
    pub files: Vec<FileTraceInfo>,
    /// Optional metadata related to all files in this entry.
    pub metadata: Option<serde_json::Value>,
    /// Optional base origin of the files in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<serde_json::Value>,
}

/// The trace information per file.
/// [req("changes.track.traces.files")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct FileTraceInfo {
    /// File that contains traces and/or elements.
    /// [req("trace.origin")]
    pub filepath: PathBuf,
    /// Hash of the file content to detect changes.
    pub file_hash: String,
    /// Traces detected in the file.
    #[serde(default)]
    pub traces: Vec<Trace>,
    /// Elements detected in the file.
    /// [req("trace.element", "testcov.static_approx")]
    #[serde(default)]
    pub elements: Vec<Element>,
}

/// A *mantra* trace.
/// [req("trace")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct Trace {
    /// The requirement IDs that are referenced by the trace.
    /// [req("trace.id", "trace.mult_reqs")]
    pub ids: Vec<ReqId>,
    /// The line the trace is defined at.
    /// [req("trace.origin")]
    pub line: Line,
    /// Optional related code block or element that is linked to the trace.
    /// [req("trace.code_block", "trace.element")]
    pub related_code: Option<TraceRelatedCodeVariant>,
    /// Trace kind.
    /// [req("trace.kind`")]
    pub kind: TraceKind,
    /// List of (custom) properties that may be set on a trace.
    /// [req("trace.properties")]
    #[serde(default)]
    pub properties: Vec<serde_json::Value>,
}

impl std::fmt::Display for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Traces req({}) at line '{}'.",
            self.ids.join(","),
            self.line
        )?;

        if let Some(code) = &self.related_code {
            match code {
                TraceRelatedCodeVariant::CodeBlock(code_block) => write!(
                    f,
                    " Related code block spans lines '{}..{}'.",
                    code_block.span.start, code_block.span.end
                )?,
                TraceRelatedCodeVariant::ElementAtLine(line) => {
                    write!(f, " Related element defined at line '{line}'.")?
                }
            }
        }

        Ok(())
    }
}

/// The trace kind.
/// [req("trace.kind")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum TraceKind {
    /// Trace links to an artifact that provides clarification for a requirement.
    Clarifies = 0,
    /// Trace links to an artifact that satisfies a requirement.
    Satisfies = 1,
    /// Trace links to an artifact that verifies a requirement.
    Verifies = 2,
}

/// Possible related code variants for a trace.
/// [req("trace.code_block", "trace.element")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum TraceRelatedCodeVariant {
    /// Code block that is linked to the trace.
    /// [req("trace.code_block")]
    CodeBlock(CodeBlock),
    /// Definition line of an element the trace is related to in the source file.
    ///
    /// e.g. line of a function definition.
    /// [req("trace.element")]
    ElementAtLine(Line),
}

/// A generic code block that is linked to a trace.
/// [req("trace.code_block")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct CodeBlock {
    /// The content of the code block.
    /// [req("report.coverage.content"]
    pub content: Option<String>,
    /// The line span of the code block.
    /// [req("trace.code_block.span")]
    pub span: LineSpan,
}

/// A generic code element.
/// e.g. function, module, type, ...
/// [req("trace.element")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct Element {
    /// Identifier of the element.
    ///
    /// **Note:** Might not be the fully qualified identifier.
    /// [req("trace.element.ident")]
    pub ident: String,
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
    /// The content of the element.
    /// [req("report.coverage.content"]
    pub content: Option<String>,
    /// Optional references of the element at other locations.
    /// [req("testcov.static_approx")]
    #[serde(default)]
    pub references: Vec<ElementReference>,
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.kind == ElementKind::Test {
            write!(f, "test: ")?;
        }

        write!(
            f,
            "`{}` @{}..{}",
            self.ident, self.span.start, self.span.end
        )
    }
}

/// [req("trace.element.kind")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct ElementReference {
    /// The filepath where the element is referenced in.
    pub filepath: PathBuf,
    /// Hash of the file content to detect changes.
    pub file_hash: String,
    /// Line the elemenet is referenced at.
    pub line: Line,
}

/// Defines supported element kinds.
/// [req("trace.element.kind")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ElementKind {
    /// Variant that should be used if no other one fits.
    Other = 0,
    /// Marks an element as a test or test case.
    #[serde(alias = "test-case")]
    Test = 1,
    /// A module or package.
    #[serde(alias = "mod", alias = "package")]
    Module = 2,
    /// A function or method.
    #[serde(alias = "fn", alias = "method")]
    Function = 3,
    /// A variable or static.
    #[serde(alias = "var", alias = "static")]
    Variable = 4,
    /// A constant.
    Const = 5,
    /// A type, struct, enum, class, or union.
    #[serde(alias = "struct", alias = "enum", alias = "class", alias = "union")]
    Type = 6,
    /// A field or property.
    #[serde(alias = "property")]
    Field = 7,
    /// A trait, interface, or other abstract type.
    #[serde(alias = "interface", alias = "abstract-type")]
    Trait = 8,
}
