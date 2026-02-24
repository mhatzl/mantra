use mantra_schema::product::Product;

use crate::cmd::collect::cfg::{
    CollectAnnotationsConfig, CollectLsifConfig, CollectRequirementsConfig, CollectReviewsConfig,
    CollectTestRunsConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MantraConfigFile {
    #[serde(alias = "product")]
    pub products: Vec<ProductConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProductConfig {
    #[serde(flatten)]
    pub product: Product,
    #[serde(default, alias = "requirement")]
    pub requirements: Vec<CollectRequirementsConfig>,
    #[serde(default, alias = "annotation")]
    pub annotations: Vec<CollectAnnotationsConfig>,
    #[serde(default, alias = "language_server_index_format")]
    pub lsif: Vec<CollectLsifConfig>,
    #[serde(default, alias = "test-run")]
    pub test_runs: Vec<CollectTestRunsConfig>,
    #[serde(default, alias = "review")]
    pub reviews: Vec<CollectReviewsConfig>,
}

#[derive(clap::Parser)]
pub struct CliConfig {
    #[command(flatten)]
    pub db: crate::db::Config,
    #[arg(long, default_value = "mantra.json5", alias = "config")]
    pub config_filepath: std::path::PathBuf,
    #[command(subcommand)]
    pub cmd: crate::cmd::Cmd,
}
