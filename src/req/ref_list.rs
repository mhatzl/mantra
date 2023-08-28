//! Contains the *references* list
//!
//! [req:wiki.ref_list]

use std::cell::OnceCell;

use regex::Regex;

use super::ReqMatchingError;

/// Type representing the *references* list.
///
/// [req:wiki.ref_list]
pub type RefList = Vec<RefListEntry>;

/// Represents one entry inside the *references* list.
///
/// [req:wiki.ref_list]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RefListEntry {
    /// The name of the branch for this entry.
    ///
    /// [req:wiki.ref_list]
    pub branch_name: String,
    /// The link to the branch for this entry.
    ///
    /// [req:wiki.ref_list.branch_link]
    pub branch_link: Option<String>,

    /// The reference counter for this entry.
    ///
    /// [req:wiki.ref_list]
    pub ref_cnt: Option<RefCntKind>,

    pub is_manual: bool,

    pub is_deprecated: bool,
}

/// Reference counter kind for a requirement.
///
/// [req:req_id.sub_req_id], [req:wiki.ref_list]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RefCntKind {
    /// Counter for a high-level requirement.
    ///
    /// [req:req_id.sub_req_id]
    HighLvl { direct_cnt: usize, sub_cnt: usize },

    /// Counter for a low-level requirement.
    ///
    /// [req:req_id.sub_req_id]
    LowLvl { cnt: usize },
}

/// Holds the regex matcher for entries.
const REF_ENTRY_MATCHER: OnceCell<Regex> = std::cell::OnceCell::new();
/// Holds the regex matcher for optional branch links.
const BRANCH_LINK_MATCHER: OnceCell<Regex> = std::cell::OnceCell::new();

/// Tries to extract a *references* list entry from the given string.
///  
/// # Arguments
///
/// - `possible_entry` ... Content that may contain an entry of a *references* list
///
/// # Possible Errors
///
/// - [`ReqMatchingError::NoMatchFound`]
/// - [`ReqMatchingError::DeprecatedHasCnt]
/// - [`ReqMatchingError::ManualCntFailingPlus]
/// - [`ReqMatchingError::DirectCntWithoutGeneralCnt]
/// - [`ReqMatchingError::CntIsNoNumber]
/// - [`ReqMatchingError::DirectCntAboveGeneralCnt]
///
/// [req:wiki.ref_list]
pub fn get_ref_entry(possible_entry: &str) -> Result<RefListEntry, ReqMatchingError> {
    let entry_matcher = REF_ENTRY_MATCHER;
    let entry_regex = entry_matcher.get_or_init(|| {
        Regex::new(r"^[-\+\*]\sin\sbranch\s(?<branch>[^\s]+):\s(?:(?<depr>deprecated)|(?<manual>manual(?<plus>\s\+)?)\s*)?(?<cnt>\d+)?(?:\s\((?<direct_cnt>\d+)\sdirect\))?")
            .expect("Regex to match a *references* list entry could **not** be created.")
    });

    match entry_regex.captures(possible_entry) {
        Some(captures) => {
            let branch = captures
                .name("branch")
                .expect("`branch` capture group was not in *references* list entry match.")
                .as_str();
            let branch_matcher = BRANCH_LINK_MATCHER;
            let branch_regex = branch_matcher.get_or_init(|| {
                Regex::new(r"^\[(?<name>[^\]]+)\]\((?<link>[^\)]+)\)")
                    .expect("Regex to match an optional branch link could **not** be created.")
            });

            let (branch_name, branch_link) = match branch_regex.captures(branch) {
                Some(captures) => {
                    let name = captures
                        .name("name")
                        .expect("`name` capture group was not in branch match.")
                        .as_str()
                        .to_string();
                    let link = captures
                        .name("link")
                        .expect("`link` capture group was not in branch match.")
                        .as_str()
                        .to_string();
                    (name, Some(link))
                }
                None => (branch.to_string(), None),
            };

            let is_deprecated = captures.name("depr").is_some();
            let is_manual = captures.name("manual").is_some();
            let has_plus = captures.name("plus").is_some();
            let opt_cnt = captures.name("cnt");
            let opt_direct_cnt = captures.name("direct_cnt");

            if is_deprecated && (opt_cnt.is_some() || opt_direct_cnt.is_some()) {
                return Err(ReqMatchingError::DeprecatedHasCnt);
            } else if is_manual && opt_cnt.is_some() && !has_plus {
                return Err(ReqMatchingError::ManualCntFailingPlus);
            } else if opt_direct_cnt.is_some() && opt_cnt.is_none() {
                return Err(ReqMatchingError::DirectCntWithoutGeneralCnt);
            }

            let ref_cnt = match opt_cnt {
                Some(cnt_match) => {
                    let cnt = usize::from_str_radix(cnt_match.as_str(), 10)
                        .map_err(|_| ReqMatchingError::CntIsNoNumber)?;
                    match opt_direct_cnt {
                        Some(direct_cnt_match) => {
                            let direct_cnt = usize::from_str_radix(direct_cnt_match.as_str(), 10)
                                .map_err(|_| ReqMatchingError::CntIsNoNumber)?;

                            if direct_cnt > cnt {
                                return Err(ReqMatchingError::DirectCntAboveGeneralCnt);
                            }

                            Some(RefCntKind::HighLvl {
                                direct_cnt,
                                sub_cnt: cnt - direct_cnt,
                            })
                        }
                        None => Some(RefCntKind::LowLvl { cnt }),
                    }
                }
                None => None,
            };

            Ok(RefListEntry {
                branch_name,
                branch_link,
                ref_cnt,
                is_deprecated,
                is_manual,
            })
        }
        None => Err(ReqMatchingError::NoMatchFound),
    }
}

/// [req:wiki.ref_list]
#[cfg(test)]
mod test {
    use crate::req::ref_list::RefCntKind;

    use super::get_ref_entry;

    #[test]
    fn basic_ref_entry() {
        let ref_entry = get_ref_entry("- in branch main: 10").unwrap();

        assert_eq!(
            ref_entry.branch_name.as_str(),
            "main",
            "Branch name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.branch_link, None,
            "Branch link was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.ref_cnt.unwrap(),
            RefCntKind::LowLvl { cnt: 10 },
            "Reference counter was not retrieved correctly."
        );

        assert!(
            !ref_entry.is_deprecated,
            "Deprecated flag wrongfully detected."
        );
        assert!(!ref_entry.is_manual, "Manual flag wrongfully detected.");
    }

    #[test]
    fn high_lvl_ref_entry() {
        let ref_entry = get_ref_entry("- in branch stable: 10 (2 direct)").unwrap();

        assert_eq!(
            ref_entry.branch_name.as_str(),
            "stable",
            "Branch name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.branch_link, None,
            "Branch link was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.ref_cnt.unwrap(),
            RefCntKind::HighLvl {
                direct_cnt: 2,
                sub_cnt: 8
            },
            "Reference counter was not retrieved correctly."
        );

        assert!(
            !ref_entry.is_deprecated,
            "Deprecated flag wrongfully detected."
        );
        assert!(!ref_entry.is_manual, "Manual flag wrongfully detected.");
    }

    #[test]
    fn ref_entry_with_branch_link() {
        let ref_entry = get_ref_entry("- in branch [main](https://github.com/mhatzl/mantra/wiki/5-REQ-req_id#req_id-requirement-id): 10 (2 direct)").unwrap();

        assert_eq!(
            ref_entry.branch_name.as_str(),
            "main",
            "Branch name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.branch_link,
            Some(
                "https://github.com/mhatzl/mantra/wiki/5-REQ-req_id#req_id-requirement-id"
                    .to_string()
            ),
            "Branch link was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.ref_cnt.unwrap(),
            RefCntKind::HighLvl {
                direct_cnt: 2,
                sub_cnt: 8
            },
            "Reference counter was not retrieved correctly."
        );

        assert!(
            !ref_entry.is_deprecated,
            "Deprecated flag wrongfully detected."
        );
        assert!(!ref_entry.is_manual, "Manual flag wrongfully detected.");
    }
}
