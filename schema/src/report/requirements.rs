use crate::report::{Aggregated, product::ProductMetadata, requirement::RequirementReference};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RequirementsReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    pub product: ProductMetadata,
    pub summary: RequirementsSummary,
    pub failed: Vec<RequirementReference>,
    pub skipped: Vec<RequirementReference>,
    pub unverified: Vec<RequirementReference>,
    pub verified: Vec<RequirementReference>,
    pub ignored: Vec<RequirementReference>,
    pub deprecated: Vec<RequirementReference>,
}

#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct RequirementsSummary {
    pub total: i64,
    /// Metric for how many requirements are non-optional.
    pub total_mandatory: Aggregated,
    /// Metric for how many requirements require manual verification.
    pub total_manual: Aggregated,
    /// Metric for how many non-optional requirements are verified.
    pub mandatory_verified: Aggregated,
    /// Metric for how many requirements requiring manual verification have been verified.
    pub mandatory_verified_manual: Aggregated,
    pub verified: Aggregated,
    pub failed: Aggregated,
    pub skipped: Aggregated,
    pub unverified: Aggregated,
    pub deprecated: Aggregated,
    pub ignored: Aggregated,
}

impl RequirementsSummary {
    pub fn add(&mut self, other: &Self) {
        self.total += other.total;

        self.total_mandatory.cnt += other.total_mandatory.cnt;
        self.total_manual.cnt += other.total_manual.cnt;

        self.verified.cnt += other.verified.cnt;
        self.mandatory_verified.cnt += other.mandatory_verified.cnt;
        self.mandatory_verified_manual.cnt += other.mandatory_verified_manual.cnt;
        self.failed.cnt += other.failed.cnt;
        self.skipped.cnt += other.skipped.cnt;
        self.unverified.cnt += other.unverified.cnt;
        self.deprecated.cnt += other.deprecated.cnt;
        self.ignored.cnt += other.ignored.cnt;

        self.update_percentages();
    }

    pub fn update_percentages(&mut self) {
        self.verified.update_percentage(self.total);
        self.failed.update_percentage(self.total);
        self.skipped.update_percentage(self.total);
        self.unverified.update_percentage(self.total);
        self.deprecated.update_percentage(self.total);
        self.ignored.update_percentage(self.total);

        self.total_mandatory.update_percentage(self.total);
        self.total_manual.update_percentage(self.total);
        self.mandatory_verified
            .update_percentage(self.total_mandatory.cnt);
        self.mandatory_verified_manual
            .update_percentage(self.total_manual.cnt);
    }
}
