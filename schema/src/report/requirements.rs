use crate::report::{Aggregated, product::ProductMetadata, requirement::RequirementReference};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementsReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub product: ProductMetadata,
    pub requirements: RequirementsOverview,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementsOverview {
    pub summary: RequirementsSummary,
    pub failed: Vec<RequirementReference>,
    pub skipped: Vec<RequirementReference>,
    pub unverified: Vec<RequirementReference>,
    pub verified: Vec<RequirementReference>,
    pub excluded: Vec<RequirementReference>,
    pub deprecated: Vec<RequirementReference>,
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
pub struct RequirementsSummary {
    pub total: i64,
    /// Metric for how many requirements are non-optional.
    pub mandatory_total: Aggregated,
    /// Metric for how many non-optional requirements are verified.
    pub mandatory_verified: Aggregated,
    /// Metric for how many requirements require manual verification.
    pub manuals_total: Aggregated,
    /// Metric for how many requirements requiring manual verification have been verified.
    pub manuals_verified: Aggregated,
    pub verified: Aggregated,
    pub failed: Aggregated,
    pub skipped: Aggregated,
    pub unverified: Aggregated,
    pub deprecated: Aggregated,
    pub excluded: Aggregated,
}

impl RequirementsSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.mandatory_total.cnt += other.mandatory_total.cnt;
        self.mandatory_verified.cnt += other.mandatory_verified.cnt;

        self.manuals_total.cnt += other.manuals_total.cnt;
        self.manuals_verified.cnt += other.manuals_verified.cnt;

        self.verified.cnt += other.verified.cnt;
        self.failed.cnt += other.failed.cnt;
        self.skipped.cnt += other.skipped.cnt;
        self.unverified.cnt += other.unverified.cnt;
        self.deprecated.cnt += other.deprecated.cnt;
        self.excluded.cnt += other.excluded.cnt;

        self.update_percentages();
    }

    pub fn update_percentages(&mut self) {
        self.mandatory_total.update_percentage(self.total);
        self.mandatory_verified
            .update_percentage(self.mandatory_total.cnt);

        self.manuals_total.update_percentage(self.total);
        self.manuals_verified
            .update_percentage(self.manuals_total.cnt);

        self.verified.update_percentage(self.total);
        self.failed.update_percentage(self.total);
        self.skipped.update_percentage(self.total);
        self.unverified.update_percentage(self.total);
        self.deprecated.update_percentage(self.total);
        self.excluded.update_percentage(self.total);
    }
}
