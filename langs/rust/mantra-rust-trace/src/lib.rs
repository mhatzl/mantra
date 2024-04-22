use mantra_lang_tracing::TraceEntry;
use tree_sitter::Node;

pub fn collect_traces_in_rust(node: &Node, src: &[u8], _args: &()) -> Option<Vec<TraceEntry>> {
    let node_kind = node.kind();

    if node_kind == "attribute" || node_kind == "macro_invocation" {
        let ident = node.named_child(0)?;
        let macro_content = node.named_child(1)?;

        if ident.kind() == "identifier"
            && ident.utf8_text(src) == Ok("req")
            && macro_content.kind() == "token_tree"
        {
            return Some(vec![TraceEntry::try_from((
                macro_content
                    .utf8_text(src)
                    .ok()?
                    .strip_prefix('(')
                    .map(|s| s.strip_suffix(')').unwrap_or(s))?,
                (ident.start_position().row + 1),
            ))
            .ok()?]);
        }
    } else if node_kind.contains("comment") {
        let trace_matcher = mantra_lang_tracing::req_trace_matcher();
        let comment_content = node.utf8_text(src).ok()?;

        if !(comment_content.starts_with("///") || comment_content.starts_with("//!")) {
            return None;
        }

        let mut traces = Vec::new();

        for (i, line_content) in comment_content.lines().enumerate() {
            for capture in trace_matcher.captures_iter(line_content) {
                traces.push(
                    TraceEntry::try_from((
                        capture.name("ids")?.as_str(),
                        (node.start_position().row + i + 1),
                    ))
                    .ok()?,
                )
            }
        }

        return Some(traces);
    }

    None
}
