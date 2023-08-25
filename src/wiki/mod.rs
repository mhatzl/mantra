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

    fn new() -> Self {
        Wiki {
            req_map: HashMap::new(),
            sub_map: HashMap::new(),
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

            if line.starts_with("```") || line.starts_with("~~~") {
                in_verbatim_context = !in_verbatim_context;
            }

            if !in_verbatim_context {
                if let Ok(req_heading) = get_req_heading(line) {
                    let req_id = req_heading.id.clone();

                    if let Some((parent_req_id, _)) = req_id.rsplit_once('.') {
                        let sub_entry = self
                            .sub_map
                            .entry(parent_req_id.to_string())
                            .or_insert(Vec::new());
                        sub_entry.push(req_id);
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
                        if let Some(prev_req) = self.req_map.insert(req_id, std::mem::take(req)) {
                            return Err(WikiError::DuplicateReqId {
                                filename: prev_req.filename,
                                line_nr: prev_req.line_nr,
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
            if let Some(prev_req) = self.req_map.insert(req_id, req) {
                return Err(WikiError::DuplicateReqId {
                    filename: prev_req.filename,
                    line_nr: prev_req.line_nr,
                });
            }
        }

        Ok(added_reqs)
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
    pub fn high_lvl_req_with_1_ref_entry() {
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
    pub fn req_missing_ref_list_start() {
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
    pub fn low_lvl_req_with_1_ref_entry() {
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
}
