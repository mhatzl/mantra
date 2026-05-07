use crate::report::{Aggregated, product::ProductMetadata, review::ReviewReference};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ReviewsReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub product: ProductMetadata,
    pub summary: ReviewsSummary,
    pub reviews: Vec<ReviewReference>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ReviewsSummary {
    pub total: i64,
    pub valid: Aggregated,
    pub obsolete: Aggregated,
    pub mandatory_requirements_verified: Aggregated,
}

impl ReviewsSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.valid.cnt += other.valid.cnt;
        self.obsolete.cnt += other.obsolete.cnt;
        self.mandatory_requirements_verified.cnt += other.mandatory_requirements_verified.cnt;

        self.update_percentages();
    }

    pub fn update_percentages(&mut self) {
        self.valid.update_percentage(self.total);
        self.obsolete.update_percentage(self.total);
        self.mandatory_requirements_verified
            .update_percentage(self.total);
    }
}
