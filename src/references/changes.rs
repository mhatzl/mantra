//! Contains the [`ReferenceChanges`] struct to get the reference differences between wiki and project.
//!
//! [req:sync]

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};

use crate::wiki::{
    ref_list::{RefCntKind, RefListEntry},
    req::{Req, ReqId},
    Wiki, WikiReq,
};

use super::ReferencesMap;

/// Keeps track of changes to requirement references.
///
/// [req:sync]
#[derive(Debug)]
pub struct ReferenceChanges {
    new_cnt_map: HashMap<ReqId, RefCntKind>,
    implicits_cnt_map: HashMap<ReqId, RefCntKind>,
    file_changes: HashMap<PathBuf, Vec<Req>>,
    branch_name: Arc<String>,
}

impl ReferenceChanges {
    /// Creates [`ReferenceChanges`] from the given wiki and reference map.
    /// Found references are compared against the references entry in the wiki for the given branch name.
    pub fn new(branch_name: Arc<String>, wiki: &Wiki, ref_map: &ReferencesMap) -> Self {
        let mut changes = ReferenceChanges {
            new_cnt_map: HashMap::new(),
            implicits_cnt_map: HashMap::new(),
            file_changes: HashMap::new(),
            branch_name,
        };

        changes.update_cnts(wiki, ref_map);
        changes
    }

    /// Returns filepaths and updated requirements if their reference counters changed.
    /// Requirements are ordered by line number in ascending order.
    /// This order helps to apply changes in one step.
    ///
    /// **Note:** Only files and requirements that changed are returned.
    ///
    /// [req:sync]
    pub fn ordered_file_changes(&self) -> Vec<(&PathBuf, Vec<Req>)> {
        let mut ordered_file_changes = Vec::with_capacity(self.file_changes.len());
        for (filepath, changes) in self.file_changes.iter() {
            let mut ordered_changes = changes.clone();
            ordered_changes.sort_by(|a, b| a.line_nr.cmp(&b.line_nr));

            for req in ordered_changes.iter_mut() {
                let req_id = &req.head.id;

                let new_cnt_kind = self
                    .new_cnt_map
                    .get(req_id)
                    .expect("Changed requirement had no new counter in `new_cnt_map`.")
                    .to_owned();

                match req
                    .ref_list
                    .iter_mut()
                    .find(|entry| entry.branch_name == self.branch_name)
                {
                    Some(entry) => entry.ref_cnt = new_cnt_kind,
                    None => req.ref_list.push(RefListEntry {
                        branch_name: self.branch_name.clone(),
                        branch_link: None,
                        ref_cnt: new_cnt_kind,
                        is_manual: false,
                        is_deprecated: false,
                    }),
                }
            }

            ordered_file_changes.push((filepath, ordered_changes));
        }

        ordered_file_changes
    }

    /// Updates the reference counters for all requirements, starting from the first leaf requirement of the first high-level requirement.
    ///
    /// **Note:** Only changed counters are added to `self.new_cnt_map`.
    fn update_cnts(&mut self, wiki: &Wiki, ref_map: &ReferencesMap) {
        let flat_wiki = wiki.flatten();

        for req in flat_wiki {
            let req_id = req.req_id().clone();

            // TODO: Check for *manual* flag
            let new_direct_cnt = ref_map
                .map
                .get(&req_id)
                .map(|atomic_cnt| atomic_cnt.load(Ordering::Relaxed))
                .unwrap_or_default();

            let new_cnt_kind = match wiki.sub_reqs(&req_id) {
                Some(sub_reqs) => {
                    let sub_cnt = self.sub_ref_cnts(sub_reqs, wiki);
                    if new_direct_cnt == 0 && sub_cnt == 0 {
                        RefCntKind::Untraced
                    } else {
                        RefCntKind::HighLvl {
                            direct_cnt: new_direct_cnt,
                            sub_cnt,
                        }
                    }
                }
                None => {
                    if new_direct_cnt == 0 {
                        RefCntKind::Untraced
                    } else {
                        RefCntKind::LowLvl {
                            cnt: new_direct_cnt,
                        }
                    }
                }
            };

            match req {
                WikiReq::Explicit { req: explicit_req } => {
                    let kind_changed = match wiki.req_ref_entry(&req_id, &self.branch_name) {
                        Some(req_entry) => req_entry.ref_cnt != new_cnt_kind,
                        // Note: Might happen if the requirement had no references in this branch before.
                        None => new_cnt_kind != RefCntKind::Untraced,
                    };

                    if kind_changed {
                        self.new_cnt_map.insert(req_id, new_cnt_kind);
                        self.file_changes
                            .entry(explicit_req.filepath.clone())
                            .or_insert(Vec::new())
                            .push(explicit_req);
                    }
                }
                WikiReq::Implicit {
                    req_id: implicit_req_id,
                } => {
                    self.implicits_cnt_map.insert(implicit_req_id, new_cnt_kind);
                }
            }
        }
    }

    /// Calculates the sum of all updated reference counters of the given sub requirements.
    ///
    /// **Note:** This function assumes that the counter for all sub-requirements was already updated.
    fn sub_ref_cnts(&mut self, sub_reqs: &HashSet<ReqId>, wiki: &Wiki) -> usize {
        let mut sub_cnt = 0;
        for sub_req in sub_reqs {
            let sub_cnt_kind = self.new_cnt_map.get(sub_req).unwrap_or_else(|| {
                match wiki.req_ref_entry(sub_req, &self.branch_name) {
                    Some(req_entry) => &req_entry.ref_cnt,
                    None => {
                        if wiki.is_implicit(sub_req) {
                            // Note: Counter for implicit requirements are already up-to-date,
                            // because like sub-requirements, they appear in the flattened wiki before the high-level requirement.
                            self.implicits_cnt_map
                                .get(sub_req)
                                .expect("Implicit requirement not in implicit cnt-map.")
                        } else {
                            // Note: Might be `None` for sub-requirements that are **not** *active* in this branch.
                            &RefCntKind::Untraced
                        }
                    }
                }
            });

            sub_cnt += match sub_cnt_kind {
                RefCntKind::HighLvl {
                    direct_cnt,
                    sub_cnt,
                } => direct_cnt + sub_cnt,
                RefCntKind::LowLvl { cnt } => *cnt,
                // TODO: Check for *manual* flag here
                RefCntKind::Untraced => 0,
            }
        }
        sub_cnt
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn setup_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# ref_req: Some Title

**References:**

- in branch main: 2

## ref_req.test: Some Title

**References:**

- in branch main: 1
        "#;

        Wiki::try_from((PathBuf::from(filename), content)).unwrap()
    }

    fn setup_references(wiki: &Wiki) -> ReferencesMap {
        let filename = "test_file";
        // Note: IDs must be identical to the one in `setup_wiki()`.
        let content = "[req:ref_req][req:ref_req.test]";

        let ref_map = ReferencesMap::with(&mut wiki.requirements());
        ref_map.trace(&PathBuf::from(filename), content).unwrap();
        ref_map
    }

    #[test]
    fn high_lvl_cnt_changed_low_lvl_unchanged() {
        let wiki = setup_wiki();
        let ref_map = setup_references(&wiki);
        let branch_name = String::from("main");

        let changes = ReferenceChanges::new(branch_name.into(), &wiki, &ref_map);

        assert_eq!(
            changes.new_cnt_map.len(),
            1,
            "More than one reference counter changed."
        );

        let new_cnt = changes
            .new_cnt_map
            .get("ref_req")
            .expect("High-level requirement did not change.");
        assert_eq!(
            new_cnt,
            &RefCntKind::HighLvl {
                direct_cnt: 1,
                sub_cnt: 1
            },
        );
    }
}
