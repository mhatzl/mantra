use crate::{
    cmd::report::{cfg::ReportConfig, create::create_report, templates::MantraTemplates},
    db::MantraDb,
};

pub mod cfg;
mod create;
mod db;
mod templates;
mod writer;

pub async fn report(db: &MantraDb, cfg: ReportConfig) -> Result<(), anyhow::Error> {
    let mut templates = MantraTemplates::new()?;

    if let Some(template_dir) = &cfg.template_dir() {
        templates.custom_templates(template_dir).await?;
    }

    let mut transaction = db.start_transaction().await?;

    create_report(&mut transaction, cfg, templates).await
}
