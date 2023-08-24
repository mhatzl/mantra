//! Contains the *references* list
//!
//! [req:wiki.ref_list]

/// Type representing the *references* list.
///
/// [req:wiki.ref_list]
pub type RefList = Vec<RefListEntry>;

/// Represents one entry inside the *references* list.
///
/// [req:wiki.ref_list]
#[derive(Debug)]
pub struct RefListEntry {
    /// The name of the branch for this entry.
    ///
    /// [req:wiki.ref_list]
    pub branch_name: String,
    /// The link to the branch for this entry.
    ///
    /// [req:wiki.ref_list.branch_link]
    pub branch_url: Option<String>,

    /// The reference counter for this entry.
    ///
    /// [req:wiki.ref_list]
    pub ref_cnt: RefCntKind,

    pub is_manual: bool,

    pub is_deprecated: bool,
}

/// Reference counter kind for a requirement.
///
/// [req:req_id.sub_req_id], [req:wiki.ref_list]
#[derive(Debug)]
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
