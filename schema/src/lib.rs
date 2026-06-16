use serde::Serializer;
use sha2::Digest;

pub mod annotations;
mod encoding;
pub mod product;
pub mod report;
pub mod requirements;
pub mod reviews;
pub mod test_runs;

pub use relative_path as path;
pub use time;

/// The version of the schema that is defined in this crate.
/// [req("exchange.versioned")]
pub const SCHEMA_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const PRODUCTS_FOLDER_NAME: &str = "products";
pub const REQUIREMENTS_FOLDER_NAME: &str = "requirements";
pub const REVIEWS_FOLDER_NAME: &str = "reviews";
pub const TEST_RUNS_FOLDER_NAME: &str = "test-runs";
pub const SOURCES_FOLDER_NAME: &str = "sources";

/// Represents a line in a text file.
/// Line numbers start at 1 in *mantra*.
pub type Line = i64;
pub type Origin = serde_json::Value;
pub type Properties = serde_json::value::Map<String, serde_json::Value>;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct LineSpan {
    pub start: Line,
    pub end: Line,
}

fn serialize_schema_version<S>(_value: &Option<String>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(SCHEMA_VERSION)
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Revision {
    pub nr: i64,
    pub authors: Vec<String>,
    pub comment: String,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    sqlx::Type,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
// #[cfg_attr(feature = "sqlx", derive(sqlx_macros::Encode, sqlx_macros::Type))]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct FmtHash(String);

impl FmtHash {
    pub fn hash(&self) -> &str {
        &self.0
    }

    pub fn new(s: &str) -> Self {
        let mut hash = sha2::Sha256::new();
        hash.update(s.as_bytes());
        Self(base16ct::lower::encode_string(&hash.finalize()))
    }

    pub fn with_inner(hash: String) -> Self {
        Self(hash)
    }
}

impl std::fmt::Display for FmtHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: serde::Serialize> From<&S> for FmtHash {
    fn from(value: &S) -> Self {
        let content = serde_json::to_string(value).expect(
            "Types that implement serde::Serialize should never fail to serialize to JSON.",
        );
        Self::new(&content)
    }
}

impl std::str::FromStr for FmtHash {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Number does not match to a known kind.")]
    UnknownKind,
    #[error("Number does not match to a known state.")]
    UnknownState,
    #[error("Given format is unknown")]
    UnknownFormat,
}

#[derive(Debug, thiserror::Error)]
pub enum IdentError {
    #[error("Given ID contains invalid characters.")]
    InvalidCharacter,
}
