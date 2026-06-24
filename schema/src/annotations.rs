use crate::path::RelativePathBuf;

use crate::{ConversionError, FmtHash, Line, LineSpan, Origin, Properties};

use super::requirements::ReqId;

/// Defines the schema to exchange mantra annotation related information.
/// [req("exchange.traces.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AnnotationSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    /// List of files that contain mantra annotations.
    pub files: Vec<FileAnnotations>,
    /// Optional properties related to detected traces in all files in this entry.
    ///
    /// **Note:** If a trace sets a property key directly,
    /// the value set at the trace will be taken.
    pub trace_properties: Option<Properties>,
    /// Optional base origin of the files in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<Origin>,
}

/// The annotation information per file.
/// [req("changes.track.traces.files")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileAnnotations {
    /// File that contains traces and/or elements.
    /// [req("trace.origin")]
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    /// Hash of the file content to detect changes.
    pub file_hash: FmtHash,
    /// Annotations found in the file.
    pub annotations: Annotations,
    /// Content of the file.
    pub content: Option<String>,
}

/// The annotation information mantra can collect.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Annotations {
    /// Traces detected in the file.
    #[serde(default)]
    pub traces: Vec<Trace>,
    /// Elements detected in the file.
    /// [req("trace.element", "testcov.static_approx")]
    #[serde(default)]
    pub elements: Vec<Element>,
    /// Coverage excludes detected in the file.
    ///
    /// TODO: add requirement trace
    #[serde(default)]
    pub coverage_excludes: Vec<CoverageExclude>,
}

/// Coverage exclusion information found in a file.
/// e.g. markers in code files may be used to exclude uncoverable lines from being considered for code coverage metrics.
///
/// TODO: add requirement trace
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct CoverageExclude {
    /// The kind of coverage exclusion.
    pub kind: CoverageExcludeKind,
    /// Mandatory comment on why the exclusion is acceptable.
    pub comment: String,
}

impl CoverageExclude {
    /// The start line the coverage exclusion starts.
    pub fn start_line(&self) -> Line {
        self.kind.start_line()
    }
}

/// The kind of coverage exclusion that was found in a file.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CoverageExcludeKind {
    /// Excludes a code span from coverage metrics.
    /// Both lines are inclusive!
    Block { start: Line, end: Line },
    /// Excludes one line from coverage metrics.
    Line(Line),
}

impl CoverageExcludeKind {
    /// The start line the coverage exclusion starts.
    pub fn start_line(&self) -> Line {
        match self {
            CoverageExcludeKind::Block { start, end: _ } => *start,
            CoverageExcludeKind::Line(line) => *line,
        }
    }
}

/// A *mantra* trace.
/// [req("trace")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
    /// List of custom properties that may be set on a trace.
    /// [req("trace.properties")]
    pub properties: Option<Properties>,
}

impl std::fmt::Display for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Traces req({}) at line '{}'.",
            self.ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(","),
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
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TraceKind {
    /// Trace links to an artifact that provides clarification for a requirement.
    Clarifies = 0,
    /// Trace links to an artifact that satisfies a requirement.
    Satisfies = 1,
    /// Trace links to an artifact that verifies a requirement.
    Verifies = 2,
    /// Trace link that provides no additional information.
    Links = 3,
}

impl TraceKind {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for TraceKind {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TraceKind::Clarifies),
            1 => Ok(TraceKind::Satisfies),
            2 => Ok(TraceKind::Verifies),
            3 => Ok(TraceKind::Links),
            _ => Err(ConversionError::UnknownKind),
        }
    }
}

/// Possible related code variants for a trace.
/// [req("trace.code_block", "trace.element")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(untagged)]
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CodeBlock {
    /// The kind of the code block.
    pub kind: CodeBlockKind,
    /// The line span of the code block.
    /// [req("trace.code_block.span")]
    pub span: LineSpan,
    /// The SHA256 content hash of the code block.
    pub content_hash: Option<FmtHash>,
}

/// The code block kind.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeBlockKind {
    Other = 0,
    If = 1,
    ElseIf = 2,
    Else = 3,
    Loop = 4,
    While = 5,
    For = 6,
    #[serde(alias = "switch", alias = "case")]
    Match = 7,
}

impl CodeBlockKind {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for CodeBlockKind {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CodeBlockKind::Other),
            1 => Ok(CodeBlockKind::If),
            2 => Ok(CodeBlockKind::ElseIf),
            3 => Ok(CodeBlockKind::Else),
            4 => Ok(CodeBlockKind::Loop),
            5 => Ok(CodeBlockKind::While),
            6 => Ok(CodeBlockKind::For),
            7 => Ok(CodeBlockKind::Match),
            _ => Err(ConversionError::UnknownKind),
        }
    }
}

/// A generic code element.
/// e.g. function, module, type, ...
/// [req("trace.element")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Element {
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
    /// The SHA256 content hash of the element.
    pub content_hash: Option<FmtHash>,
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.kind == ElementKind::Test {
            write!(f, "test: ")?;
        }

        write!(f, "`{}` @{}..{}", self.name, self.span.start, self.span.end)
    }
}

/// Defines supported element kinds.
/// [req("trace.element.kind")]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    /// Variant that should be used if no other one fits.
    Other = 0,
    /// Marks an element as a test or test case.
    #[serde(alias = "test_case")]
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
    #[serde(alias = "interface", alias = "abstract_type")]
    Trait = 8,
    /// A function signature or virtual function that has no *body*. It is likely declared inside a trait/interface.
    #[serde(alias = "virtual_function")]
    FunctionSignature = 9,
}

impl ElementKind {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for ElementKind {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ElementKind::Other),
            1 => Ok(ElementKind::Test),
            2 => Ok(ElementKind::Module),
            3 => Ok(ElementKind::Function),
            4 => Ok(ElementKind::Variable),
            5 => Ok(ElementKind::Const),
            6 => Ok(ElementKind::Type),
            7 => Ok(ElementKind::Field),
            8 => Ok(ElementKind::Trait),
            _ => Err(ConversionError::UnknownKind),
        }
    }
}
