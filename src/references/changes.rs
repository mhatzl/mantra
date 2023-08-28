use std::{collections::HashMap, path::PathBuf, sync::atomic::Ordering};

use crate::{
    req::{ref_list::RefCntKind, Req, ReqId},
    wiki::Wiki,
};

use super::ReferencesMap;

pub struct ReferenceChanges {
    new_cnt_map: HashMap<ReqId, RefCntKind>,
    file_changes: HashMap<PathBuf, Vec<Req>>,
    branch_name: String,
}

impl ReferenceChanges {
    pub fn new(branch_name: String, wiki: &Wiki, ref_map: &ReferencesMap) -> Self {
        let mut new_cnt_map = HashMap::new();
        let mut file_changes = HashMap::new();
        let flat_wiki = wiki.flatten();

        for req in flat_wiki {
            let req_id = req.head.id.clone();

            let new_direct_cnt = ref_map
                .map
                .get(&req_id)
                .map(|atomic_cnt| atomic_cnt.load(Ordering::Relaxed))
                .unwrap_or_default();

            let new_cnt_kind = match wiki.sub_reqs(&req_id) {
                Some(sub_reqs) => RefCntKind::HighLvl {
                    direct_cnt: new_direct_cnt,
                    sub_cnt: sub_ref_cnts(sub_reqs, &branch_name, wiki, &new_cnt_map),
                },
                None => RefCntKind::LowLvl {
                    cnt: new_direct_cnt,
                },
            };

            let kind_changed = match wiki.req_ref_entry(&req_id, &branch_name) {
                Some(req_entry) => match &req_entry.ref_cnt {
                    Some(req_entry_cnt) => req_entry_cnt != &new_cnt_kind,
                    None => true,
                },
                None => true,
            };

            if kind_changed {
                new_cnt_map.insert(req_id, new_cnt_kind);
                file_changes
                    .entry(req.filepath.clone())
                    .or_insert(Vec::new())
                    .push(req);
            }
        }

        ReferenceChanges {
            new_cnt_map,
            file_changes,
            branch_name,
        }
    }
}

/// Calculates the sum of all updated reference counters of the given sub requirements.
///
/// **Note:** This function assumes that the counter for all sub-requirements was already updated.
fn sub_ref_cnts(
    sub_reqs: &Vec<ReqId>,
    branch_name: &str,
    wiki: &Wiki,
    new_cnt_map: &HashMap<ReqId, RefCntKind>,
) -> usize {
    let mut sub_cnt = 0;
    for sub_req in sub_reqs {
        let sub_cnt_kind = new_cnt_map.get(sub_req).unwrap_or_else(|| {
            match wiki.req_ref_entry(sub_req, &branch_name) {
                Some(req_entry) => match &req_entry.ref_cnt {
                    Some(cnt_kind) => cnt_kind,
                    None => &RefCntKind::LowLvl { cnt: 0 },
                },
                None => &RefCntKind::LowLvl { cnt: 0 },
            }
        });

        sub_cnt += match sub_cnt_kind {
            RefCntKind::HighLvl {
                direct_cnt,
                sub_cnt,
            } => direct_cnt + sub_cnt,
            RefCntKind::LowLvl { cnt } => *cnt,
        }
    }
    sub_cnt
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::{references::ReferencesMap, req::ref_list::RefCntKind, wiki::Wiki};

    use super::ReferenceChanges;

    fn setup_wiki() -> Wiki {
        let filename = "test_wiki";
        let content = r#"
# req_id: Some Title

**References:**

- in branch main: 2

## req_id.sub_req: Some Title

**References:**

- in branch main: 1
        "#;

        Wiki::try_from((PathBuf::from(filename), content)).unwrap()
    }

    fn setup_references(wiki: &Wiki) -> ReferencesMap {
        let filename = "test_file";
        // Note: IDs must be identical to the one in `setup_wiki()`.
        let content = "[req:req_id][req:req_id.sub_req]";

        let ref_map = ReferencesMap::with(&mut wiki.requirements());
        ref_map.trace(filename.to_string(), content).unwrap();
        ref_map
    }

    #[test]
    fn high_lvl_cnt_changed_low_lvl_unchanged() {
        let wiki = setup_wiki();
        let ref_map = setup_references(&wiki);
        let branch_name = String::from("main");

        let changes = ReferenceChanges::new(branch_name, &wiki, &ref_map);

        assert_eq!(
            changes.new_cnt_map.len(),
            1,
            "More than one reference counter changed."
        );

        let new_cnt = changes
            .new_cnt_map
            .get("req_id")
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
