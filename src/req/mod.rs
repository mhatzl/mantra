//! Contains the structure to represent requirements and the *references* list.
use std::{cell::OnceCell, sync::atomic::AtomicUsize};

use regex::Regex;

use self::ref_list::{RefCntKind, RefList};

mod ref_list;

/// Type for a requirement ID.
///
/// **Note:** Wrapper around [`String`] to be mor eexplicit.
///
/// [req:req_id], [req:req_id.sub_req_id]
pub type ReqId = String;

/// Stores information for a requirement found in the wiki.
///
/// [req:wiki.ref_list]
#[derive(Debug)]
pub struct Req {
    /// The heading of the requirement in the wiki.
    ///
    /// [req:wiki]
    pub head: ReqHeading,

    /// The *references* list of this requirement.
    ///
    /// [req:wiki.ref_list]
    pub ref_list: RefList,

    /// List of sub-requirements that are one level *deeper* that this requirement might have.
    ///
    /// **Note:** This list is empty if this requirement has no sub-requirements.
    ///
    /// [req:req_id.sub_req_id]
    pub sub_reqs: Vec<ReqId>,

    /// The filename this requirement is defined in.
    pub filename: String,

    /// The line number the requirement heading starts in the file.
    pub line_nr: usize,

    /// The current reference counter for direct references to this requirement.
    /// This counter is used to update/validate the existing reference count.
    ///
    /// **Note:** Atomic to be updated concurrently.
    ///
    /// [req:ref_req]
    pub curr_direct_refs: AtomicUsize,

    /// The new reference counter for this requirement, or `None` if references are not being updated.
    ///
    /// [req:ref_req]
    pub new_cnt_kind: Option<RefCntKind>,

    /// An optional link to this requirement inside the wiki.
    ///
    /// [req:wiki]
    pub wiki_link: Option<String>,
}

#[derive(Debug)]
pub struct ReqHeading {
    /// Requirement ID of this requirement.
    ///
    /// [req:req_id]
    pub id: ReqId,

    /// The heading level of the requirement.
    ///
    /// Ranges from 1 to 6
    pub lvl: usize,

    /// The title of the requirement.
    pub title: String,
}

const REQ_HEADING_MATCHER: OnceCell<Regex> = std::cell::OnceCell::new();

pub fn get_req_heading(possible_heading: &str) -> Result<ReqHeading, ReqMatchingError> {
    let binding = REQ_HEADING_MATCHER;
    let regex = binding.get_or_init(|| {
        // Note: This pattern may only be used to match the first line of a requirement heading.
        // Creating a pattern to match multiple lines until the *references* list is found would be too complicated.
        // => iterate through files line by line
        //
        // Regex to match full req-structure: (?:^|\n)(?<heading_lvl>#+) (?<id>[^:]+):(?<heading>(?:.|\n)+)\*\*References:\*\*\n\n*(?<entries>(?:[-\+\*][^\n]*\n?){1,})
        Regex::new(r"(?:^|\n)(?<lvl>#+)\s(?<id>[^:]+):(?<title>.+)")
            .expect("Regex to match the requirement heading could **not** be created.")
    });

    match regex.captures(possible_heading) {
        Some(captures) => Ok(ReqHeading {
            id: captures
                .name("id")
                .expect("`id` capture group was not in heading match.")
                .as_str()
                .to_string(),
            lvl: captures
                .name("lvl")
                .expect("`lvl` capture group was not in heading match.")
                .len(),
            title: captures
                .name("title")
                .expect("`title` capture group was not in heading match.")
                .as_str()
                .trim()
                .to_string(),
        }),
        None => Err(ReqMatchingError::NoMatchFound),
    }
}

/// Errors that may occure when trying to match regex patterns against given inputs.
#[derive(Debug)]
pub enum ReqMatchingError {
    /// No match was found in the given input.
    NoMatchFound,
}

#[cfg(test)]
mod test {
    use super::get_req_heading;

    #[test]
    pub fn get_high_lvl_req() {
        let act_heading = get_req_heading("# req_id: Some Title").unwrap();

        assert_eq!(
            act_heading.id.as_str(),
            "req_id",
            "Requirement ID was not retrieved correctly."
        );
        assert_eq!(
            act_heading.lvl, 1,
            "Requirement ID was not retrieved correctly."
        );
        assert_eq!(
            act_heading.title.as_str(),
            "Some Title",
            "Requirement ID was not retrieved correctly."
        );
    }
}
