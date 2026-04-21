use std::str::Lines;

use mantra_schema::{
    Line,
    annotations::{Annotations, TraceKind},
    requirements::ReqId,
};

use crate::collect::collector::AnnotationCollector;

#[cfg(test)]
mod tests;

pub struct PlainTextCollector {
    annotations: Annotations,
    start_line: Line,
    state: PlainTextCollectorState,
}

impl PlainTextCollector {
    pub fn new(content: &str) -> Self {
        Self::with_start_line(content, 1)
    }

    pub fn with_start_line(content: &str, start_line: Line) -> Self {
        PlainTextCollector {
            annotations: Annotations {
                traces: vec![],
                elements: vec![],
                coverage_excludes: vec![],
            },
            start_line,
            state: PlainTextCollectorState::Finished,
        }
    }

    pub fn annotations(&self) -> &Annotations {
        // TODO: log warn if partial annotation

        &self.annotations
    }

    pub fn add_lines(&mut self, lines: Lines<'_>) {}

    pub fn ends_with_partial_annotation(&self) -> bool {
        self.state == PlainTextCollectorState::Finished
    }
}

impl AnnotationCollector for PlainTextCollector {
    fn collect_relative(content: &str, start_line: Line) -> Result<Annotations, anyhow::Error> {
        let collector = PlainTextCollector::with_start_line(content, start_line);
        Ok(collector.annotations().clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlainTextCollectorState {
    Finished,
    PendingTrace(PendingTrace),
    PendingProperty {
        kind: TraceKind,
        ids: Vec<ReqId>,
        pending_property: PendingProperty,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingTrace {
    PendingKind(String),
    PendingId {
        kind: TraceKind,
        valid_ids: Vec<ReqId>,
        partial_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
// TODO
enum PendingProperty {}
