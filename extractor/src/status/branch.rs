//! Contains the status overview for a specific branch.
//!
//! [req:status.branch]

use crate::wiki::Wiki;

use super::StatusParameter;

/// Creates an overview for a specific branch in the wiki.
///
/// [req:status.branch]
pub fn status_branch(wiki: &Wiki, param: &StatusParameter) -> String {
    let mut ready_cnt = 0;
    let mut ready_details = Vec::new();

    let mut active_cnt = 0;
    let mut active_details = Vec::new();

    let mut deprecated_cnt = 0;
    let mut deprecated_details = Vec::new();

    let mut manual_cnt = 0;
    let mut manual_details = Vec::new();

    for req in wiki.reqs() {
        match req.ref_list.iter().find(|entry| {
            entry.proj_line.branch_name.as_ref() == &param.branch
                && entry.proj_line.repo_name.as_deref() == param.repo_name.as_ref()
        }) {
            Some(entry) => {
                if entry.is_deprecated {
                    deprecated_cnt += 1;

                    if param.detail_deprecated {
                        deprecated_details.push((req.head.id.clone(), req.head.title.clone()));
                    }
                } else {
                    active_cnt += 1;

                    if param.detail_active {
                        active_details.push((req.head.id.clone(), req.head.title.clone()));
                    }
                }

                if entry.is_manual {
                    manual_cnt += 1;

                    if param.detail_manual {
                        manual_details.push((req.head.id.clone(), req.head.title.clone()));
                    }
                }
            }
            None => {
                ready_cnt += 1;

                if param.detail_ready {
                    ready_details.push((req.head.id.clone(), req.head.title.clone()));
                }
            }
        }
    }

    let mut overview = format!(
        "**Wiki status for {}branch `{}`:**\n\n",
        if let Some(repo) = &param.repo_name {
            format!("repository '{}' with ", repo)
        } else {
            String::new()
        },
        param.branch
    );

    overview.push_str(&format!(
        "- {} requirement{} *ready* to be implemented\n",
        ready_cnt,
        if ready_cnt == 1 { " is" } else { "s are" },
    ));

    overview.push_str(&format!(
        "- {} requirement{} *active*\n",
        active_cnt,
        if active_cnt == 1 { " is" } else { "s are" },
    ));

    overview.push_str(&format!(
        "- {} requirement{} *deprecated*\n",
        deprecated_cnt,
        if deprecated_cnt == 1 { " is" } else { "s are" },
    ));

    overview.push_str(&format!(
        "- {} requirement{} *manual* verification\n",
        manual_cnt,
        if manual_cnt == 1 { " needs" } else { "s need" },
    ));

    if !ready_details.is_empty() {
        overview.push_str("\n***Ready* requirements:**\n\n");

        ready_details.sort_by(|a, b| a.0.cmp(&b.0));

        overview.push_str(&create_req_details(ready_details));
    }

    if !active_details.is_empty() {
        overview.push_str("\n***Active* requirements:**\n\n");

        active_details.sort_by(|a, b| a.0.cmp(&b.0));

        overview.push_str(&create_req_details(active_details));
    }

    if !deprecated_details.is_empty() {
        overview.push_str("\n***Deprecated* requirements:**\n\n");

        deprecated_details.sort_by(|a, b| a.0.cmp(&b.0));

        overview.push_str(&create_req_details(deprecated_details));
    }

    if !manual_details.is_empty() {
        overview.push_str("\n***Manual* flagged requirements:**\n\n");

        manual_details.sort_by(|a, b| a.0.cmp(&b.0));

        overview.push_str(&create_req_details(manual_details));
    }

    overview
}

/// Creates list of requirement details.
///
/// Format: `- <req-id>: <req-title>`
fn create_req_details(details: Vec<(String, String)>) -> String {
    let mut req_list = String::new();

    for (req_id, title) in details {
        req_list.push_str(&format!("- {}: {}\n", req_id, title,));
    }

    req_list
}
