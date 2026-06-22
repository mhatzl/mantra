use crate::{IdentError, Origin, Properties, product::ProductId};

/// Defines the schema to exchange requirements related information.
/// [req("exchange.requirements.schema")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct RequirementSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    /// List of requirements.
    pub requirements: Vec<Requirement>,
    /// Optional properties related to all requirements in this entry.
    ///
    /// **Note:** If a requirement sets a property key directly,
    /// the value set at the requirement will be taken.
    pub properties: Option<Properties>,
    /// Optional base origin of the requirements in this entry.
    /// e.g. specific branch or commit from a git repository
    pub origin: Option<Origin>,
}

/// Type for a requirement ID.
/// [req("req.id")]
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct ReqId(String);

impl ReqId {
    pub fn new(id: String) -> Result<Self, IdentError> {
        Ok(Self(id))
    }
}

impl std::ops::Deref for ReqId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for ReqId {
    type Err = IdentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ReqId::new(s.to_owned())
    }
}

impl TryFrom<String> for ReqId {
    type Error = IdentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ReqId::new(value)
    }
}

impl std::fmt::Display for ReqId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// This struct defines the information *mantra* stores about a requirement.
/// [req("req")]
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct Requirement {
    /// ID of the requirement.
    /// [req("req.id")]
    pub id: ReqId,
    /// Optional list of parent requirement IDs.
    /// [req("req.hierarchy.mult_parents")]
    pub parents: Option<Vec<RequirementPk>>,
    /// Title of the requirement.
    /// [req("req.title")]
    pub title: String,
    /// Optional description of the requirement.
    /// [req("req.description")]
    pub description: Option<String>,
    /// Origin where the requirement is defined at.
    /// [req("req.origin")]
    pub origin: Origin,
    /// true: Marks the requirement to require manual verification.
    ///
    /// **Note:** All potential children of such a requirement are also marked
    /// to require manual verification.
    /// [req("req.manual")]
    #[serde(default)]
    pub manual_verification: bool,
    /// true: Marks the requirement to be deprecated.
    ///
    /// **Note:** All potential children of such a requirement are also marked as deprecated.
    /// [req("req.deprecated")]
    #[serde(default)]
    pub deprecated: bool,
    /// true: Instructs mantra to exclude the requirement for the product it is mapped to.
    ///
    /// **Note:** All potential children of such a requirement will also be excluded.
    /// [req("req.exclude")]
    #[serde(default)]
    pub exclude: bool,
    /// true: Instructs mantra to treat the requirement for the product as optional.
    ///
    /// **Note:** All potential children of such a requirement are also marked as optional.
    /// [req("req.optional")]
    #[serde(default)]
    pub optional: bool,
    /// List of custom properties of a requirement.
    /// [req("req.properties")]
    pub properties: Option<Properties>,
}

/// This struct defines the primary key to identify requirements for a product.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct RequirementPk {
    /// ID of the parent requirement.
    /// [req("req.id")]
    pub id: ReqId,
    /// ID of the product the parent requirement is defined in.
    /// If `None`, the parent is assumed to be defined in the same product as the child requirement.
    pub product_id: Option<ProductId>,
}
