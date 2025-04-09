use std::path::PathBuf;

use crate::Line;

use super::requirements::ReqId;

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
pub struct LineSpan {
    pub start: Line,
    pub end: Line,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ItemEntry {
    /// Name of the item.
    pub ident: String,
    /// The line span of the item.
    pub span: LineSpan,
    /// Indicates if this item is a test.
    pub is_test: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TraceEntry {
    pub ids: Vec<ReqId>,
    /// The line the trace is defined at
    pub line: Line,
    /// Optional start line of an item this trace entry is related to in the source file.
    ///
    /// e.g. start line of a function definition.
    #[serde(alias = "item-start-line")]
    pub item_start_line: Option<Line>,
}

impl std::fmt::Display for TraceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Traces req({}) at line '{}'.",
            self.ids.join(","),
            self.line
        )?;

        if let Some(line) = self.item_start_line {
            write!(f, " Related item starts at line '{}'.", line)?;
        }

        Ok(())
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TraceSchema {
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    pub files: Vec<FileTraces>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct FileTraces {
    pub filepath: PathBuf,
    #[serde(default)]
    pub traces: Vec<TraceEntry>,
    #[serde(default)]
    pub items: Vec<ItemEntry>,
}
