//! Contains the structure to represent requirements and the *references* list.
//!
//! [req:req_id], [req:wiki.ref_list]
use std::path::PathBuf;

use regex::Regex;
use thiserror::Error;

use super::ref_list::RefList;

/// Type for a requirement ID.
///
/// **Note:** Wrapper around [`String`] to be mor eexplicit.
///
/// [req:req_id], [req:req_id.sub_req_id]
pub type ReqId = String;

/// Stores information for a requirement found in the wiki.
///
/// [req:wiki.ref_list]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Req {
    /// The heading of the requirement in the wiki.
    ///
    /// [req:wiki]
    pub head: ReqHeading,

    /// The *references* list of this requirement.
    ///
    /// [req:wiki.ref_list]
    pub ref_list: RefList,

    /// The filepath this requirement is defined in.
    pub filepath: PathBuf,

    /// The line number the requirement heading starts in the file.
    ///
    /// **Note:** Starts at `0`.
    pub line_nr: usize,

    /// An optional link to this requirement inside the wiki.
    ///
    /// [req:wiki]
    pub wiki_link: Option<String>,
}

/// Represents a requirement heading in the wiki.
///
/// [req:wiki]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
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

/// Holds the regex matcher for requirement headings.
static REQ_HEADING_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

/// Tries to extract a requirement heading from the given content.
///  
/// # Arguments
///
/// - `possible_heading` ... Content that may contain a requirement heading
///
/// # Possible Errors
///
/// - [`ReqMatchingError::NoMatchFound`]
///
/// [req:req_id], [req:wiki]
pub fn get_req_heading(possible_heading: &str) -> Result<ReqHeading, ReqMatchingError> {
    let regex = REQ_HEADING_MATCHER.get_or_init(|| {
        // Note: This pattern may only be used to match the first line of a requirement heading.
        // Creating a pattern to match multiple lines until the *references* list is found would be too complicated.
        // => iterate through files line by line
        //
        // Regex to match full req-structure: (?:^|\n)(?<heading_lvl>#+) (?<id>[^:]+):(?<heading>(?:.|\n)+)\*\*References:\*\*\n\n*(?<entries>(?:[-\+\*][^\n]*\n?){1,})
        Regex::new(r"^(?<lvl>#+)\s(?<id>[^\s:]+):(?<title>.+)")
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
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReqMatchingError {
    /// No match was found in the given input.
    #[error("No match was found in the given input.")]
    NoMatchFound,

    /// Entry in the *references* list is marked as *deprecated*, but has reference counters.
    #[error(
        "Entry in the *references* list is marked as *deprecated*, but has reference counters."
    )]
    DeprecatedHasCnt,

    /// Optional count after `manual` flag is not separated by `+`.
    ///
    /// Correct would be `manual + <counter>`
    #[error("Optional count after `manual` flag is not separated by `+`.")]
    ManualCntFailingPlus,

    /// A direct references counter was set without the general counter before.
    ///
    /// Correct would be `<general counter> (<direct counter> direct)`.
    #[error("A direct references counter was set without the general counter before.")]
    DirectCntWithoutGeneralCnt,

    /// The matched counter could not be converted to a number.
    #[error("The matched counter could not be converted to a number.")]
    CntIsNoNumber,

    /// The matched direct counter is higher than the general counter.
    ///
    /// **Note:** The general counter must be the sum of the direct counter and all sub-requirement counter.
    #[error("The matched direct counter is higher than the general counter.")]
    DirectCntAboveGeneralCnt,
}

#[cfg(test)]
mod test {
    use super::get_req_heading;

    #[test]
    fn get_high_lvl_req() {
        let act_heading = get_req_heading("# req_id: Some Title").unwrap();

        assert_eq!(
            act_heading.id.as_str(),
            "req_id",
            "Requirement ID was not retrieved correctly."
        );
        assert_eq!(
            act_heading.lvl, 1,
            "Heading level was not retrieved correctly."
        );
        assert_eq!(
            act_heading.title.as_str(),
            "Some Title",
            "Heading title was not retrieved correctly."
        );
    }

    #[test]
    fn get_low_lvl_req() {
        let act_heading = get_req_heading("# req_id.sub_req: Some Title").unwrap();

        assert_eq!(
            act_heading.id.as_str(),
            "req_id.sub_req",
            "Requirement ID was not retrieved correctly."
        );
        assert_eq!(
            act_heading.lvl, 1,
            "Heading level was not retrieved correctly."
        );
        assert_eq!(
            act_heading.title.as_str(),
            "Some Title",
            "Heading title was not retrieved correctly."
        );
    }

    #[test]
    fn get_req_in_sub_heading() {
        let act_heading = get_req_heading("## req_id.sub_req: Some Title").unwrap();

        assert_eq!(
            act_heading.id.as_str(),
            "req_id.sub_req",
            "Requirement ID was not retrieved correctly."
        );
        assert_eq!(
            act_heading.lvl, 2,
            "Heading level was not retrieved correctly."
        );
        assert_eq!(
            act_heading.title.as_str(),
            "Some Title",
            "Heading title was not retrieved correctly."
        );
    }

    #[test]
    fn ignore_req_id_with_whitespace() {
        let act_heading = get_req_heading("# req id: Some Title");

        assert_eq!(
            act_heading.unwrap_err(),
            super::ReqMatchingError::NoMatchFound,
            "Requirement ID with whitespace was extracted as valid ID."
        );
    }
}
