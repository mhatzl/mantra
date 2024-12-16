use serde::Serializer;

pub mod coverage;
pub mod requirements;
pub mod reviews;
pub mod traces;

pub type Line = u32;

pub const SCHEMA_VERSION: &str = env!("CARGO_PKG_VERSION");

fn serialize_schema_version<S>(_value: &Option<String>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(SCHEMA_VERSION)
}
