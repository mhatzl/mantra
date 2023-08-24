use std::{collections::HashMap, path::PathBuf};

use crate::req::{get_req_heading, ref_list::get_ref_entry, Req, ReqId, ReqMatchingError};

/// Struct representing a wiki that stores requirements.
///
/// [req:wiki]
pub struct Wiki {
    /// Map for all found requirements in the wiki.
    req_map: HashMap<ReqId, Req>,

    /// Map to store sub-requirement IDs of high-level requirements.
    /// Only sub-requirements one level *deeper* are stored.
    ///
    /// [req:req_id.sub_req_id]
    sub_map: HashMap<ReqId, ReqId>,

    /// List of high-level requirements of this wiki.
    /// May be used as starting point to travers the wiki like a tree.
    ///
    /// [req:wiki]
    high_lvl_reqs: Vec<ReqId>,
}

impl From<PathBuf> for Wiki {
    fn from(value: PathBuf) -> Self {
        todo!()
    }
}

impl Wiki {
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
        let mut curr_req = None;

        while let Some(line) = lines.next() {
            line_nr += 1;

            if let Ok(req_heading) = get_req_heading(line) {
                let req_id = req_heading.id.clone();

                if let Some((parent_req_id, _)) = req_id.rsplit_once('.') {
                    self.sub_map.insert(parent_req_id.to_string(), req_id);
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
            } else if let Some(req) = curr_req.as_mut() {
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
    pub fn low_lvl_req_with_1_ref_entry() {
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
}
