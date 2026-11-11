use jiff::Zoned;
use serde::Serializer;
use sha2::Digest;

pub mod annotations;
mod encoding;
pub mod media_type;
pub mod product;
pub mod report;
pub mod requirements;
pub mod reviews;
pub mod test_runs;

pub use jiff;
pub use mime_guess;
pub use relative_path as path;

use crate::{media_type::MediaType, product::ProductId};

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
pub type Column = i64;
pub type Origin = serde_json::Value;
pub type Properties = serde_json::value::Map<String, serde_json::Value>;

/// Tries to convert the given string to the time zone aware time [`jiff::Zoned`].
/// The date must be given adhering to the Temporal ISO 8601 grammar.
/// See: https://tc39.es/proposal-temporal/#sec-temporal-iso8601grammar
///
/// If no time zone is set in the string, the time zone of the system is used as fallback.
pub fn datetime_from_str(datetime: &str) -> Result<Zoned, jiff::Error> {
    match datetime.parse::<Zoned>() {
        Ok(zoned) => Ok(zoned),
        Err(err) => {
            if let Ok(dt) = datetime.parse::<jiff::civil::DateTime>()
                && let Ok(zoned) = dt.to_zoned(jiff::tz::TimeZone::system())
            {
                Ok(zoned)
            } else {
                Err(err)
            }
        }
    }
}

pub fn deserialize_zoned<'de, D>(deserializer: D) -> Result<Zoned, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    datetime_from_str(s).map_err(serde::de::Error::custom)
}

pub fn deserialize_opt_zoned<'de, D>(deserializer: D) -> Result<Option<Zoned>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt_s: Option<&str> = serde::Deserialize::deserialize(deserializer)?;
    match opt_s {
        None => Ok(None),
        Some(s) => datetime_from_str(s)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

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
#[serde(deny_unknown_fields)]
pub struct LineSpan {
    pub start: Line,
    pub end: Line,
}

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
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub start_line: Line,
    pub start_column: Option<Column>,
    pub end_line: Option<Line>,
    pub end_column: Option<Column>,
}

pub fn serialize_schema_version<S>(_value: &Option<String>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(SCHEMA_VERSION)
}

pub trait Schema: serde::Serialize {
    fn version(&self) -> &str;
    fn product_id(&self) -> &Option<ProductId>;
    fn origin(&self) -> &Option<Origin>;
    fn media_type(&self) -> &Option<MediaType>;
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub number: i64,
    pub authors: Vec<String>,
    pub summary: String,
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

    pub fn new_serialized<S: serde::Serialize>(value: &S) -> Self {
        let content = serde_json::to_string(value).expect(
            "Types that implement serde::Serialize should never fail to serialize to JSON.",
        );
        Self::new(&content)
    }
}

impl std::fmt::Display for FmtHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
