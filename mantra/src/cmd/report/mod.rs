use std::collections::HashSet;

use anyhow::bail;

use crate::{
    cmd::report::{
        cfg::{ReportConfig, ReportFormat},
        create::create_report,
        templates::MantraTemplates,
    },
    db::MantraDb,
};

pub mod cfg;
mod create;
mod db;
mod templates;
mod writer;

pub async fn report<'db>(db: &'db MantraDb, cfg: ReportConfig) -> Result<(), anyhow::Error> {
    // let products_overview =
    //     db::products_overview(&mut transaction, &cfg.cfg_filepath, cfg.args.product_ids).await?;

    let formats = HashSet::<ReportFormat>::from_iter(cfg.args.formats.into_iter());
    // TODO: check if formats were set twice

    let templates = MantraTemplates::new()?;

    let mut transaction = db.start_transaction().await?;

    create_report(
        &mut transaction,
        &cfg.args.output_dir,
        formats,
        templates,
        cfg.args.product_ids.as_deref(),
    )
    .await

    // for format in formats {
    //     let (extension, content) = match format {
    //         ReportFormat::Html => {
    //             let template = include_str!("templates/multi_product/multi_product.html");
    //             let html_content = tera::Tera::one_off(
    //                 template,
    //                 &Context::from_serialize(&products_overview)?,
    //                 true,
    //             )?;

    //             ("html", html_content)
    //         }
    //         ReportFormat::Json => {
    //             let report_schema = json5::to_string(&products_overview)?;

    //             ("json5", report_schema)
    //         }
    //         ReportFormat::Markdown => todo!(),
    //     };

    //     let mut out_path = cfg.args.output_dir.clone();
    //     if out_path.set_extension(extension) {
    //         tokio::fs::write(out_path, content).await?;
    //     } else {
    //         bail!("Given output path does not contain a filename.");
    //     }
    // }
}
