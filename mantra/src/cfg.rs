use std::path::PathBuf;

use crate::{
    cmd::Cmd,
    db::{self},
};

#[derive(clap::Parser)]
pub struct Config {
    #[command(flatten)]
    pub db: db::Config,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Clone, clap::Args)]
pub struct CollectConfig {
    #[arg(default_value = "mantra.toml")]
    pub filepath: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollectFile {
    pub requirements: crate::cmd::requirements::Format,
    pub traces: crate::cmd::trace::TraceKind,
    pub coverage: Option<crate::cmd::coverage::Config>,
    pub reviews: Option<crate::cmd::review::ReviewConfig>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteOldConfig {
    /// Delete test runs and reviews that have no linked requirement or coverage remaining.
    #[arg(long)]
    pub clean: bool,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteReqsConfig {
    #[arg(long)]
    pub ids: Option<Vec<String>>,
    /// Delete requirements before the set generation.
    #[arg(long)]
    pub before: Option<i64>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteTracesConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
    /// Delete traces before the set generation.
    #[arg(long)]
    pub before: Option<i64>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteTestRunsConfig {
    #[arg(long, alias = "older-than")]
    pub before: Option<String>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteCoverageConfig {
    #[arg(long, alias = "id")]
    pub req_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeleteReviewsConfig {
    #[arg(long, alias = "older-than")]
    pub before: Option<String>,
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    fn collect_file_syntax() {
        let content = r#"
                            [requirements.from-wiki]
                            root = "reqs.md"
                            link = "cloud-repo.something"

                            [traces.from-source]
                            root = "./"

                            [coverage]
                            data = ["coverage.json"]

                            [reviews]
                            files = ["first_review.toml"]
                            "#;

        let file: crate::cfg::CollectFile = toml::from_str(content).unwrap();

        assert_eq!(
            file.coverage.unwrap().data.first().unwrap(),
            &PathBuf::from("coverage.json"),
            "Coverage info not correctly extracted."
        );
    }
}
