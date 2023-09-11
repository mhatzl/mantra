//! Contains functionality to create a release report.
//!
//! [req:release]

use clap::Args;

use crate::wiki::{ref_list::RefCntKind, req::ReqId, Wiki, WikiError};

/// Parameters for the `release` command.
///
/// [req:status]
#[derive(Args, Debug, Clone)]
pub struct ReleaseParameter {
    /// The folder that is searched recursively for defined requirements.
    ///
    /// [req:wiki]
    #[arg(index = 1, required = true)]
    pub req_folder: std::path::PathBuf,

    /// The branch name to create the overview for.
    /// Is used as first branch for comparisons.
    #[arg(long, alias = "branch-name", required = false, default_value = "main")]
    pub branch: String,

    /// The tag/version for this release.
    #[arg(long, alias = "tag", required = true)]
    pub release_tag: String,

    /// An optional URL prefix for links to the wiki.
    /// This options must be set if links to the wiki should be added in the report.
    ///
    /// [req:release]
    #[arg(long)]
    pub wiki_url_prefix: Option<String>,

    /// An optional filepath for the report.
    /// If this option is not set, the report is printed to stdout.
    ///
    /// **Note:** The report will be a markdown file, so the given extension is ignored.
    #[arg(long, aliases = ["release-file", "out-file", "checklist-file"])]
    pub report_file: Option<std::path::PathBuf>,

    /// Set this flag to turn this report into a checklist for requirements tagged with *manual*.
    ///
    /// [req:release.checklist]
    #[arg(long)]
    pub checklist: bool,
}

/// Creates a release report, and writes the report either to the given report-file,
/// or to stdout if no filepath is given.
///
/// [req:release]
pub fn release(param: &ReleaseParameter) -> Result<(), ReleaseError> {
    let wiki = Wiki::try_from(&param.req_folder)?;
    let high_reqs = wiki.high_lvl_reqs();

    let head = if param.checklist {
        "Requirements requiring *manual* verification for"
    } else {
        "*Active* requirements in"
    };

    let report = format!(
        "**{} release {}:**\n\n{}",
        head,
        param.release_tag,
        release_list(
            &wiki,
            &param.wiki_url_prefix,
            &param.branch,
            high_reqs.iter(),
            0,
            param.checklist,
        )
    );

    match &param.report_file {
        Some(filepath) => {
            let mut report_file = filepath.clone();
            report_file.set_extension(".md");

            std::fs::write(report_file, report)
                .map_err(|_| logid::pipe!(ReleaseError::WritingReport))?;
        }
        None => println!("{report}"),
    }

    Ok(())
}

/// Creates a release list for the given list of requirement IDs including all sub-requirements.
/// The indentation is increased by 2 per sub-requirement *depth*.
///
/// **Example:**
///
/// ```text
/// - high_lvl: Some title
///   - high_lvl.sub_req: Some title
/// ```
///
/// [req:release]
fn release_list<'a>(
    wiki: &'a Wiki,
    wiki_url_prefix: &Option<String>,
    branch: &str,
    req_ids: impl Iterator<Item = &'a ReqId>,
    indent: usize,
    checklist: bool,
) -> String {
    let mut list = String::new();

    req_ids.for_each(|req_id| {
        let mut sub_indent = indent;

        if let Some(req) = wiki.req(req_id) {
            if !checklist {
                sub_indent += 2; // only indent explicit requirements
            }

            if let Some(entry) = req
                .ref_list
                .iter()
                .find(|entry| entry.proj_line.branch_name.as_str() == branch)
            {
                if !entry.is_deprecated
                    && (entry.is_manual || (!checklist && entry.ref_cnt != RefCntKind::Untraced))
                {
                    let wiki_link = match &wiki_url_prefix {
                        Some(prefix) => {
                            let file_link = req
                                .filepath
                                .file_stem()
                                .map_or("bad-file".to_string(), |f| {
                                    f.to_string_lossy().to_string()
                                });
                            let file_link = file_link
                                .split_whitespace()
                                .collect::<Vec<&str>>()
                                .join("-")
                                .to_lowercase();

                            format!(
                                "([wiki-link]({}{}{}))",
                                prefix,
                                if prefix.ends_with('/') { "" } else { "/" },
                                file_link
                            )
                        }
                        None => String::new(),
                    };
                    list.push_str(&format!(
                        "{}- {}{}: {}{}\n",
                        " ".repeat(indent),
                        if checklist { "[ ] " } else { "" },
                        req_id,
                        req.head.title,
                        wiki_link,
                    ));
                }
            }
        }

        if let Some(subs) = wiki.sub_reqs(req_id) {
            let mut ordered_subs: Vec<&String> = subs.iter().collect();
            ordered_subs.sort();

            list.push_str(&release_list(
                wiki,
                wiki_url_prefix,
                branch,
                ordered_subs.iter().copied(),
                sub_indent,
                checklist,
            ));
        }
    });

    list
}

/// Possible errors that may occure while creating a release report.
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum ReleaseError {
    #[error("Failed to parse the wiki.")]
    WikiSetup,

    #[error("Could not write report to given file.")]
    WritingReport,
}

impl From<WikiError> for ReleaseError {
    fn from(_value: WikiError) -> Self {
        ReleaseError::WikiSetup
    }
}
