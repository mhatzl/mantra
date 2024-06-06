#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RequirementSchema {
    pub requirements: Vec<Requirement>,
}

pub use mantra_lang_tracing::ReqId;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Requirement {
    pub id: ReqId,
    pub title: String,
    pub link: String,
    pub manual: bool,
    pub deprecated: bool,
    pub annotation: Option<serde_json::Value>,
}
