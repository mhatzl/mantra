//! Contains the [`Wiki`] struct, representing found requirements of a wiki.
//!
//! [req:wiki]

use std::{
    collections::{hash_map::Keys, HashMap, HashSet},
    path::PathBuf,
};

use walkdir::WalkDir;

use crate::req::{
    get_req_heading,
    ref_list::{get_ref_entry, RefListEntry},
    Req, ReqId, ReqMatchingError,
};

/// Struct representing a wiki that stores requirements.
///
/// [req:wiki]
#[derive(Debug)]
pub struct Wiki {
    /// Map for all found requirements in the wiki.
    req_map: HashMap<ReqId, Req>,

    /// Map to store sub-requirement IDs of high-level requirements.
    /// Only sub-requirements one level *deeper* are stored.
    ///
    /// **Note:** This may include IDs for non-existing requirements if a sub-requirement is created at a *deeper* level, without creating the *implicit* IDs between.
    /// e.g. Creating `high_lvl.test.test_sub_req`, but not `high_lvl.test`.
    ///
    /// [req:req_id.sub_req_id]
    sub_map: HashMap<ReqId, HashSet<ReqId>>,

    /// List of high-level requirements found in this wiki.
    ///
    /// [req:wiki]
    high_lvl_reqs: Vec<ReqId>,
}

impl TryFrom<&PathBuf> for Wiki {
    type Error = WikiError;

    fn try_from(req_path: &PathBuf) -> Result<Self, Self::Error> {
        let mut wiki = Wiki::new();

        if req_path.is_dir() {
            let mut walk = WalkDir::new(req_path).into_iter().filter_entry(|entry| {
                entry.file_type().is_dir()
                    || entry
                        .path()
                        .extension()
                        .map_or(false, |ext| ext == "md" || ext == "markdown")
            });
            while let Some(Ok(dir_entry)) = walk.next() {
                if dir_entry.file_type().is_file() {
                    let filepath = dir_entry.into_path();
                    let content = std::fs::read_to_string(filepath.clone()).map_err(|_| {
                        logid::pipe!(WikiError::CouldNotAccessFile(filepath.clone()))
                    })?;
                    wiki.add(filepath, &content)?;
                }
            }
        } else {
            let filepath = req_path.to_path_buf();
            let content = std::fs::read_to_string(req_path)
                .map_err(|_| logid::pipe!(WikiError::CouldNotAccessFile(filepath.clone())))?;
            wiki.add(filepath, &content)?;
        }

        Ok(wiki)
    }
}

impl TryFrom<(PathBuf, &str)> for Wiki {
    type Error = WikiError;

    /// Tries to create a wiki given (filepath, content).
    fn try_from(value: (PathBuf, &str)) -> Result<Self, Self::Error> {
        let filepath = value.0;
        let content = value.1;

        let mut wiki = Wiki::new();
        wiki.add(filepath, content)?;
        Ok(wiki)
    }
}

impl Wiki {
    /// Returns the number of requirements that were found in the wiki.
    pub fn req_cnt(&self) -> usize {
        self.req_map.len()
    }

    /// Returns an iterator over the IDs of found requirements in the wiki.
    pub fn requirements(&self) -> Keys<ReqId, Req> {
        self.req_map.keys()
    }

    /// Returns the sub-requirements of a given requirement, or `None` if it is a *leaf* requirement.
    pub fn sub_reqs(&self, req_id: &ReqId) -> Option<&HashSet<ReqId>> {
        self.sub_map.get(req_id)
    }

    /// Gets the requirement associated with the given requirement ID, or `None` if the ID does not exist in this wiki.
    pub fn req(&self, req_id: &ReqId) -> Option<&Req> {
        self.req_map.get(req_id)
    }

    /// Checks if the given requirement ID is only implicitly created in the Wiki,
    /// but has no wiki-section on its own.
    ///
    /// Returns `true` if the given requirement is implicit,
    /// or `false` if it is explicit, or does not exist in the wiki.
    pub fn is_implicit(&self, req_id: &ReqId) -> bool {
        self.req_map.get(req_id).is_none() && self.sub_map.get(req_id).is_some()
    }

    pub fn req_ref_entry(&self, req_id: &ReqId, branch_name: &str) -> Option<&RefListEntry> {
        match self.req(req_id) {
            Some(wiki_req) => wiki_req
                .ref_list
                .iter()
                .find(|entry| entry.branch_name.as_ref() == branch_name),
            None => None,
        }
    }

    /// Creates a new wiki.
    fn new() -> Self {
        Wiki {
            req_map: HashMap::new(),
            sub_map: HashMap::new(),
            high_lvl_reqs: Vec::new(),
        }
    }

    /// Iterate through the given content, and add found requirements.
    fn add(&mut self, filepath: PathBuf, content: &str) -> Result<usize, WikiError> {
        let lines = content.lines();

        let mut added_reqs = 0;
        let mut has_references_list = false;
        let mut curr_req: Option<Req> = None;

        let mut in_verbatim_context = false;

        for (line_nr, line) in lines.enumerate() {
            if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
                in_verbatim_context = !in_verbatim_context;
            }

            if !in_verbatim_context {
                if line.starts_with("**References:**") {
                    has_references_list = true;
                } else if let Ok(req_heading) = get_req_heading(line) {
                    // Add previous found requirement to wiki, before starting this one.
                    if let Some(req) = curr_req.as_mut() {
                        let req_id = req.head.id.clone();
                        added_reqs += 1;
                        let prev_req = self.req_map.insert(req_id.clone(), std::mem::take(req));

                        if let Some(prev) = prev_req {
                            return logid::err!(WikiError::DuplicateReqId {
                                req_id: req_id.clone(),
                                filepath: prev.filepath.clone(),
                                line_nr: prev.line_nr,
                            });
                        }
                    }

                    has_references_list = false;

                    let req_id = req_heading.id.clone();
                    let split_id = req_id.clone();
                    let mut req_id_parts = split_id.split('.');
                    let id_part_len = req_id_parts.clone().count();
                    if id_part_len > 1 {
                        let mut parent_req_id = req_id_parts
                            .next()
                            .expect("More than one ID part, but first `next()` failed.")
                            .to_string();

                        for part in req_id_parts {
                            let curr_id = format!("{}.{}", parent_req_id, part);
                            let entry = self.sub_map.entry(parent_req_id).or_insert(HashSet::new());

                            // Note: HashSet is used to prevent duplicate entries for the same ID.
                            entry.insert(curr_id.clone());

                            parent_req_id = curr_id;
                        }
                    } else {
                        self.high_lvl_reqs.push(req_id);
                    }

                    curr_req = Some(Req {
                        head: req_heading,
                        ref_list: Vec::new(),
                        filepath: filepath.clone(),
                        line_nr,
                        wiki_link: None,
                    });
                } else if has_references_list {
                    if let Some(req) = curr_req.as_mut() {
                        match get_ref_entry(line) {
                            Ok(entry) => {
                                req.ref_list.push(entry);
                            }
                            Err(ReqMatchingError::NoMatchFound) => continue,
                            Err(err) => {
                                return logid::err!(WikiError::InvalidRefListEntry {
                                    filepath: filepath.clone(),
                                    line_nr,
                                    cause: err.to_string(),
                                })
                            }
                        }

                        // Reset flag after the *references* list entries to accept new requirement headings.
                        if !req.ref_list.is_empty() && line.trim().is_empty() {
                            has_references_list = false;
                        }
                    }
                }
            }
        }

        // Last found requirement might not have been inserted in loop, because we were waiting for the *references* list.
        if let Some(req) = curr_req {
            added_reqs += 1;
            let req_id = req.head.id.clone();
            let prev_req = self.req_map.insert(req_id.clone(), req);

            if let Some(prev) = prev_req {
                return logid::err!(WikiError::DuplicateReqId {
                    req_id: req_id.clone(),
                    filepath: prev.filepath.clone(),
                    line_nr: prev.line_nr,
                });
            }
        }

        Ok(added_reqs)
    }

    /// Flattens this wiki, starting at the first *leaf* requirement of the first high-level requirement.
    /// Resulting in a depth-first flat representation of the requirement hierarchy of the wiki.
    pub(crate) fn flatten(&self) -> Vec<WikiReq> {
        // Note: Due to possible implicit requirements, this capacity may not be enough, but it is the closest we can get without increasing complexity.
        let mut flat_wiki = Vec::with_capacity(self.req_cnt());

        for req in &self.high_lvl_reqs {
            self.flatten_req(req, &mut flat_wiki);
        }

        flat_wiki
    }

    /// Recursively flattens requirements of the wiki.
    ///
    /// **Note:** All sub-requirements are added **before** the requirement identified by the given `req_id`.
    fn flatten_req(&self, req_id: &ReqId, flat_wiki: &mut Vec<WikiReq>) {
        if let Some(sub_reqs) = self.sub_map.get(req_id) {
            for sub_req_id in sub_reqs {
                self.flatten_req(sub_req_id, flat_wiki);
            }
        }

        let wiki_req = match self.req(req_id) {
            Some(req) => WikiReq::Explicit { req: req.clone() },
            None => WikiReq::Implicit {
                req_id: req_id.clone(),
            },
        };

        flat_wiki.push(wiki_req);
    }
}

/// Represents different requirement representations in the wiki.
pub enum WikiReq {
    /// Explicit requirements have a heading in the wiki.
    Explicit { req: Req },
    /// Implicit requireemnts have no distinct heading in the wiki.
    /// They are implicitly created, when a sub-requirement would refer to a parent requirement that does not exist in the wiki.
    ///
    /// **Example:**
    ///
    /// ```text
    /// # req_id: Some high-level requirement
    ///
    /// Explicit requirement.
    ///
    /// ## req_id.test.sub_req: Some low-level requirement
    ///
    /// This automatically creates `req_id.test` as an implicit requirement.
    /// ```
    Implicit { req_id: ReqId },
}

impl WikiReq {
    pub fn req_id(&self) -> &ReqId {
        match self {
            WikiReq::Explicit { req } => &req.head.id,
            WikiReq::Implicit { req_id } => req_id,
        }
    }
}

/// Enum representing possible errors that may occur, when using functions for [`Wiki`].
#[derive(Debug, thiserror::Error, logid::ErrLogId)]
pub enum WikiError {
    #[error("Could not access file '{}' in wiki.", .0.to_string_lossy())]
    CouldNotAccessFile(PathBuf),

    // Note: +1 for line number, because internally, lines start at index 0.
    #[error("Duplicate requirement ID '{}' found in file '{}' at line '{}'.", .req_id, .filepath.to_string_lossy(), .line_nr + 1)]
    DuplicateReqId {
        /// The requirement ID
        req_id: String,
        /// Name of the file the ID is already specified.
        filepath: PathBuf,
        /// Line number in the file the ID is already specified.
        line_nr: usize,
    },

    // Note: +1 for line number, because internally, lines start at index 0.
    #[error("Found an invalid entry in the references list in file '{}' at line '{}'. Cause: {}", .filepath.to_string_lossy(), .line_nr + 1, .cause)]
    InvalidRefListEntry {
        /// Name of the file the invalid entry was found in.
        filepath: PathBuf,
        /// Line number in the file the invalid entry was found at.
        line_nr: usize,
        /// The reason why this entry is invalid.
        cause: String,
    },
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::Wiki;

    #[test]
    fn high_lvl_req_with_1_ref_entry() {
        let filename = "test_file";
        // Note: String moved to the most left to get correct new line behavior.
        let content = r#"
# req_id: Some Title

**References:**

- in branch main: 2
        "#;

        let mut wiki = Wiki::new();
        wiki.add(PathBuf::from(filename), content).unwrap();

        assert!(
            wiki.req_map.contains_key("req_id"),
            "Requirement was not added to the wiki."
        );
        assert!(
            wiki.high_lvl_reqs.contains(&"req_id".to_string()),
            "Requirement not added to list of high-level requirements"
        );

        let req = wiki.req_map.get("req_id").unwrap();
        assert_eq!(
            req.ref_list.len(),
            1,
            "Wrong number of *references* list entries added."
        );
    }

    #[test]
    fn req_missing_ref_list_start() {
        let filename = "test_file";
        let content = r#"
# req_id: Some Title

- in branch main: 2
        "#;

        let mut wiki = Wiki::new();
        wiki.add(PathBuf::from(filename), content).unwrap();

        assert!(
            wiki.req_map.contains_key("req_id"),
            "Requirement was not added to the wiki."
        );
        assert!(
            wiki.high_lvl_reqs.contains(&"req_id".to_string()),
            "Requirement not added to list of high-level requirements"
        );

        let req = wiki.req_map.get("req_id").unwrap();
        assert_eq!(
            req.ref_list.len(),
            0,
            "References added with missing **References:**."
        );
    }

    #[test]
    fn low_lvl_req_with_1_ref_entry() {
        let filename = "test_file";
        let content = r#"
# req_id.sub_req: Some Title

**References:**

- in branch main: 2
        "#;

        let mut wiki = Wiki::new();
        wiki.add(PathBuf::from(filename), content).unwrap();

        assert!(
            wiki.req_map.contains_key("req_id.sub_req"),
            "Requirement was not added to the wiki."
        );
        assert!(
            wiki.sub_map.contains_key("req_id"),
            "Parent requirement was not added to sub-map for high-level requirement."
        );
        assert!(
            wiki.sub_map
                .get("req_id")
                .unwrap()
                .contains(&"req_id.sub_req".to_string()),
            "Sub-requirement was not added to parent in sub-map."
        );
        assert_eq!(
            wiki.high_lvl_reqs.len(),
            0,
            "Low-level requirement added to list of high-level requirements"
        );

        let req = wiki.req_map.get("req_id.sub_req").unwrap();
        assert_eq!(
            req.ref_list.len(),
            1,
            "Wrong number of *references* list entries added."
        );
    }

    #[test]
    fn flattend_wiki_one_sub() {
        let filename = "test_file";
        let content = r#"
# req_id: Some Title

**References:**

- in branch main: 2

## req_id.sub_req: Some Title

**References:**

- in branch main: 2
        "#;

        let mut wiki = Wiki::new();
        wiki.add(PathBuf::from(filename), content).unwrap();

        let flattened = wiki.flatten();

        assert_eq!(
            flattened.len(),
            2,
            "Length of flattened wiki did not match number of existing requirements."
        );

        let req_0 = &flattened[0];
        assert_eq!(
            req_0.req_id(),
            "req_id.sub_req",
            "First entry in flattened wiki was not sub-requirement."
        );

        let req_1 = &flattened[1];
        assert_eq!(
            req_1.req_id(),
            "req_id",
            "Second entry in flattened wiki was not high-level requirement."
        );
    }

    #[test]
    fn low_lvl_req_skipped_one_lvl() {
        let filename = "test_file";
        let content = r#"
# req_id: Some Title

**References:**

- in branch main: 2 (1 direct)

## req_id.test.some_test: Some Test

**References:**

- in branch main: 1
        "#;

        let mut wiki = Wiki::new();
        wiki.add(PathBuf::from(filename), content).unwrap();

        assert!(
            wiki.req_map.contains_key("req_id.test.some_test"),
            "Requirement was not added to the wiki."
        );
        assert!(
            wiki.sub_map.contains_key("req_id.test"),
            "Parent requirement was not added to sub-map for mid-level requirement."
        );
        assert!(
            wiki.sub_map
                .get("req_id.test")
                .unwrap()
                .contains(&"req_id.test.some_test".to_string()),
            "Sub-requirement was not added to test requirement in sub-map."
        );

        assert!(
            wiki.sub_map.contains_key("req_id"),
            "Parent requirement was not added to sub-map for high-level requirement."
        );
        assert!(
            wiki.sub_map
                .get("req_id")
                .unwrap()
                .contains(&"req_id.test".to_string()),
            "Sub-requirement was not added to parent in sub-map."
        );
    }

    #[test]
    fn one_implicit_req() {
        let filename = "test_file";
        let content = r#"
# req_id: Some Title

**References:**

- in branch main: 2 (1 direct)

## req_id.test.some_test: Some Test

**References:**

- in branch main: 1
        "#;

        let mut wiki = Wiki::new();
        wiki.add(PathBuf::from(filename), content).unwrap();

        assert!(
            wiki.is_implicit(&"req_id.test".to_string()),
            "`req_id.test` not identified as implicit requirement."
        );

        let flat_wiki = wiki.flatten();

        assert_eq!(
            flat_wiki.len(),
            3,
            "Implicit requirement not added to flattened wiki list."
        );
    }
}
