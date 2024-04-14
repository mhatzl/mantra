use std::path::Path;

pub struct ReqTrace {
    req_id: String,
    line: u32,
}

impl ReqTrace {
    pub fn new(req_id: impl Into<String>, line: u32) -> Self {
        Self {
            req_id: req_id.into(),
            line,
        }
    }

    pub fn req_id(&self) -> &str {
        &self.req_id
    }

    pub fn line(&self) -> &u32 {
        &self.line
    }
}

pub trait TraceCollector {
    fn collect(filepath: &Path) -> Vec<ReqTrace>;
}
