use mantra_schema::traces::LineSpan;

#[cfg(feature = "collect")]
pub mod collect;

#[cfg(feature = "extract")]
pub mod extract;

pub mod path;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RawTraceEntry<'a> {
    /// String containing requirement IDs.
    /// The format is defined in the README section [specifying requirement IDs](https://github.com/mhatzl/mantra/tree/main/langs/mantra-lang-tracing#specifying-requirement-ids).
    ids: &'a str,
    /// The line the trace is defined
    line: usize,
    /// Optional span of lines this entry affects in the source.
    ///
    /// e.g. lines of a function body for a trace set at start of the function.
    line_span: Option<LineSpan>,
}

impl<'a> RawTraceEntry<'a> {
    pub fn new(ids: &'a str, line: usize, line_span: Option<LineSpan>) -> Self {
        Self {
            ids,
            line,
            line_span,
        }
    }
}
