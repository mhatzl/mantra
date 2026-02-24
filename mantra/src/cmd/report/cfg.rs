use std::path::PathBuf;

use mantra_schema::product::ProductId;

pub struct ReportConfig {
    /// Path to the mantra config file that is used to collect the data.
    pub cfg_filepath: PathBuf,
    pub args: ReportArguments,
    pub envs: ReportEnvironmentVariables,
}

#[derive(Debug, Clone, clap::Args)]
pub struct ReportArguments {
    #[clap(long)]
    pub formats: Vec<ReportFormat>,
    #[clap(long = "output-path")]
    pub output_path: PathBuf,
    #[clap(long = "product-id")]
    pub product_ids: Option<Vec<ProductId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum ReportFormat {
    Html,
    Json,
    Markdown,
    Custom,
}

pub struct ReportEnvironmentVariables {}
