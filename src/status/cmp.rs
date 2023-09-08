//! Contains the status overview for the comparison between two branches.
//!
//! [req:status.cmp]

use crate::wiki::Wiki;

use super::StatusError;

/// Creates an overview for the comparison between two branches in the wiki.
///
/// [req:status.cmp]
pub fn status_cmp(wiki: &Wiki, branch_a: &str, branch_b: &str) -> Result<String, StatusError> {
    todo!()
}
