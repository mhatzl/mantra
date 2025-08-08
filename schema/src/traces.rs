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
    /// Optional definition line of an element
    /// this trace is related to in the source file.
    ///
    /// e.g. line of a function definition.
    /// [req("trace.element")]
    pub element_definition_line: Option<Line>,
    /// `true`: Marks that a trace *satisfies* the traced requirements.
    /// [req("trace.properties.satisfies`")]
    #[serde(default)]
    pub satisfies: bool,
    /// `true`: Marks that a trace *verifies* the traced requirements.
    /// [req("trace.properties.verifies")]
    #[serde(default)]
    pub verifies: bool,
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

        if let Some(line) = self.element_definition_line {
            write!(f, " Related element defined at line '{}'.", line)?;
        }

        Ok(())
    }
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
