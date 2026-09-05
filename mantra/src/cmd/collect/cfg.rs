use std::path::{Path, PathBuf};

use anyhow::Context;
use mantra_schema::{Origin, Properties, path::RelativePathBuf, product::ProductId};

use crate::cfg::{MantraConfigFile, ResolvedProductConfig};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CollectConfig {
    /// Path to the mantra config file that is used to collect the data.
    pub(super) cfg_filepath: PathBuf,
    pub(super) product_cfgs: Vec<ResolvedProductConfig>,
    pub(super) args: CollectArguments,
    pub(super) envs: CollectEnvironmentVariables,
}

impl CollectConfig {
    pub fn new(
        cfg_filepath: PathBuf,
        cfg_file: MantraConfigFile,
        args: CollectArguments,
        envs: CollectEnvironmentVariables,
    ) -> Result<Self, anyhow::Error> {
        let mut product_cfgs = Vec::with_capacity(cfg_file.products.len());

        for product_cfg in cfg_file.products {
            product_cfgs.push(
                product_cfg
                    .resolve(&cfg_file.inheritable_product_cfg)
                    .context("Failed to resolve a product")?,
            );
        }

        Ok(Self {
            cfg_filepath,
            product_cfgs,
            args,
            envs,
        })
    }

    pub fn cfg_filepath(&self) -> &Path {
        &self.cfg_filepath
    }

    pub fn product_cfgs(&self) -> &[ResolvedProductConfig] {
        &self.product_cfgs
    }

    pub fn args(&self) -> &CollectArguments {
        &self.args
    }

    pub fn envs(&self) -> &CollectEnvironmentVariables {
        &self.envs
    }
}

#[derive(
    Debug, Default, Clone, clap::Args, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct CollectArguments {
    /// `true`: tells mantra to replace previously collected content
    /// even if the stored hash is equal to the new one.
    #[arg(long)]
    pub replace_hashed: bool,
    /// Mantra will only collect data for this product ID.
    #[arg(long)]
    pub product_id: Option<ProductId>,
    /// Mantra will set the given value as baseline for the product identified by the product ID.
    #[arg(long, requires = "product_id")]
    pub product_base: Option<String>,
    /// Mantra will set the given value as version for the product identified by the product ID.
    #[arg(long, requires = "product_id")]
    pub product_version: Option<String>,
}

#[derive(
    Debug, Default, Clone, clap::Args, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct CollectEnvironmentVariables {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CollectRequirementsConfig {
    pub path: RelativePathBuf,
    #[serde(default)]
    pub source: RequirementSourceVariant,
    pub origin: Option<Origin>,
    pub properties: Option<Properties>,
    /// Optional pattern a filepath must match to be considered as requirement source.
    pub pattern: Option<String>,
}

/// Supported source variants to define requirements.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RequirementSourceVariant {
    /// Following specific markup syntax in supported markup languages.
    /// e.g. Markdown
    #[default]
    Markup,
    /// Following the JSON schema.
    Schema,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CollectAnnotationsConfig {
    pub path: RelativePathBuf,
    #[serde(default)]
    pub source: AnnotationSourceVariant,
    pub origin: Option<Origin>,
    pub trace_properties: Option<Properties>,
    /// Optional pattern a filepath must match to be considered as annotation source.
    pub pattern: Option<String>,
}

/// Supported source variants to retrieve annotations.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationSourceVariant {
    /// Extracts annotations from file content.
    #[default]
    Content,
    /// Following the JSON schema.
    Schema,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CollectLsifConfig {
    pub path: RelativePathBuf,
    /// The LSIF specification version.
    pub version: Option<String>,
    /// Optional pattern a filepath must match to be considered as LSIF source.
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CollectTestRunsConfig {
    pub path: RelativePathBuf,
    #[serde(default)]
    pub source: TestRunSourceVariant,
    pub origin: Option<Origin>,
    pub test_run_properties: Option<Properties>,
    pub test_case_properties: Option<Properties>,
    /// Optional pattern a filepath must match to be considered as test run source.
    pub pattern: Option<String>,
}

/// Supported source variants to retrieve test runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunSourceVariant {
    /// Following the JSON schema.
    #[default]
    Schema,
    /// Following well-known formats for test and code coverage results.
    #[serde(untagged)]
    WellKnown {
        test: WellKnownTest,
        coverage: WellKnownCoverage,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WellKnownTest {
    /// The well-known test format the data is stored in.
    #[serde(default)]
    pub format: WellKnownTestFormat,
    /// Pattern a filepath must match to be considered as test output source.
    pub pattern: String,
}

/// Supported source variants to define reviews.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WellKnownTestFormat {
    /// Following the [Jenkins JUnit](https://llg.cubic.org/docs/junit/) format.
    #[default]
    Junit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WellKnownCoverage {
    /// The well-known coverage format the data is stored in.
    #[serde(default)]
    pub format: WellKnownCoverageFormat,
    /// Pattern a filepath must match to be considered as coverage source.
    pub pattern: String,
}

/// Supported well-known coverage formats mantra is able to extract coverage data from.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WellKnownCoverageFormat {
    /// Following the [Cobertura Loose] format.
    ///
    /// [Cobertura Loose]: https://github.com/cobertura/cobertura/blob/master/cobertura/src/site/htdocs/xml/coverage-loose.dtd
    #[default]
    CoberturaLoose,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CollectReviewsConfig {
    pub path: RelativePathBuf,
    #[serde(default)]
    pub source: ReviewSourceVariant,
    pub origin: Option<Origin>,
    pub properties: Option<Properties>,
    /// Optional pattern a filepath must match to be considered as review source.
    pub pattern: Option<String>,
}

/// Supported source variants to define reviews.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSourceVariant {
    /// Following specific markup syntax in supported markup languages.
    /// e.g. Markdown
    #[default]
    Markup,
    /// Following the JSON schema.
    Schema,
}
