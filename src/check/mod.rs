//! Contains functionality to validate the wiki, and references to requirements in the project.
//!
//! [req:check]

use std::{path::PathBuf, sync::Arc};

use clap::Args;
use logid::{
    evident::event::Event,
    log_id::{LogId, LogLevel},
    logging::{event_entry::LogEventEntry, msg::LogMsg},
};

use crate::{
    globals::GlobalParameter,
    references::{changes::ReferenceChanges, ReferencesError, ReferencesMap},
    wiki::{ref_list::RefCntKind, Wiki, WikiError},
};

/// Parameters for the `check` command.
///
/// [req:check]
#[derive(Args, Debug, Clone)]
pub struct CheckParameter {
    /// Global parameter needed for all commands.
    #[command(flatten)]
    pub global: GlobalParameter,

    /// The name of the branch project references should be validated against in the wiki.
    /// If not set, 'main' is used as default branch.
    ///
    /// [req:wiki.ref_list]
    #[arg(long, required = false, default_value = "main")]
    pub branch_name: String,
}

/// Counter for errors indicating references to missing requirement IDs in the wiki.
static REQ_ID_MISSING_CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Counter for errors indicating references to deprecated requirements.
static DEPR_REQ_REF_CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Counter for errors indicating duplicate requirement IDs in the wiki.
static DUPL_REQ_ID_CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Counter for errors indicating invalid entries in *references* lists.
static BAD_WIKI_ENTRY_CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// Counter for other errors during validation.
static OTHER_ERROR_CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Updates error counters according to received log event.
fn handle_error_cnts(event: Arc<Event<LogId, LogMsg, LogEventEntry>>) {
    let log_id = event.get_event_id();

    let missing_req_id = LogId::from(ReferencesError::ReqNotInWiki {
        req_id: String::new(),
        filepath: PathBuf::new(),
        line_nr: 0,
    });
    let depr_referenced = LogId::from(ReferencesError::DeprecatedReqReferenced {
        req_id: String::new(),
        branch_name: String::new(),
    });
    let dupl_req_id = LogId::from(WikiError::DuplicateReqId {
        req_id: String::new(),
        filepath: PathBuf::new(),
        line_nr: 0,
    });
    let bad_entry = LogId::from(WikiError::InvalidRefListEntry {
        filepath: PathBuf::new(),
        line_nr: 0,
        cause: String::new(),
    });

    match log_id {
        id if id == &missing_req_id => {
            REQ_ID_MISSING_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        id if id == &depr_referenced => {
            DEPR_REQ_REF_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        id if id == &dupl_req_id => {
            DUPL_REQ_ID_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        id if id == &bad_entry => {
            BAD_WIKI_ENTRY_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        id if id.get_log_level() == LogLevel::Error => {
            OTHER_ERROR_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        _ => {}
    }
}

/// Validates wiki and references in the project.
/// Prints a report to stdout.
///
/// [req:check]
pub fn check(param: &CheckParameter) -> Result<(), CheckError> {
    let error_monitor = logid::event_handler::builder::LogEventHandlerBuilder::new()
        .add_handler(handle_error_cnts)
        .all_log_events()
        .build()
        .map_err(|_| CheckError::ErrorMonitor)?;

    // This allows to capture multiple errors.
    crate::globals::disable_early_exit();

    let wiki = Wiki::try_from(&param.global.req_folder)?;
    let ref_map = ReferencesMap::try_from((&wiki, &param.global.proj_folder))?;

    let changes = ReferenceChanges::new(param.branch_name.clone().into(), None, &wiki, &ref_map)?;
    let cnt_changes = changes.cnt_changes();

    let mut new_active_reqs = Vec::new();
    let mut increased_refs = Vec::new();
    let mut decreased_refs = Vec::new();
    let mut became_untraced = Vec::new();

    for (req_id, new_cnt_kind) in cnt_changes {
        match wiki.req_ref_entry(&req_id, &param.branch_name) {
            Some(entry) => {
                let old_cnt_kind = entry.ref_cnt;
                match old_cnt_kind {
                    crate::wiki::ref_list::RefCntKind::HighLvl {
                        direct_cnt: old_direct_cnt,
                        sub_cnt: _,
                    } => match new_cnt_kind {
                        crate::wiki::ref_list::RefCntKind::HighLvl {
                            direct_cnt: new_direct_cnt,
                            sub_cnt: _,
                        } => match old_direct_cnt.cmp(&new_direct_cnt) {
                            std::cmp::Ordering::Less => {
                                increased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                            }
                            std::cmp::Ordering::Greater => {
                                decreased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                            }
                            _ => {}
                        },
                        crate::wiki::ref_list::RefCntKind::LowLvl { cnt: new_cnt } => {
                            match old_direct_cnt.cmp(&new_cnt) {
                                std::cmp::Ordering::Less => {
                                    increased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                                }
                                std::cmp::Ordering::Greater => {
                                    decreased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                                }
                                _ => {}
                            }
                        }
                        crate::wiki::ref_list::RefCntKind::Untraced => {
                            became_untraced.push((req_id, old_cnt_kind))
                        }
                    },
                    crate::wiki::ref_list::RefCntKind::LowLvl { cnt: old_cnt } => {
                        match new_cnt_kind {
                            crate::wiki::ref_list::RefCntKind::HighLvl {
                                direct_cnt: new_direct_cnt,
                                sub_cnt: _,
                            } => match old_cnt.cmp(&new_direct_cnt) {
                                std::cmp::Ordering::Less => {
                                    increased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                                }
                                std::cmp::Ordering::Greater => {
                                    decreased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                                }
                                _ => {}
                            },
                            crate::wiki::ref_list::RefCntKind::LowLvl { cnt: new_cnt } => {
                                match old_cnt.cmp(&new_cnt) {
                                    std::cmp::Ordering::Less => {
                                        increased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                                    }
                                    std::cmp::Ordering::Greater => {
                                        decreased_refs.push((req_id, new_cnt_kind, old_cnt_kind))
                                    }
                                    _ => {}
                                }
                            }
                            crate::wiki::ref_list::RefCntKind::Untraced => {
                                became_untraced.push((req_id, old_cnt_kind))
                            }
                        }
                    }
                    crate::wiki::ref_list::RefCntKind::Untraced => match new_cnt_kind {
                        crate::wiki::ref_list::RefCntKind::Untraced => continue,
                        _ => new_active_reqs.push((req_id, new_cnt_kind)),
                    },
                }
            }
            None => match new_cnt_kind {
                crate::wiki::ref_list::RefCntKind::Untraced => continue,
                _ => new_active_reqs.push((req_id, new_cnt_kind)),
            },
        }
    }

    error_monitor.shutdown();

    // Sort by ReqId, because HashMap traversal is indeterministic.
    new_active_reqs.sort_by(|a, b| a.0.cmp(&b.0));
    increased_refs.sort_by(|a, b| a.0.cmp(&b.0));
    decreased_refs.sort_by(|a, b| a.0.cmp(&b.0));
    became_untraced.sort_by(|a, b| a.0.cmp(&b.0));

    println!(
        "{}",
        check_report(
            &param.branch_name,
            &new_active_reqs,
            &increased_refs,
            &decreased_refs,
            &became_untraced
        )
    );

    REQ_ID_MISSING_CNT.store(0, std::sync::atomic::Ordering::Relaxed);
    DEPR_REQ_REF_CNT.store(0, std::sync::atomic::Ordering::Relaxed);
    DUPL_REQ_ID_CNT.store(0, std::sync::atomic::Ordering::Relaxed);
    BAD_WIKI_ENTRY_CNT.store(0, std::sync::atomic::Ordering::Relaxed);
    OTHER_ERROR_CNT.store(0, std::sync::atomic::Ordering::Relaxed);

    Ok(())
}

/// Creates the report for mantra check.
fn check_report(
    branch_name: &str,
    new_active_reqs: &[(String, RefCntKind)],
    increased_refs: &[(String, RefCntKind, RefCntKind)],
    decreased_refs: &[(String, RefCntKind, RefCntKind)],
    became_untraced: &[(String, RefCntKind)],
) -> String {
    let mut report = String::new();

    let req_missing_cnt = REQ_ID_MISSING_CNT.load(std::sync::atomic::Ordering::Relaxed);
    let depr_ref_cnt = DEPR_REQ_REF_CNT.load(std::sync::atomic::Ordering::Relaxed);
    let dupl_req_cnt = DUPL_REQ_ID_CNT.load(std::sync::atomic::Ordering::Relaxed);
    let bad_entry_cnt = BAD_WIKI_ENTRY_CNT.load(std::sync::atomic::Ordering::Relaxed);
    let other_err_cnt = OTHER_ERROR_CNT.load(std::sync::atomic::Ordering::Relaxed);

    let err_cnt = req_missing_cnt + depr_ref_cnt + dupl_req_cnt + bad_entry_cnt + other_err_cnt;

    if err_cnt > 0 {
        report.push_str(&format!(
            "--------------------------------------------------------------------
`mantra check` found {} error{} for branch: {}\n\n**Failed checks:**\n\n",
            err_cnt,
            if err_cnt > 1 { "s" } else { "" },
            branch_name,
        ));

        if req_missing_cnt > 0 {
            let s = if req_missing_cnt > 1 { "s" } else { "" };
            report.push_str(&format!(
                "- {} reference{} to non-existing requirement{}\n",
                req_missing_cnt, s, s,
            ));
        }
        if depr_ref_cnt > 0 {
            let s = if depr_ref_cnt > 1 { "s" } else { "" };
            report.push_str(&format!(
                "- {} reference{} to deprecated requirement{}\n",
                depr_ref_cnt, s, s
            ));
        }
        if dupl_req_cnt > 0 {
            report.push_str(&format!(
                "- {} duplicate requirement ID{} found\n",
                dupl_req_cnt,
                if dupl_req_cnt > 1 { "s" } else { "" }
            ));
        }
        if bad_entry_cnt > 0 {
            let s = if bad_entry_cnt > 1 { "s" } else { "" };
            report.push_str(&format!(
                "- {} invalid entrie{} in *references* list{}\n",
                bad_entry_cnt, s, s
            ));
        }
        if other_err_cnt > 0 {
            report.push_str(&format!(
                "- {} other error{} detected during validation\n",
                other_err_cnt,
                if other_err_cnt > 1 { "s" } else { "" }
            ));
        }

        report.push_str("\nSee log output for more details.\n\n**Passed checks:**\n\n");

        if req_missing_cnt == 0 {
            report.push_str("- All references refer to existing requirements\n");
        }
        if depr_ref_cnt == 0 {
            report.push_str("- No deprecated requirement referenced\n");
        }
        if dupl_req_cnt == 0 {
            report.push_str("- No duplicate requirement IDs in wiki\n");
        }
        if bad_entry_cnt == 0 {
            report.push_str("- All entries in *references* lists are valid\n");
        }
    } else {
        // Note: New lines are added automatically, and indentation is preserved for multi-line strings.
        report.push_str(&format!(
            "--------------------------------------------------------------------
`mantra check` ran successfully for branch: {branch_name}\n
**Checks:**\n
- All references refer to existing requirements
- No deprecated requirement referenced
- No duplicate requirement IDs in wiki
- All entries in *references* lists are valid\n"
        ));
    }

    if !new_active_reqs.is_empty() {
        report.push_str(&format!(
            "\n**{} new *active* requirement{}:**\n\n",
            new_active_reqs.len(),
            if new_active_reqs.len() > 1 { "s" } else { "" }
        ));

        for (req_id, new_cnt) in new_active_reqs {
            report.push_str(&format!("- req:{req_id} references: {new_cnt}\n"));
        }
    }

    if !became_untraced.is_empty() {
        report.push_str(&format!(
            "\n**Untraced {} requirement{}:**\n\n",
            became_untraced.len(),
            if became_untraced.len() > 1 { "s" } else { "" }
        ));

        for (req_id, old_cnt) in became_untraced {
            report.push_str(&format!(
                "- req:{req_id} references: {old_cnt} -> untraced\n"
            ));
        }
    }

    if !increased_refs.is_empty() {
        report.push_str(&format!(
            "\n**Increased direct references for {} requirement{}:**\n\n",
            increased_refs.len(),
            if increased_refs.len() > 1 { "s" } else { "" }
        ));

        for (req_id, new_cnt, old_cnt) in increased_refs {
            report.push_str(&format!(
                "- req:{req_id} references: {old_cnt} -> {new_cnt}\n"
            ));
        }
    }

    if !decreased_refs.is_empty() {
        report.push_str(&format!(
            "\n**Decreased direct references for {} requirement{}:**\n\n",
            decreased_refs.len(),
            if decreased_refs.len() > 1 { "s" } else { "" }
        ));

        for (req_id, new_cnt, old_cnt) in decreased_refs {
            report.push_str(&format!(
                "- req:{req_id} references: {old_cnt} -> {new_cnt}\n"
            ));
        }
    }

    report
}

/// Possible errors that may occure during validation.
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum CheckError {
    #[error("Failed to create error monitoring.")]
    ErrorMonitor,
    #[error("Failed to validate requirements in the wiki.")]
    WikiValidation,
    #[error("Failed to validate references in the project.")]
    ReferenceValidation,
}

impl From<WikiError> for CheckError {
    fn from(_value: WikiError) -> Self {
        CheckError::WikiValidation
    }
}

impl From<ReferencesError> for CheckError {
    fn from(_value: ReferencesError) -> Self {
        CheckError::ReferenceValidation
    }
}
