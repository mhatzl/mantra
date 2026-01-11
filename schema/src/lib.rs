use serde::Serializer;

pub mod product;
pub mod requirements;
pub mod reviews;
pub mod testcov;
pub mod traces;

/// The version of the schema that is defined in this crate.
/// [req("exchange.versioned")]
pub const SCHEMA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Represents a line in a text file.
/// Line numbers start at 1 in *mantra*.
pub type Line = u32;

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
