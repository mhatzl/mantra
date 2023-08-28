use std::{
    collections::{hash_map::Keys, HashMap},
    path::PathBuf,
};

use walkdir::WalkDir;

use crate::req::{get_req_heading, ref_list::get_ref_entry, Req, ReqId, ReqMatchingError};

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
    /// [req:req_id.sub_req_id]
    sub_map: HashMap<ReqId, Vec<ReqId>>,

    /// Map to store the requirement ID of the direct parent requirement.
    ///
    /// **Note:** High-level requirements are not added to this map.
    ///
    /// [req:req_id.sub_req_id]
    parent_map: HashMap<ReqId, ReqId>,

    /// List of high-level requirements of this wiki.
    /// May be used as starting point to travers the wiki like a tree.
    ///
    /// [req:wiki]
    high_lvl_reqs: Vec<ReqId>,
}

impl TryFrom<PathBuf> for Wiki {
    type Error = WikiError;

    fn try_from(req_path: PathBuf) -> Result<Self, Self::Error> {
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
                    let filename = dir_entry.file_name().to_string_lossy().to_string();
                    let content = std::fs::read_to_string(dir_entry.path())
                        .map_err(|_| WikiError::CouldNotAccessFile(filename.clone()))?;
                    wiki.add(filename, &content)?;
                }
            }
        } else {
            let filename = req_path.to_string_lossy().to_string();
            let content = std::fs::read_to_string(req_path)
                .map_err(|_| WikiError::CouldNotAccessFile(filename.clone()))?;
            wiki.add(filename, &content)?;
        }

        Ok(wiki)
    }
}

impl TryFrom<(String, &str)> for Wiki {
    type Error = WikiError;

    /// Tries to create a wiki given (filename, content).
    fn try_from(value: (String, &str)) -> Result<Self, Self::Error> {
        let filename = value.0;
        let content = value.1;

        let mut wiki = Wiki::new();
        wiki.add(filename, content)?;
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

    pub fn high_lvl_reqs(&self) -> &Vec<ReqId> {
        &self.high_lvl_reqs
    }

    pub fn sub_reqs(&self, req_id: &ReqId) -> Option<&Vec<ReqId>> {
        self.sub_map.get(req_id)
    }

    pub fn req(&self, req_id: &ReqId) -> Option<&Req> {
        self.req_map.get(req_id)
    }

    fn new() -> Self {
        Wiki {
            req_map: HashMap::new(),
            sub_map: HashMap::new(),
            parent_map: HashMap::new(),
            high_lvl_reqs: Vec::new(),
        }
    }

    /// Iterate through the given content, and add found requirements.
    fn add(&mut self, filename: String, content: &str) -> Result<usize, WikiError> {
        let mut lines = content.lines();
        let mut line_nr = 0;

        let mut added_reqs = 0;
        let mut added_refs = 0;
        let mut has_references_list = false;
        let mut curr_req = None;

        let mut in_verbatim_context = false;

        while let Some(line) = lines.next() {
            line_nr += 1;

            if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
                in_verbatim_context = !in_verbatim_context;
            }

            if !in_verbatim_context {
                if let Ok(req_heading) = get_req_heading(line) {
                    let req_id = req_heading.id.clone();

                    if let Some((parent_req_id, _)) = req_id.rsplit_once('.') {
                        self.sub_map
                            .entry(parent_req_id.to_string())
                            .or_insert(Vec::new())
                            .push(req_id.clone());

                        let prev_parent_id = self
                            .parent_map
                            .insert(req_id.clone(), parent_req_id.to_string());
                        if let Some(prev_parent) = prev_parent_id {
                            return Err(WikiError::MoreThanOneParent {
                                req_id: req_id.clone(),
                                parent_1: prev_parent,
                                parent_2: parent_req_id.to_string(),
                            });
                        }
                    } else {
                        self.high_lvl_reqs.push(req_id);
                    }

                    curr_req = Some(Req {
                        head: req_heading,
                        ref_list: Vec::new(),
                        filename: filename.clone(),
                        line_nr,
                        wiki_link: None,
                    })
                } else if line.starts_with("**References:**") {
                    has_references_list = true;
                } else if line.starts_with("#") || (added_refs > 0 && line.is_empty()) {
                    if let Some(req) = curr_req.as_mut() {
                        let req_id = req.head.id.clone();
                        let prev_req = self.req_map.insert(req_id, std::mem::take(req));
                        if let Some(prev) = prev_req {
                            return Err(WikiError::DuplicateReqId {
                                filename: prev.filename,
                                line_nr: prev.line_nr,
                            });
                        }
                    }

                    added_refs = 0;
                    curr_req = None;
                    has_references_list = false;
                } else if has_references_list {
                    if let Some(req) = curr_req.as_mut() {
                        match get_ref_entry(line) {
                            Ok(entry) => {
                                req.ref_list.push(entry);
                                added_refs += 1;
                            }
                            Err(ReqMatchingError::NoMatchFound) => continue,
                            Err(err) => {
                                return Err(WikiError::InvalidRefListEntry {
                                    filename,
                                    line_nr,
                                    cause: err.to_string(),
                                })
                            }
                        }
                    }
                }
            }
        }

        // Last found requirement might not have been inserted in loop, because we were waiting for the *references* list.
        if let Some(req) = curr_req {
            added_reqs += 1;
            let req_id = req.head.id.clone();
            let prev_req = self.req_map.insert(req_id, req);
            if let Some(prev) = prev_req {
                return Err(WikiError::DuplicateReqId {
                    filename: prev.filename,
                    line_nr: prev.line_nr,
                });
            }
        }

        Ok(added_reqs)
    }

    /// Flattens this wiki, starting at the first *leaf* requirement of the first high-level requirement.
    /// Resulting in a depth-first flat representation of the requirement hierarchy of the wiki.
    pub(crate) fn flatten(&self) -> Vec<Req> {
        let mut flat_wiki = Vec::with_capacity(self.req_cnt());

        for req in &self.high_lvl_reqs {
            self.flatten_req(req, &mut flat_wiki);
        }

        flat_wiki
    }

    fn flatten_req(&self, req_id: &ReqId, flat_wiki: &mut Vec<Req>) {
        if let Some(sub_reqs) = self.sub_map.get(req_id) {
            for sub_req_id in sub_reqs {
                self.flatten_req(sub_req_id, flat_wiki);
            }
        }

        let req = self
            .req(req_id)
            .expect(format!("Requirement with ID '{}' not in wiki, but ID is.", req_id).as_str());
        flat_wiki.push(req.clone());
    }
}

#[derive(Debug)]
pub enum WikiError {
    CouldNotAccessFile(String),

    /// Duplicate requirement ID found.
    DuplicateReqId {
        /// Name of the file the ID is already specified.
        filename: String,
        /// Line number in the file the ID is already specified.
        line_nr: usize,
    },

    MoreThanOneParent {
        req_id: String,
        parent_1: String,
        parent_2: String,
    },

    InvalidRefListEntry {
        filename: String,
        line_nr: usize,
        cause: String,
    },
}

#[cfg(test)]
mod test {
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
        wiki.add(filename.to_string(), content).unwrap();

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
        // Note: String moved to the most left to get correct new line behavior.
        let content = r#"
# req_id: Some Title

- in branch main: 2
        "#;

        let mut wiki = Wiki::new();
        wiki.add(filename.to_string(), content).unwrap();

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
        // Note: String moved to the most left to get correct new line behavior.
        let content = r#"
# req_id.sub_req: Some Title

**References:**

- in branch main: 2
        "#;

        let mut wiki = Wiki::new();
        wiki.add(filename.to_string(), content).unwrap();

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
        // Note: String moved to the most left to get correct new line behavior.
        let content = r#"
# req_id: Some Title

**References:**

- in branch main: 2

## req_id.sub_req: Some Title

**References:**

- in branch main: 2
        "#;

        let mut wiki = Wiki::new();
        wiki.add(filename.to_string(), content).unwrap();

        let flattened = wiki.flatten();

        assert_eq!(
            flattened.len(),
            2,
            "Length of flattened wiki did not match number of existing requirements."
        );

        let req_0 = &flattened[0];
        assert_eq!(
            req_0.head.id, "req_id.sub_req",
            "First entry in flattened wiki was not sub-requirement."
        );

        let req_1 = &flattened[1];
        assert_eq!(
            req_1.head.id, "req_id",
            "Second entry in flattened wiki was not high-level requirement."
        );
    }
}
