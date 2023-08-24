use std::{collections::HashMap, sync::atomic::AtomicUsize};

use crate::req::ReqId;

/// HashMap to store the current reference counter for direct references to requirements.
/// This counter is used to update/validate the existing reference counts.
///
/// **Note:** Atomic to be updated concurrently.
///
/// [req:ref_req]
pub type TraceMap = HashMap<ReqId, AtomicUsize>;
