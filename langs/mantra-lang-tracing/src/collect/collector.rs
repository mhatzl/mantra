use mantra_schema::{Line, annotations::Annotations};

pub trait AnnotationCollector {
    fn collect(content: &str) -> Result<Annotations, anyhow::Error> {
        Self::collect_relative(content, 1)
    }

    fn collect_relative(content: &str, start_line: Line) -> Result<Annotations, anyhow::Error>;
}
