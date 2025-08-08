use std::path::PathBuf;

use crate::Line;

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
    pub origin: RequirementOrigin,
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

/// Defines the origin variants of a requirement.
/// [req("req.origin")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum RequirementOrigin {
    /// Marks that a requirement was defined in a markup-based Wiki.
    /// [req("req.origin.wiki")]
    Wiki(WikiRequirementOrigin),
    /// Marks that a requirement was defined in an external source.
    /// [req("req.origin.extern")]
    Extern(ExternRequirementOrigin),
}

/// Struct for the wiki origin of a requirement.
/// [req("req.origin.wiki")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub struct WikiRequirementOrigin {
    /// The file the requirement is defined in in the wiki.
    pub filepath: PathBuf,
    /// The line the requirement is defined at.
    pub line: Line,
    /// Optional URL to the repository of the wiki.
    pub repo_url: Option<String>,
    /// Optional URL to the rendered view of the wiki.
    pub rendered_url: Option<String>,
}

/// Struct for the external origin of a requirement.
/// [req("req.origin.extern")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ExternRequirementOrigin {
    /// The URL a requirement is defined at externally to mantra.
    pub url: String,
}
