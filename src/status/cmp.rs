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
pub fn status_cmp(
    wiki: &Wiki,
    repo_a: Option<&str>,
    branch_a: &str,
    repo_b: Option<&str>,
    branch_b: &str,
) -> String {
    let mut differences = Vec::new();

    let column_a_head = match repo_a {
        Some(repo) => format!("{repo}/{branch_a}"),
        None => branch_a.to_string(),
    };
    let column_b_head = match repo_b {
        Some(repo) => format!("{repo}/{branch_b}"),
        None => branch_b.to_string(),
    };

    let req_column_head = "REQ-ID";
    let mut max_req_column_width = req_column_head.len();
    let mut max_branch_a_column_width = column_a_head.len();
    let mut max_branch_b_column_width = column_b_head.len();

    for req in wiki.reqs() {
        let phase_a = req_phase(&req.ref_list, repo_a, branch_a);
        let phase_b = req_phase(&req.ref_list, repo_b, branch_b);

        if phase_a != phase_b {
            max_req_column_width = max_req_column_width.max(req.head.id.len());
            max_branch_a_column_width = max_branch_a_column_width.max(phase_a.len());
            max_branch_b_column_width = max_branch_b_column_width.max(phase_b.len());

            differences.push((req.head.id.clone(), phase_a, phase_b));
        }
    }

    if differences.is_empty() {
        return format!(
            "No differences between `{}` and `{}`.",
            column_a_head, column_b_head
        );
    }

    let mut status = format!(
        "**Wiki differences between `{}` and `{}`:**

| {}{} | {}{} | {}{} |
| {} | {} | {} |\n",
        column_a_head,
        column_b_head,
        req_column_head,
        " ".repeat(max_req_column_width - req_column_head.len()),
        column_a_head,
        " ".repeat(max_branch_a_column_width - column_a_head.len()),
        column_b_head,
        " ".repeat(max_branch_b_column_width - column_b_head.len()),
        "-".repeat(max_req_column_width),
        "-".repeat(max_branch_a_column_width),
        "-".repeat(max_branch_b_column_width),
    );

    for (req_id, phase_a, phase_b) in differences {
        let req_spaces = " ".repeat(max_req_column_width - req_id.len());
        let column_a_spaces = " ".repeat(max_branch_a_column_width - phase_a.len());
        let column_b_spaces = " ".repeat(max_branch_b_column_width - phase_b.len());

        status.push_str(&format!(
            "| {}{} | {}{} | {}{} |",
            req_id, req_spaces, phase_a, column_a_spaces, phase_b, column_b_spaces
        ));
    }

    status
}

/// Returns the *phase* of the requirement in the given branch.
fn req_phase(ref_list: &[RefListEntry], repo: Option<&str>, branch: &str) -> String {
    let phase = match ref_list.iter().find(|entry| {
        entry.proj_line.branch_name.as_str() == branch
            && entry.proj_line.repo_name.as_ref().map(|s| s.as_str()) == repo
    }) {
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

        let status = status_cmp(&wiki, None, "main", None, "stable");

        assert_eq!(
            status,
            "**Wiki differences between `main` and `stable`:**

| REQ-ID       | main       | stable |
| ------------ | ---------- | ------ |
| ref_req.test | deprecated | active |",
            "Generated status differs for deprecated wiki entry."
        );
    }

    fn setup_mult_repo_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# ref_req: Some Title

**References:**

- in repo my_repo in branch main: 2
- in repo cmp_repo in branch main: 2 (0 direct)

## ref_req.test: Some Title

**References:**

- in repo my_repo in branch main: deprecated
- in repo cmp_repo in branch main: 2
        "#;

        Wiki::try_from((std::path::PathBuf::from(filename), content)).unwrap()
    }

    #[test]
    fn deprecated_req_in_mult_repo() {
        let wiki = setup_mult_repo_wiki();

        let status = status_cmp(&wiki, Some("my_repo"), "main", Some("cmp_repo"), "main");

        assert_eq!(
            status,
            "**Wiki differences between `my_repo/main` and `cmp_repo/main`:**

| REQ-ID       | my_repo/main | cmp_repo/main |
| ------------ | ------------ | ------------- |
| ref_req.test | deprecated   | active        |",
            "Generated status differs for deprecated wiki entry."
        );
    }
}
