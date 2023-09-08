//! Contains the [`ReferenceChanges`] struct to get the reference differences between wiki and project.
//!
//! [req:sync]

use std::{
    collections::{hash_map::IntoIter, HashMap, HashSet},
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};

use crate::{
    references::ReferencesError,
    wiki::{
        ref_list::{RefCntKind, RefListEntry},
        req::{Req, ReqId},
        Wiki, WikiReq,
    },
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
    branch_link: Option<Arc<String>>,
}

impl ReferenceChanges {
    /// Creates [`ReferenceChanges`] from the given wiki and reference map.
    /// Found references are compared against the references entry in the wiki for the given branch name.
    ///
    /// # Possible Errors
    ///
    /// - [`ReferencesError::DeprecatedReqReferenced`]
    pub fn new(
        branch_name: Arc<String>,
        branch_link: Option<Arc<String>>,
        wiki: &Wiki,
        ref_map: &ReferencesMap,
    ) -> Result<Self, ReferencesError> {
        let mut changes = ReferenceChanges {
            new_cnt_map: HashMap::new(),
            implicits_cnt_map: HashMap::new(),
            file_changes: HashMap::new(),
            branch_name,
            branch_link,
        };

        changes.update_cnts(wiki, ref_map)?;
        Ok(changes)
    }

    /// Returns an iterator over all requirements with changed counters.
    ///
    /// [req:check]
    pub fn cnt_changes(self) -> IntoIter<ReqId, RefCntKind> {
        self.new_cnt_map.into_iter()
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
                        branch_link: self.branch_link.clone(), // [req:wiki.ref_list.branch_link] (see DR-20230906_2 for more info)
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
    ///
    /// # Possible Errors
    ///
    /// - [`ReferencesError::DeprecatedReqReferenced`]
    fn update_cnts(&mut self, wiki: &Wiki, ref_map: &ReferencesMap) -> Result<(), ReferencesError> {
        let flat_wiki = wiki.flatten();

        for req in flat_wiki {
            let req_id = req.req_id().clone();

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
                        Some(req_entry) => {
                            // [req:wiki.ref_list.deprecated]
                            if req_entry.is_deprecated && new_cnt_kind != RefCntKind::Untraced {
                                let err = logid::err!(ReferencesError::DeprecatedReqReferenced {
                                    req_id: req_id.clone(),
                                    branch_name: self.branch_name.to_string()
                                });

                                if crate::globals::early_exit() {
                                    return err;
                                } else {
                                    continue;
                                }
                            }

                            req_entry.ref_cnt != new_cnt_kind
                        }
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

        Ok(())
    }

    /// Calculates the sum of all updated reference counters of the given sub requirements.
    ///
    /// **Note:** This function assumes that the counter for all sub-requirements was already updated.
    fn sub_ref_cnts(&mut self, sub_reqs: &HashSet<ReqId>, wiki: &Wiki) -> usize {
        let mut sub_cnt = 0;
        for sub_req in sub_reqs {
            let opt_ref_entry = wiki.req_ref_entry(sub_req, &self.branch_name);

            let mut sub_cnt_kind = self.new_cnt_map.get(sub_req).copied().unwrap_or_else(|| {
                match opt_ref_entry {
                    Some(entry) => entry.ref_cnt,
                    None => {
                        if wiki.is_implicit(sub_req) {
                            // Note: Counter for implicit requirements are already up-to-date,
                            // because like sub-requirements, they appear in the flattened wiki before the high-level requirement.
                            *self
                                .implicits_cnt_map
                                .get(sub_req)
                                .expect("Implicit requirement not in implicit cnt-map.")
                        } else {
                            // Note: Might be `None` for sub-requirements that are **not** *active* in this branch.
                            RefCntKind::Untraced
                        }
                    }
                }
            });

            // [req:wiki.ref_list.manual]
            if opt_ref_entry.map_or_else(|| false, |entry| entry.is_manual) {
                sub_cnt_kind = match sub_cnt_kind {
                    RefCntKind::HighLvl {
                        direct_cnt,
                        sub_cnt,
                    } => RefCntKind::HighLvl {
                        direct_cnt: direct_cnt + 1,
                        sub_cnt,
                    },
                    RefCntKind::LowLvl { cnt } => RefCntKind::LowLvl { cnt: cnt + 1 },
                    RefCntKind::Untraced => RefCntKind::LowLvl { cnt: 1 },
                };
            }

            sub_cnt += match sub_cnt_kind {
                RefCntKind::HighLvl {
                    direct_cnt,
                    sub_cnt,
                } => direct_cnt + sub_cnt,
                RefCntKind::LowLvl { cnt } => cnt,
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

        let ref_map = ReferencesMap::with(&mut wiki.req_ids());
        ref_map.trace(&PathBuf::from(filename), content).unwrap();
        ref_map
    }

    #[test]
    fn high_lvl_cnt_changed_low_lvl_unchanged() {
        let wiki = setup_wiki();
        let ref_map = setup_references(&wiki);
        let branch_name = String::from("main");

        let changes = ReferenceChanges::new(branch_name.into(), None, &wiki, &ref_map).unwrap();

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

    fn setup_partial_referenced_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# ref_req: Some Title

**References:**

- in branch main: 2 (1 direct)

## ref_req.test: Some Title

        "#;

        Wiki::try_from((PathBuf::from(filename), content)).unwrap()
    }

    #[test]
    fn branch_link_updated_for_new_ref_entries() {
        let wiki = setup_partial_referenced_wiki();
        let ref_map = setup_references(&wiki);
        let branch_name = String::from("main");
        let branch_link = String::from("https://github.com/mhatzl/mantra/tree/main");

        let changes = ReferenceChanges::new(
            branch_name.into(),
            Some(branch_link.clone().into()),
            &wiki,
            &ref_map,
        )
        .unwrap();

        assert_eq!(
            changes.new_cnt_map.len(),
            1,
            "More than one reference counter changed."
        );

        let file_changes = changes.ordered_file_changes();
        let test_file_changes = &file_changes[0].1;

        assert_eq!(
            test_file_changes.len(),
            1,
            "More than one requirement reference changed."
        );

        let low_lvl_req = &test_file_changes[0];
        assert_eq!(
            low_lvl_req.head.id, "ref_req.test",
            "Wrong requirement Id changed."
        );
        assert_eq!(
            low_lvl_req.ref_list.len(),
            1,
            "More than one ref entry created."
        );

        let ref_entry = &low_lvl_req.ref_list[0];
        assert_eq!(
            ref_entry.branch_link,
            Some(branch_link.into()),
            "Branch link was not added to new ref entry."
        );
    }

    fn setup_deprecated_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# ref_req: Some Title

**References:**

- in branch main: deprecated
        "#;

        Wiki::try_from((PathBuf::from(filename), content)).unwrap()
    }

    fn setup_deprecated_references(wiki: &Wiki) -> ReferencesMap {
        let filename = "test_file";
        // Note: IDs must be identical to the one in `setup_wiki()`.
        let content = "[req:ref_req]";

        let ref_map = ReferencesMap::with(&mut wiki.req_ids());
        ref_map.trace(&PathBuf::from(filename), content).unwrap();
        ref_map
    }

    #[test]
    fn deprecated_req_referenced() {
        let wiki = setup_deprecated_wiki();
        let ref_map = setup_deprecated_references(&wiki);
        let branch_name = String::from("main");
        let branch_link = String::from("https://github.com/mhatzl/mantra/tree/main");

        let changes = ReferenceChanges::new(
            branch_name.into(),
            Some(branch_link.clone().into()),
            &wiki,
            &ref_map,
        );

        assert!(
            changes.is_err(),
            "Referencing deprecated requirement did not result in error."
        );
    }

    fn setup_manual_verified_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# ref_req: Some Title

**References:**

- in branch main: 1

## ref_req.test: Some Title

**References:**

- in branch main: manual
        "#;

        Wiki::try_from((PathBuf::from(filename), content)).unwrap()
    }

    #[test]
    fn manual_flag_for_low_lvl_ref_entry() {
        let wiki = setup_manual_verified_wiki();
        let ref_map = setup_references(&wiki);
        let branch_name = String::from("main");
        let branch_link = String::from("https://github.com/mhatzl/mantra/tree/main");

        let changes = ReferenceChanges::new(
            branch_name.into(),
            Some(branch_link.clone().into()),
            &wiki,
            &ref_map,
        )
        .unwrap();

        let file_changes = changes.ordered_file_changes();
        let test_file_changes = &file_changes[0].1;

        let high_lvl_ref_entry = &test_file_changes[0].ref_list[0];

        assert_eq!(
            high_lvl_ref_entry.to_string(),
            "- in branch main: 3 (1 direct)",
            "*manual* flag not correctly counted in parent requirement."
        );

        let low_lvl_ref_entry = &test_file_changes[1].ref_list[0];

        assert_eq!(
            low_lvl_ref_entry.to_string(),
            "- in branch main: manual + 1",
            "*manual* flag + reference not correctly displayed."
        );
    }
}
