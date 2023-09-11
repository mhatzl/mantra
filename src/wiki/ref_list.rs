//! Contains the *references* list
//!
//! [req:wiki.ref_list]

use std::sync::Arc;

use regex::Regex;

use super::ReqMatchingError;

/// Type representing the *references* list.
///
/// [req:wiki.ref_list]
pub type RefList = Vec<RefListEntry>;

/// Represents the project line in an entry inside the *references* list.
/// A project line contains the branch, and an optional repository name.
///
/// [req:wiki.ref_list.repo]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ProjectLine {
    /// The name of the branch for an entry in the *references* list.
    ///
    /// [req:wiki.ref_list]
    pub branch_name: Arc<String>,

    /// The link to the branch for an entry in the *references* list.
    ///
    /// [req:wiki.ref_list.branch_link] (see DR-20230906_2 for more info)
    pub branch_link: Option<Arc<String>>,

    /// The optional repository name for an entry in the *references* list.
    ///
    /// [req:wiki.ref_list.repo]
    pub repo_name: Option<Arc<String>>,
}

impl ProjectLine {
    pub fn new(
        repo_name: Option<String>,
        branch_name: String,
        branch_link: Option<String>,
    ) -> Self {
        ProjectLine {
            branch_name: branch_name.into(),
            branch_link: branch_link.map(|s| s.into()),
            repo_name: repo_name.map(|s| s.into()),
        }
    }
}

/// Represents one entry inside the *references* list.
///
/// [req:wiki.ref_list]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RefListEntry {
    /// The project line this entry is associated with.
    ///
    /// [req:wiki.ref_list.repo]
    pub proj_line: ProjectLine,

    /// The reference counter for this entry.
    ///
    /// [req:wiki.ref_list]
    pub ref_cnt: RefCntKind,

    /// Marks this entry to require manual verification.
    ///
    /// [req:wiki.ref_list.manual]
    pub is_manual: bool,

    /// Marks this requirement to be deprecated in this branch.
    ///
    /// [req:wiki.ref_list.deprecated]
    pub is_deprecated: bool,
}

impl std::fmt::Display for RefListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // [req:wiki.ref_list.manual], [req:wiki.ref_list.deprecated]
        let cnt = if self.is_deprecated {
            "deprecated".to_string()
        } else if self.is_manual {
            let mut s = "manual".to_string();
            if self.ref_cnt != RefCntKind::Untraced {
                s.push_str(&format!(" + {}", self.ref_cnt));
            }
            s
        } else {
            self.ref_cnt.to_string()
        };

        let repo = match &self.proj_line.repo_name {
            Some(repo_name) => format!("in repo {} ", repo_name),
            None => String::new(),
        };

        let branch = match &self.proj_line.branch_link {
            Some(link) => format!("in branch [{}]({})", self.proj_line.branch_name, link),
            None => format!("in branch {}", self.proj_line.branch_name),
        };

        write!(f, "- {repo}{branch}: {cnt}")
    }
}

/// Reference counter kind for a requirement.
///
/// [req:req_id.sub_req_id], [req:wiki.ref_list]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RefCntKind {
    /// Counter for a high-level requirement.
    ///
    /// [req:req_id.sub_req_id]
    HighLvl { direct_cnt: usize, sub_cnt: usize },

    /// Counter for a low-level requirement.
    ///
    /// [req:req_id.sub_req_id]
    LowLvl { cnt: usize },

    /// Special variant that marks a requirement as having no references.
    Untraced,
}

impl std::fmt::Display for RefCntKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefCntKind::HighLvl {
                direct_cnt,
                sub_cnt,
            } => write!(f, "{} ({} direct)", direct_cnt + sub_cnt, direct_cnt),
            RefCntKind::LowLvl { cnt } => write!(f, "{}", cnt),
            RefCntKind::Untraced => write!(f, "0"),
        }
    }
}

/// Holds the regex matcher for entries.
static REF_ENTRY_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
/// Holds the regex matcher for optional branch links.
static BRANCH_LINK_MATCHER: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();

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
    let entry_regex = REF_ENTRY_MATCHER.get_or_init(|| {
        Regex::new(r"^[-\+\*]\s(?:in\srepo\s(?<repo>[^\s]+)\s)?in\sbranch\s(?<branch>[^\s]+):\s(?:(?<depr>deprecated)|(?<manual>manual))?(?<plus>\s\+\s)?\s*(?<cnt>\d+)?(?:\s\((?<direct_cnt>\d+)\sdirect\))?")
            .expect("Regex to match a *references* list entry could **not** be created.")
    });

    match entry_regex.captures(possible_entry) {
        Some(captures) => {
            let repo_name = captures
                .name("repo")
                .map(|r| Arc::new(r.as_str().to_string()));

            let branch = captures
                .name("branch")
                .expect("`branch` capture group was not in *references* list entry match.")
                .as_str();
            let branch_regex = BRANCH_LINK_MATCHER.get_or_init(|| {
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
                    (Arc::new(name), Some(Arc::new(link)))
                }
                None => (Arc::new(branch.to_string()), None),
            };

            // [req:wiki.ref_list.deprecated]
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
                    let cnt = cnt_match
                        .as_str()
                        .parse::<usize>()
                        .map_err(|_| ReqMatchingError::CntIsNoNumber)?;
                    match opt_direct_cnt {
                        Some(direct_cnt_match) => {
                            let direct_cnt = direct_cnt_match
                                .as_str()
                                .parse::<usize>()
                                .map_err(|_| ReqMatchingError::CntIsNoNumber)?;

                            if direct_cnt > cnt {
                                return Err(ReqMatchingError::DirectCntAboveGeneralCnt);
                            }

                            RefCntKind::HighLvl {
                                direct_cnt,
                                sub_cnt: cnt - direct_cnt,
                            }
                        }
                        None => RefCntKind::LowLvl { cnt },
                    }
                }
                None => RefCntKind::Untraced,
            };

            Ok(RefListEntry {
                proj_line: ProjectLine {
                    branch_name,
                    branch_link,
                    repo_name,
                },
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
    use super::*;

    #[test]
    fn basic_ref_entry() {
        let ref_entry = get_ref_entry("- in branch main: 10").unwrap();

        assert_eq!(
            ref_entry.proj_line.branch_name.as_str(),
            "main",
            "Branch name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.proj_line.branch_link, None,
            "Branch link was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.ref_cnt,
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
            ref_entry.proj_line.branch_name.as_str(),
            "stable",
            "Branch name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.proj_line.branch_link, None,
            "Branch link was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.ref_cnt,
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
            ref_entry.proj_line.branch_name.as_str(),
            "main",
            "Branch name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.proj_line.branch_link.unwrap().as_ref(),
            &"https://github.com/mhatzl/mantra/wiki/5-REQ-req_id#req_id-requirement-id".to_string(),
            "Branch link was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.ref_cnt,
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
    fn deprecated_ref_entry() {
        let ref_entry = get_ref_entry("- in branch main: deprecated").unwrap();

        assert_eq!(
            ref_entry.ref_cnt,
            RefCntKind::Untraced,
            "Reference counter wrongfully set for *deprecated* requirement."
        );

        assert!(ref_entry.is_deprecated, "Deprecated flag not detected.");
        assert!(!ref_entry.is_manual, "Manual flag wrongfully detected.");
    }

    #[test]
    fn deprecated_plus_cnt_ref_entry() {
        let ref_entry_result = get_ref_entry("- in branch main: deprecated + 1");

        assert!(
            ref_entry_result.is_err(),
            "Deprecated flag with cnt did not result in error."
        );
    }

    #[test]
    fn deprecated_cnt_no_plus_ref_entry() {
        let ref_entry_result = get_ref_entry("- in branch main: deprecated 1");

        assert!(
            ref_entry_result.is_err(),
            "Deprecated flag with cnt did not result in error."
        );
    }

    #[test]
    fn deprecated_ref_entry_to_string() {
        let ref_entry = get_ref_entry("- in branch main: deprecated").unwrap();

        assert_eq!(
            ref_entry.to_string(),
            "- in branch main: deprecated",
            "*deprecated* requirement not printed correctly."
        );
    }

    #[test]
    fn manual_ref_entry_to_string() {
        let ref_entry = get_ref_entry("- in branch main: manual").unwrap();

        assert_eq!(
            ref_entry.to_string(),
            "- in branch main: manual",
            "*manual* requirement not printed correctly."
        );
    }

    #[test]
    fn manual_ref_entry_with_refs_to_string() {
        let ref_entry = get_ref_entry("- in branch main: manual + 2 (1 direct)").unwrap();

        assert_eq!(
            ref_entry.to_string(),
            "- in branch main: manual + 2 (1 direct)",
            "*manual* requirement with references not printed correctly."
        );
    }

    #[test]
    fn basic_repo_ref_entry() {
        let ref_entry = get_ref_entry("- in repo mantra in branch main: 10").unwrap();

        assert_eq!(
            ref_entry.proj_line.repo_name.unwrap().as_str(),
            "mantra",
            "Repo name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.proj_line.branch_name.as_str(),
            "main",
            "Branch name was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.proj_line.branch_link, None,
            "Branch link was not retrieved correctly."
        );
        assert_eq!(
            ref_entry.ref_cnt,
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
    fn repo_ref_entry_to_string() {
        let ref_entry = get_ref_entry("- in repo mantra in branch main: 2").unwrap();

        assert_eq!(
            ref_entry.to_string(),
            "- in repo mantra in branch main: 2",
            "Repository name in entry not printed correctly."
        );
    }
}
