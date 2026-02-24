use crate::cmd::report::cfg::ReportConfig;

pub mod cfg;
mod db;

pub async fn report(cfg: ReportConfig) -> Result<(), anyhow::Error> {
    todo!()
}
