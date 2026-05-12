use std::str::FromStr;

use mantra_schema::{
    annotations::{Trace, TraceKind},
    requirements::ReqId,
};

use crate::collect::plain_text::PlainTextCollector;

macro_rules! mock_trace {
    (kind: $kind:expr, $($id:literal),+) => {
        mock_trace!(offset: 1, kind: $kind, $($id),+)
    };
    (offset: $line:literal, kind: $kind:expr, $($id:literal),+) => {
        Trace {
            ids: vec![$(ReqId::from_str($id).unwrap()),+],
            line: $line,
            related_code: None,
            kind: $kind,
            properties: None,
        }
    };
}

#[test]
fn satisfies_mock_trace() {
    let exp = Trace {
        ids: vec![ReqId::from_str("req.id").unwrap()],
        line: 1,
        related_code: None,
        kind: TraceKind::Satisfies,
        properties: None,
    };
    let mocked_trace = mock_trace!(kind: TraceKind::Satisfies, "req.id");

    assert_eq!(mocked_trace, exp, "Mocked trace differs from expected");
}

#[test]
fn verifies_mock_trace_mult_ids() {
    let exp = Trace {
        ids: vec![
            ReqId::from_str("req.id").unwrap(),
            ReqId::from_str("annotations.trace.mult-reqs").unwrap(),
        ],
        line: 1,
        related_code: None,
        kind: TraceKind::Verifies,
        properties: None,
    };
    let mocked_trace =
        mock_trace!(kind: TraceKind::Verifies, "req.id", "annotations.trace.mult-reqs");

    assert_eq!(mocked_trace, exp, "Mocked trace differs from expected");
}

#[test]
fn clarifies_mock_trace_with_offset() {
    let exp = Trace {
        ids: vec![ReqId::from_str("req.id").unwrap()],
        line: 10,
        related_code: None,
        kind: TraceKind::Clarifies,
        properties: None,
    };
    let mocked_trace = mock_trace!(offset: 10, kind: TraceKind::Clarifies, "req.id");

    assert_eq!(mocked_trace, exp, "Mocked trace differs from expected");
}

#[test]
fn single_satisfy_variant_one_trace_no_other_text() {
    let src = r#"[req("annotations.trace.plain-text")]"#;

    let collector = PlainTextCollector::new(src);
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn single_satisfy_variant_two_trace_no_other_text() {
    let src = r#"[req_satisfied("annotations.trace.plain-text")]"#;

    let collector = PlainTextCollector::new(src);
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn single_verify_variant_one_trace_no_other_text() {
    let src = r#"[req_verified("annotations.trace.plain-text")]"#;

    let collector = PlainTextCollector::new(src);
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn single_verify_variant_two_trace_no_other_text() {
    let src = r#"[req_test("annotations.trace.plain-text")]"#;

    let collector = PlainTextCollector::new(src);
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn single_clarify_trace_no_other_text() {
    let src = r#"[req_note("annotations.trace.plain-text")]"#;

    let collector = PlainTextCollector::new(src);
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn single_link_trace_no_other_text() {
    let src = r#"[req_link("annotations.trace.plain-text")]"#;

    let collector = PlainTextCollector::new(src);
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn single_trace_before_other_text() {
    let src = r#"[req("annotations.trace.plain-text")] followed by other text"#;
}

#[test]
fn single_trace_after_other_text() {
    let src = r#"Some text before trace [req("annotations.trace.plain-text")]"#;
}

#[test]
fn single_trace_after_mult_line_text() {
    let src =
        "Some text\nspanning multiple lines\n before trace [\"annotations.trace.plain-text\"]";
}

#[test]
fn two_ids_in_one_trace() {
    let src = r#"["annotations.trace.plain-text", "annotations.trace.mult-req-ids"]"#;
}

#[test]
fn trace_split_over_mult_lines() {
    let src = "[\"annotations.\ntrace.\nplain-text\"]";
}

#[test]
fn trace_streamed() {
    let src_part_1 = r#"["annotations."#;
    let src_part_2 = r#"trace.plain-text"]"#;

    let mut collector = PlainTextCollector::new(src_part_1);
    collector.add_lines(src_part_1.lines());
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn trace_with_offset() {
    let line_offset = 10;
    let src = r#"["annotations.trace.plain-text"]"#;

    let collector = PlainTextCollector::new(&src);
    let annotations = collector.annotations();

    assert_eq!(
        annotations.traces.first().unwrap(),
        &mock_trace!(offset: 10, kind: TraceKind::Satisfies, "annotations.trace.plain-text"),
        "Failed to detect requirements trace"
    );
}

#[test]
fn trace_over_mult_lines_with_offset() {
    let line_offset = 10;
    let src =
        "Some text\nspanning multiple lines\n before trace [\"annotations.trace.plain-text\"]";
}
