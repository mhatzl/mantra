use mantra_schema::Line;

#[cfg(feature = "collect")]
pub mod collect;
#[cfg(feature = "collect")]
pub mod lsif_graph;

#[cfg(feature = "extract")]
pub mod extract;

pub mod path;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RawTraceEntry<'a> {
    /// String containing requirement IDs.
    /// The format is defined in the README section [specifying requirement IDs](https://github.com/mhatzl/mantra/tree/main/langs/mantra-lang-tracing#specifying-requirement-ids).
    ids: &'a str,
    /// The line the trace is defined
    line: Line,
    /// Optional start line of an item this trace entry is related to in the source file.
    ///
    /// e.g. start line of a function definition.
    item_start_line: Option<Line>,
}

impl<'a> RawTraceEntry<'a> {
    pub fn new(ids: &'a str, line: Line, item_start_line: Option<Line>) -> Self {
        Self {
            ids,
            line,
            item_start_line,
        }
    }
}
