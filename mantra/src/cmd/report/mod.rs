use std::collections::HashSet;

use anyhow::bail;
use tera::Context;

use crate::{
    cmd::report::cfg::{ReportConfig, ReportFormat},
    db::MantraDb,
};

pub mod cfg;
mod db;

pub async fn report<'db>(db: &'db MantraDb, cfg: ReportConfig) -> Result<(), anyhow::Error> {
    let mut transaction = db.start_transaction().await?;

    let products_overview =
        db::products_overview(&mut transaction, &cfg.cfg_filepath, cfg.args.product_ids).await?;

    let formats = HashSet::<ReportFormat>::from_iter(cfg.args.formats.into_iter());
    // TODO: check if formats were set twice

    for format in formats {
        let (extension, content) = match format {
            ReportFormat::Html => {
                let template = include_str!("templates/multi_product/multi_product.html");
                let html_content = tera::Tera::one_off(
                    template,
                    &Context::from_serialize(&products_overview)?,
                    true,
                )?;

                ("html", html_content)
            }
            ReportFormat::Json => {
                let report_schema = json5::to_string(&products_overview)?;

                ("json5", report_schema)
            }
            ReportFormat::Markdown => todo!(),
            ReportFormat::Custom => todo!(),
        };

        let mut out_path = cfg.args.output_path.clone();
        if out_path.set_extension(extension) {
            tokio::fs::write(out_path, content).await?;
        } else {
            bail!("Given output path does not contain a filename.");
        }
    }

    Ok(())
}
