use serde::Serializer;

pub mod annotations;
pub mod product;
pub mod requirements;
pub mod reviews;
pub mod test_runs;

/// The version of the schema that is defined in this crate.
/// [req("exchange.versioned")]
pub const SCHEMA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Represents a line in a text file.
/// Line numbers start at 1 in *mantra*.
pub type Line = u32;
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
    pub nr: usize,
    pub authors: String,
    pub comment: String,
}
