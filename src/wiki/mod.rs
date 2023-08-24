use std::collections::HashMap;

use crate::req::{Req, ReqId};

/// Struct representing a wiki that stores requirements.
///
/// [req:wiki]
pub struct Wiki {
    /// Map for all found requirements in the wiki.
    req_map: HashMap<ReqId, Req>,

    /// List of high-level requirements of this wiki.
    /// May be used as starting point to travers the wiki like a tree.
    ///
    /// [req:wiki]
    high_lvl_reqs: Vec<ReqId>,
}
