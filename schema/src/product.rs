use std::ops::Deref;

use relative_path::RelativePathBuf;

use crate::{IdentError, Properties, encoding::TargetEncoding};

/// Type for a product ID.
///
/// TODO: map to requirement
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
pub struct ProductId(String);

impl ProductId {
    pub fn new(id: String) -> Result<Self, IdentError> {
        Ok(Self(id))
    }

    pub fn url_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Url)
    }

    pub fn os_path(&self) -> RelativePathBuf {
        self.encode_path(TargetEncoding::Os)
    }

    fn encode_path(&self, target: TargetEncoding) -> RelativePathBuf {
        let limit_id = crate::encoding::limit_str_len(&self.0);
        let encoded_id = crate::encoding::encode(&limit_id, target);

        RelativePathBuf::from(super::PRODUCTS_FOLDER_NAME).join(encoded_id)
    }
}

impl Deref for ProductId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for ProductId {
    type Err = IdentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ProductId::new(s.to_owned())
    }
}

impl TryFrom<String> for ProductId {
    type Error = IdentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ProductId::new(value)
    }
}

impl std::fmt::Display for ProductId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// This struct defines the information *mantra* stores about a product.
///
/// TODO: map to requirement
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Product {
    /// The product ID.
    ///
    /// TODO: map to requirement
    pub id: ProductId,
    /// The name of the product.
    ///
    /// TODO: map to requirement
    pub name: String,
    /// Optional baseline of the product.
    /// e.g. git branch or commit hash
    ///
    /// TODO: map to requirement
    pub base: Option<String>,
    /// Optional version of the product.
    ///
    /// TODO: map to requirement
    pub version: Option<String>,
    /// Optional link to the homepage of the product.
    ///
    /// TODO: map to requirement
    pub homepage: Option<String>,
    /// Optional link to the repository of the product.
    ///
    /// TODO: map to requirement
    pub repository: Option<String>,
    /// Optional license of the product.
    ///
    /// TODO: map to requirement
    pub license: Option<String>,
    /// Optional description of the product.
    ///
    /// TODO: map to requirement
    pub description: Option<String>,
    /// Optional properties of the product.
    ///
    /// TODO: map to requirement
    pub properties: Option<Properties>,
}
