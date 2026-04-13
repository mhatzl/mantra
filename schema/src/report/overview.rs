use time::OffsetDateTime;

use crate::{
    product::Product,
    report::{RequirementReference, RequirementState, TestRunReference},
    requirements::ReqId,
    reviews::{OverrideTestRun, VerifiedRequirement, review_date_format},
    test_runs::TestCaseState,
};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ProductsOverviewReport {
    pub product_reports: Vec<ProductOverviewReport>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ProductOverviewReport {
    pub product: Product,
    pub requirements: RequirementsOverview,
    pub test_runs: Vec<TestRunOverview>,
    pub reviews: Vec<ReviewOverview>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct RequirementsOverview {
    pub roots: Vec<RequirementOverview>,
    pub all: Vec<RequirementOverview>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct RequirementOverview {
    pub id: ReqId,
    pub title: String,
    pub state: RequirementState,
    pub optional: bool,
    pub parents: Option<Vec<RequirementReference>>,
    pub children: Option<Vec<RequirementReference>>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct TestRunOverview {
    pub name: String,
    #[serde(
        serialize_with = "time::serde::iso8601::serialize",
        deserialize_with = "time::serde::iso8601::deserialize"
    )]
    #[schemars(with = "String")]
    pub utc_date: time::OffsetDateTime,
    pub state: TestCaseState,
    pub test_cases: Option<Vec<TestCaseOverview>>,
    pub parents: Option<Vec<TestRunReference>>,
    pub children: Option<Vec<TestRunReference>>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct TestCaseOverview {
    pub name: String,
    pub state: TestCaseState,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReviewOverview {
    pub name: String,
    /// The UTC date and time the review was started.
    /// [req("review.id")]
    #[serde(with = "review_date_format")]
    #[schemars(
        with = "String",
        regex(
            pattern = r"(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})(T| )(?<hour>\d{2}):(?<minute>\d{2})(?<second>:\d{2}(?<subsecond>\.\d{3})?)?utc(?<utc>(-|+)\d{2})"
        )
    )]
    pub utc_date: OffsetDateTime,
    pub authors: Vec<String>,
    pub requirements: Vec<VerifiedRequirement>,
    pub test_run_overrides: Vec<OverrideTestRun>,
}
