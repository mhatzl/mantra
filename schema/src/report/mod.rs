use crate::{ConversionError, product::ProductId, requirements::ReqId};

pub mod overview;

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct RequirementReference {
    pub id: ReqId,
    pub product_id: Option<ProductId>,
    pub state: RequirementState,
    pub optional: bool,
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
#[serde(rename_all = "snake_case")]
pub enum RequirementState {
    Failed = 0,
    Verified = 1,
    Skipped = 2,
    Unverified = 3,
    Deprecated = 4,
    Ignored = 5,
}

impl RequirementState {
    pub fn as_nr(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i64> for RequirementState {
    type Error = ConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RequirementState::Failed),
            1 => Ok(RequirementState::Verified),
            2 => Ok(RequirementState::Skipped),
            3 => Ok(RequirementState::Unverified),
            4 => Ok(RequirementState::Deprecated),
            5 => Ok(RequirementState::Ignored),
            _ => Err(ConversionError::UnknownState),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestRunReference {
    pub name: String,
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
}
