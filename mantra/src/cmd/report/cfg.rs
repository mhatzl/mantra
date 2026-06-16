use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use mantra_schema::{ConversionError, product::ProductId};

use crate::cfg::MantraConfigFile;

pub struct ReportConfig {
    /// Path to the mantra config file that is used to collect the data.
    cfg_filepath: PathBuf,
    /// Optional path to a directory containing custom templates.
    template_dir: Option<PathBuf>,
    formats: HashSet<ReportFormat>,
    out_dir: PathBuf,
    args: ReportArguments,
    envs: ReportEnvironmentVariables,
}

impl ReportConfig {
    pub fn new(
        cfg_filepath: PathBuf,
        cfg_file: MantraConfigFile,
        args: ReportArguments,
        envs: ReportEnvironmentVariables,
    ) -> Result<Self, anyhow::Error> {
        let mut template_dir = None;
        if args.template_dir.is_some() {
            template_dir = args.template_dir.clone();
        } else if cfg_file.reports.template_dir.is_some() {
            template_dir = cfg_file.reports.template_dir;
        };

        let args_formats = HashSet::from_iter(args.formats.iter().cloned());
        let cfg_formats_len = cfg_file.reports.formats.len();
        let cfg_formats = HashSet::from_iter(cfg_file.reports.formats.into_iter());

        if args_formats.len() != args.formats.len() {
            log::warn!(
                "Duplicate report formats in arguments. Reports are only generated once per format"
            );
        }
        if cfg_formats.len() != cfg_formats_len {
            log::warn!(
                "Duplicate report formats in configuration file. Reports are only generated once per format"
            );
        }

        let formats = if !args_formats.is_empty() {
            args_formats
        } else if !cfg_formats.is_empty() {
            cfg_formats
        } else {
            HashSet::from([ReportFormat::Html])
        };

        let out_dir = if args.output_dir.is_absolute() {
            args.output_dir.clone()
        } else {
            crate::io::abs_parent_path(&cfg_filepath)?.join(&args.output_dir)
        };

        Ok(Self {
            cfg_filepath,
            template_dir,
            formats,
            out_dir,
            args,
            envs,
        })
    }

    pub fn template_dir(&self) -> Option<&Path> {
        self.template_dir.as_deref()
    }

    pub fn formats(&self) -> &HashSet<ReportFormat> {
        &self.formats
    }

    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }

    pub fn product_ids(&self) -> Option<&[ProductId]> {
        self.args.product_ids.as_deref()
    }
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
    /// Optional path to the directory that is searched for report templates.
    /// This overwrites the optional setting in the mantra config file.
    ///
    /// **Note:** If a template is not found in this directory, the default template will be used.
    #[arg(long = "template-dir")]
    pub template_dir: Option<PathBuf>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
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

impl TryFrom<&OsStr> for ReportFormat {
    type Error = ConversionError;

    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        if value.to_str() == Some(&ReportFormat::Html.to_string()) {
            Ok(ReportFormat::Html)
        } else if value.to_str() == Some(&ReportFormat::Json.to_string()) {
            Ok(ReportFormat::Json)
        } else if value.to_str() == Some(&ReportFormat::Markdown.to_string()) {
            Ok(ReportFormat::Markdown)
        } else {
            Err(ConversionError::UnknownFormat)
        }
    }
}

pub struct ReportEnvironmentVariables {}
