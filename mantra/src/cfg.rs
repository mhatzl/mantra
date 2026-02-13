use mantra_schema::product::Product;

use crate::cmd::collect::cfg::{
    CollectAnnotationsConfig, CollectRequirementsConfig, CollectReviewsConfig,
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
    #[serde(default, alias = "test-run")]
    pub test_runs: Vec<CollectTestRunsConfig>,
    #[serde(default, alias = "review")]
    pub reviews: Vec<CollectReviewsConfig>,
}

// use std::path::PathBuf;

// use crate::{cmd::Cmd, db};

// #[derive(clap::Parser)]
// pub struct Config {
//     #[command(flatten)]
//     pub db: db::Config,

//     #[command(subcommand)]
//     pub cmd: Cmd,
// }

// #[derive(Debug, Clone, clap::Args)]
// pub struct MantraConfigPath {
//     #[arg(default_value = "mantra.toml")]
//     pub filepath: PathBuf,
// }

// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
// pub struct MantraConfigFile {
//     #[serde(default)]
//     pub requirements: Vec<crate::cmd::requirements_old::Format>,
//     #[serde(default)]
//     pub traces: Vec<crate::cmd::trace_old::TraceKind>,
//     pub coverage: Option<crate::cmd::coverage_old::Config>,
//     pub review: Option<crate::cmd::review_old::ReviewConfig>,
//     #[serde(default, skip_serializing_if = "crate::cfg::Project::is_none")]
//     pub project: Project,
//     #[serde(
//         alias = "report-template",
//         default,
//         skip_serializing_if = "crate::cmd::report::ReportTemplate::is_none"
//     )]
//     pub report_template: crate::cmd::report_old::ReportTemplate,
// }

// #[derive(
//     Default,
//     Debug,
//     Clone,
//     PartialEq,
//     serde::Serialize,
//     serde::Deserialize,
//     clap::Args,
//     schemars::JsonSchema,
// )]
// pub struct Project {
//     #[arg(id = "project-name", long = "project-name")]
//     pub name: Option<String>,
//     #[arg(id = "project-version", long = "project-version")]
//     pub version: Option<String>,
//     #[arg(id = "project-repository", long = "project-repository")]
//     pub repository: Option<String>,
//     #[arg(id = "project-homepage", long = "project-homepage")]
//     pub homepage: Option<String>,
// }

// impl Project {
//     pub(crate) fn is_none(&self) -> bool {
//         self.name.is_none()
//             && self.version.is_none()
//             && self.repository.is_none()
//             && self.homepage.is_none()
//     }
// }

// #[derive(Debug, Clone, clap::Args)]
// pub struct DeleteOldConfig {
//     /// Delete test runs and reviews that have no linked requirement or coverage remaining.
//     #[arg(long)]
//     pub clean: bool,
// }

// #[derive(Debug, Clone, clap::Args)]
// pub struct DeleteReqsConfig {
//     #[arg(long)]
//     pub ids: Option<Vec<String>>,
//     /// Delete requirements before the set generation.
//     #[arg(long)]
//     pub before: Option<i64>,
// }

// #[derive(Debug, Clone, clap::Args)]
// pub struct DeleteTracesConfig {
//     #[arg(long, alias = "id")]
//     pub req_ids: Option<Vec<String>>,
//     /// Delete traces before the set generation.
//     #[arg(long)]
//     pub before: Option<i64>,
// }

// #[derive(Debug, Clone, clap::Args)]
// pub struct DeleteTestRunsConfig {
//     #[arg(long, alias = "older-than")]
//     pub before: Option<String>,
// }

// #[derive(Debug, Clone, clap::Args)]
// pub struct DeleteCoverageConfig {
//     #[arg(long, alias = "id")]
//     pub req_ids: Option<Vec<String>>,
// }

// #[derive(Debug, Clone, clap::Args)]
// pub struct DeleteReviewsConfig {
//     #[arg(long, alias = "older-than")]
//     pub before: Option<String>,
// }

// #[cfg(test)]
// mod test {
//     use std::path::PathBuf;

//     #[test]
//     fn collect_file_syntax() {
//         let content = r#"
//                             [project]
//                             name = "test-proj"
//                             version = "0.1.0"
//                             repository = "some.link"
//                             homepage = "some-other.link"

//                             [report-template]
//                             base = "base-template.html"
//                             req-data = "req-template.html"
//                             test-run-data = "test-run-template.html"

//                             [[requirements]]
//                             root = "reqs.md"
//                             origin = "cloud-repo.something"

//                             [[requirements]]
//                             files = ["extern-reqs.json"]

//                             [[traces]]
//                             root = ""

//                             [[traces]]
//                             files = ["extern-traces.json"]

//                             [coverage]
//                             files = ["coverage.json"]

//                             [review]
//                             files = ["first_review.toml"]
//                             "#;

//         let file: crate::cfg::MantraConfigFile = toml::from_str(content).unwrap();

//         assert_eq!(
//             file.coverage.unwrap().files.first().unwrap(),
//             &PathBuf::from("coverage.json"),
//             "Coverage info not correctly extracted."
//         );
//     }
// }
