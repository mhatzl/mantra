use mantra_schema::Line;
use mantra_schema::annotations::Trace;

pub fn map_to_absolut_traces<'src>(
    rel_traces: Vec<PlainTextTrace<'src>>,
    line: Line,
) -> Vec<Trace> {
    rel_traces.into_iter().map(|rt| rt.to_trace(line)).collect()
}

pub struct PlainTextTrace<'src> {
    variant: &'src str,
    line: Line,
    ids: Vec<&'src str>,
}

impl<'src> PlainTextTrace<'src> {
    pub fn to_trace(self, line: Line) -> Trace {
        todo!()
    }
}
