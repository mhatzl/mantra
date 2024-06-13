use std::path::PathBuf;

use mantra_lang_tracing::{Line, TraceEntry};

use super::requirements::ReqId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TraceSchema {
    pub traces: Vec<FileTraces>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FileTraces {
    pub filepath: PathBuf,
    pub traces: Vec<TraceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
