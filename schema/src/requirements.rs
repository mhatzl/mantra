/// Defines the schema to exchange requirements related information.
/// [req("exchange.requirements.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct RequirementSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub version: Option<String>,
    /// List of requirements.
    pub requirements: Vec<Requirement>,
    /// Optional metadata related to all requirements in this entry.
    pub metadata: Option<serde_json::Value>,
    /// Optional base origin of the requirements in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<serde_json::Value>,
}

/// Type alias for a requirement ID.
/// [req("req.id")]
pub type ReqId = String;

/// This struct defines the information *mantra* stores about a requirement.
/// [req("req")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct Requirement {
    /// ID of the requirement.
    /// [req("req.id")]
    pub id: ReqId,
    /// Hash of the requirement content to detect changes.
    ///
    /// If not provided, will be computed using the fields:
    /// parents, title, origin, manual_verification, deprecated, properties
    pub content_hash: Option<String>,
    /// Optional list of parent requirement IDs.
    /// [req("req.hierarchy.mult_parents")]
    pub parents: Option<Vec<ReqId>>,
    /// Title of the requirement.
    /// [req("req.title")]
    pub title: String,
    /// Optional description of the requirement.
    /// [req("req.description")]
    pub description: Option<String>,
    /// Origin where the requirement is defined at.
    /// [req("req.origin")]
    pub origin: serde_json::Value,
    /// true: Marks the requirement to require manual verification.
    /// [req("req.manual")]
    #[serde(default)]
    pub manual_verification: bool,
    /// true: Marks the requirement to be deprecated.
    /// [req("req.deprecated")]
    #[serde(default)]
    pub deprecated: bool,
    /// List of custom properties of a requirement.
    /// [req("req.properties")]
    #[serde(default)]
    pub properties: Vec<serde_json::Value>,
}
