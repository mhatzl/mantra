use relative_path::RelativePathBuf;

use crate::{
    ConversionError, FmtHash, Line, annotations::TraceKind, product::ProductId,
    requirements::ReqId, test_runs::TestState,
};

pub mod overview;

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct Aggregated {
    pub cnt: i64,
    pub percentage: f32,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TraceReference {
    #[schemars(with = "String")]
    pub filepath: RelativePathBuf,
    pub file_hash: FmtHash,
    pub line: Line,
    pub kind: TraceKind,
}

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
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub state: TestState,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestCaseReference {
    pub test_run_name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub test_run_date: time::OffsetDateTime,
    pub test_case_name: String,
    pub state: TestState,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReviewReference {
    pub name: String,
    #[serde(with = "time::serde::iso8601")]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
}
