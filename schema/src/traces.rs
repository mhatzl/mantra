use std::path::PathBuf;

use mantra_lang_tracing::TraceEntry;

use super::requirements::ReqId;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TraceSchema {
    pub traces: Vec<FileTraces>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileTraces {
    pub filepath: PathBuf,
    pub traces: Vec<TraceEntry>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TracePk {
    pub req_id: ReqId,
    pub filepath: PathBuf,
    pub line: u32,
}
