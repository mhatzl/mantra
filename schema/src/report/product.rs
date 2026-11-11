use jiff::Zoned;

use crate::{
    Properties,
    product::ProductId,
    report::{
        formattable_content::FormattableContent, requirement::RequirementReference,
        requirements::RequirementsSummary, review::ReviewReference, reviews::ReviewsSummary,
        test_run::TestRunReference, tests::TestsSummary,
    },
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ProductReportSchema {
    /// The schema version.
    /// [req("exchange.versioned")]
    #[serde(serialize_with = "crate::serialize_schema_version")]
    pub schema_version: Option<String>,
    /// The datetime the product was last collected.
    ///
    /// **Note:** The date must be given adhering to the Temporal ISO 8601 grammar.
    /// See: https://tc39.es/proposal-temporal/#sec-temporal-iso8601grammar
    #[schemars(with = "String")]
    pub last_collected_date: Zoned,
    pub summary: ProductSummary,
    pub root_requirements: Vec<RequirementReference>,
    pub root_test_runs: Vec<TestRunReference>,
    pub reviews: Vec<ReviewReference>,
    /// The product ID.
    ///
    /// TODO: map to requirement
    pub id: ProductId,
    /// The name of the product.
    ///
    /// TODO: map to requirement
    pub name: String,
    /// Optional baseline of the product.
    /// e.g. git branch or commit hash
    ///
    /// TODO: map to requirement
    pub base: Option<String>,
    /// Optional version of the product.
    ///
    /// TODO: map to requirement
    pub version: Option<String>,
    /// Optional link to the homepage of the product.
    ///
    /// TODO: map to requirement
    pub homepage: Option<String>,
    /// Optional link to the repository of the product.
    ///
    /// TODO: map to requirement
    pub repository: Option<String>,
    /// Optional license of the product.
    ///
    /// TODO: map to requirement
    pub license: Option<String>,
    /// Optional description of the product.
    ///
    /// TODO: map to requirement
    pub description: Option<FormattableContent>,
    /// Optional properties of the product.
    ///
    /// TODO: map to requirement
    pub properties: Option<Properties>,
}

impl ProductReportSchema {
    pub fn metadata(&self) -> ProductMetadata {
        ProductMetadata {
            id: self.id.clone(),
            name: self.name.clone(),
            base: self.base.clone(),
            version: self.version.clone(),
            homepage: self.homepage.clone(),
            repository: self.repository.clone(),
            license: self.license.clone(),
        }
    }
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
pub struct ProductSummary {
    pub requirements: RequirementsSummary,
    pub test_cases: TestsSummary,
    pub reviews: ReviewsSummary,
}

impl ProductSummary {
    pub fn add(&mut self, other: &Self) {
        self.requirements.add(&other.requirements);
        self.test_cases.add(&other.test_cases);
        self.reviews.add(&other.reviews);
    }
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub struct ProductMetadata {
    /// The product ID.
    ///
    /// TODO: map to requirement
    pub id: ProductId,
    /// The name of the product.
    ///
    /// TODO: map to requirement
    pub name: String,
    /// Optional baseline of the product.
    /// e.g. git branch or commit hash
    ///
    /// TODO: map to requirement
    pub base: Option<String>,
    /// Optional version of the product.
    ///
    /// TODO: map to requirement
    pub version: Option<String>,
    /// Optional link to the homepage of the product.
    ///
    /// TODO: map to requirement
    pub homepage: Option<String>,
    /// Optional link to the repository of the product.
    ///
    /// TODO: map to requirement
    pub repository: Option<String>,
    /// Optional license of the product.
    ///
    /// TODO: map to requirement
    pub license: Option<String>,
}
