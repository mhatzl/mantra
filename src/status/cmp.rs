//! Contains the status overview for the comparison between two branches.
//!
//! [req:status.cmp]

use crate::wiki::{
    ref_list::{RefCntKind, RefListEntry},
    Wiki,
};

/// Creates an overview for the comparison between two branches in the wiki.
///
/// [req:status.cmp]
pub fn status_cmp(wiki: &Wiki, branch_a: &str, branch_b: &str) -> String {
    let mut differences = Vec::new();

    let req_column_head = "REQ-ID";
    let mut max_req_column_width = req_column_head.len();
    let mut max_branch_a_column_width = branch_a.len();
    let mut max_branch_b_column_width = branch_b.len();

    for req in wiki.reqs() {
        let phase_a = req_phase(&req.ref_list, branch_a);
        let phase_b = req_phase(&req.ref_list, branch_b);

        if phase_a != phase_b {
            max_req_column_width = max_req_column_width.max(req.head.id.len());
            max_branch_a_column_width = max_branch_a_column_width.max(phase_a.len());
            max_branch_b_column_width = max_branch_b_column_width.max(phase_b.len());

            differences.push((req.head.id.clone(), phase_a, phase_b));
        }
    }

    if differences.is_empty() {
        return format!("No differences between `{}` and `{}`.", branch_a, branch_b);
    }

    let mut status = format!(
        "**Wiki differences between `{}` and `{}`:**

| {}{} | {}{} | {}{} |
| {} | {} | {} |\n",
        branch_a,
        branch_b,
        req_column_head,
        " ".repeat(max_req_column_width - req_column_head.len()),
        branch_a,
        " ".repeat(max_branch_a_column_width - branch_a.len()),
        branch_b,
        " ".repeat(max_branch_b_column_width - branch_b.len()),
        "-".repeat(max_req_column_width),
        "-".repeat(max_branch_a_column_width),
        "-".repeat(max_branch_b_column_width),
    );

    for (req_id, phase_a, phase_b) in differences {
        let req_spaces = " ".repeat(max_req_column_width - req_id.len());
        let branch_a_spaces = " ".repeat(max_branch_a_column_width - phase_a.len());
        let branch_b_spaces = " ".repeat(max_branch_b_column_width - phase_b.len());

        status.push_str(&format!(
            "| {}{} | {}{} | {}{} |",
            req_id, req_spaces, phase_a, branch_a_spaces, phase_b, branch_b_spaces
        ));
    }

    status
}

/// Returns the *phase* of the requirement in the given branch.
fn req_phase(ref_list: &[RefListEntry], branch: &str) -> String {
    let phase = match ref_list
        .iter()
        .find(|entry| entry.proj_line.branch_name.as_str() == branch)
    {
        Some(entry) => {
            if entry.is_deprecated {
                "deprecated"
            } else if entry.is_manual {
                if entry.ref_cnt == RefCntKind::Untraced {
                    "manual"
                } else {
                    "manual-traced"
                }
            } else if entry.ref_cnt == RefCntKind::Untraced {
                "ready"
            } else {
                "active"
            }
        }
        None => "ready",
    };

    phase.to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    fn setup_deprecated_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# ref_req: Some Title

**References:**

- in branch main: 2
- in branch stable: 2 (0 direct)

## ref_req.test: Some Title

**References:**

- in branch main: deprecated
- in branch stable: 2
        "#;

        Wiki::try_from((std::path::PathBuf::from(filename), content)).unwrap()
    }

    #[test]
    fn deprecated_req_in_new_branch() {
        let wiki = setup_deprecated_wiki();

        let status = status_cmp(&wiki, "main", "stable");

        assert_eq!(
            status,
            "**Wiki differences between `main` and `stable`:**

| REQ-ID       | main       | stable |
| ------------ | ---------- | ------ |
| ref_req.test | deprecated | active |",
            "Generated status differs for deprecated wiki entry."
        );
    }
}
