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
    #[arg(long)]
    pub formats: Vec<ReportFormat>,
    #[arg(long = "output-dir")]
    pub output_dir: PathBuf,
    /// List of product IDs that should be part of the report.
    /// If none are given, all collected products are reported.
    #[arg(long = "product-id")]
    pub product_ids: Option<Vec<ProductId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum ReportFormat {
    Html,
    Json,
    Markdown,
}

impl ReportFormat {
    pub fn as_extension(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportFormat::Html => write!(f, "html"),
            ReportFormat::Json => write!(f, "json5"),
            ReportFormat::Markdown => write!(f, "md"),
        }
    }
}

pub struct ReportEnvironmentVariables {}
