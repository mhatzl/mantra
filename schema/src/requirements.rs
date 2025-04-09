#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementSchema {
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    pub requirements: Vec<Requirement>,
}

/// Type alias for a requirement ID.
pub type ReqId = String;

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Requirement {
    /// ID of the requirement.
    pub id: ReqId,
    /// Hash of the requirement content to detect changes.
    #[serde(alias = "content-hash")]
    pub content_hash: Option<String>,
    /// ISO8601 timestamp when the requirement content was last modified.
    #[serde(
        alias = "last-modified-at",
        serialize_with = "time::serde::iso8601::option::serialize",
        deserialize_with = "time::serde::iso8601::option::deserialize"
    )]
    #[schemars(with = "Option<String>")]
    pub last_modified_at: Option<time::OffsetDateTime>,
    /// ISO8601 timestamp when the requirement content was last checked for modification.
    #[serde(
        alias = "last-checked-at",
        serialize_with = "time::serde::iso8601::option::serialize",
        deserialize_with = "time::serde::iso8601::option::deserialize"
    )]
    #[schemars(with = "Option<String>")]
    pub last_checked_at: Option<time::OffsetDateTime>,
    /// Optional list of parent requirements.
    pub parents: Option<Vec<ReqId>>,
    /// Title of the requirement.
    pub title: String,
    /// Link to the origin the requirement is defined.
    pub origin: String,
    /// true: Marks the requirement to require manual verification.
    pub manual: bool,
    /// true: Marks the requirement to be deprecated.
    pub deprecated: bool,
    /// Field to store custom information per requirement.
    pub data: Option<serde_json::Value>,
}
