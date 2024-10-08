#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementSchema {
    pub requirements: Vec<Requirement>,
}

pub type ReqId = String;

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Requirement {
    pub id: ReqId,
    pub title: String,
    pub link: String,
    pub manual: bool,
    pub deprecated: bool,
    pub info: Option<serde_json::Value>,
}
