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
pub struct TraceEntry {
    pub ids: Vec<ReqId>,
    /// The line the trace is defined
    pub line: Line,
    /// Optional span of lines this entry affects in the source.
    ///
    /// e.g. lines of a function body for a trace set at start of the function.
    pub line_span: Option<LineSpan>,
    /// Optional name that is linked to this trace entry
    pub item_name: Option<String>,
}

impl std::fmt::Display for TraceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "req({}) at '{}'", self.ids.join(","), self.line)?;

        if let Some(span) = self.line_span {
            write!(f, " spans lines '{}:{}'", span.start, span.end)?;
        }

        Ok(())
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TraceSchema {
    pub traces: Vec<FileTraces>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct FileTraces {
    pub filepath: PathBuf,
    pub traces: Vec<TraceEntry>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TracePk {
    pub req_id: ReqId,
    pub filepath: PathBuf,
    pub line: Line,
}

impl std::fmt::Display for TracePk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "id=`{}`, file='{}', line='{}'",
            self.req_id,
            self.filepath.display(),
            self.line
        )
    }
}
