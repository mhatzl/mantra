#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementSchema {
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    pub requirements: Vec<Requirement>,
}

/// Type alias for a requirement ID
pub type ReqId = String;

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Requirement {
    /// ID of the requirement
    pub id: ReqId,
    /// Optional list of parent requirements
    pub parents: Option<Vec<ReqId>>,
    /// Title of the requirement
    pub title: String,
    /// Link to the origin the requirement is defined
    pub link: String,
    /// true: Marks the requirement to require manual verification
    pub manual: bool,
    /// true: Marks the requirement to be deprecated
    pub deprecated: bool,
    /// Field to store custom information per requirement
    pub info: Option<serde_json::Value>,
}
