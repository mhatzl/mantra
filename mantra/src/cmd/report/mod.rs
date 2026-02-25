use std::{collections::HashSet, path::PathBuf, str::FromStr};

use anyhow::bail;
use mantra_schema::{
    product::ProductId,
    report::{ReportProduct, short::ShortReport},
    time::OffsetDateTime,
};
use tera::Context;

use crate::{
    cmd::report::cfg::{ReportConfig, ReportFormat},
    db::{MantraConnection, MantraDb, MantraTransaction},
};

pub mod cfg;
mod db;

pub async fn report<'db>(db: &'db MantraDb, cfg: ReportConfig) -> Result<(), anyhow::Error> {
    let mut transaction = db.start_transaction().await?;

    let product_ids = if let Some(product_ids) = cfg.args.product_ids
        && !product_ids.is_empty()
    {
        product_ids
    } else {
        // generate report over all collected products
        let product_ids = sqlx::query!(
            "
            select id
            from Products
            "
        )
        .fetch_all(transaction.as_mut())
        .await?;

        if product_ids.is_empty() {
            anyhow::bail!("No products collected to generate a report for.");
        }

        product_ids.into_iter().map(|p| p.id).collect()
    };

    let mut reports = Vec::new();
    for product_id in product_ids {
        let reporter =
            ProductReporter::new(&mut transaction, cfg.cfg_filepath.clone(), product_id).await?;
        let report = reporter.short_report().await?;
        reports.push(report);
    }

    let single_product = reports.len() == 1;
    let formats = HashSet::<ReportFormat>::from_iter(cfg.args.formats.into_iter());
    // TODO: check if formats were set twice

    for format in formats {
        let (extension, content) = match format {
            ReportFormat::Html => {
                let html_content = if single_product {
                    let template = include_str!("templates/single_product/short_report.html");
                    tera::Tera::one_off(
                        template,
                        &Context::from_serialize(
                            &reports.first().expect("Checked that one report exists"),
                        )?,
                        true,
                    )?
                } else {
                    todo!()
                };

                ("html", html_content)
            }
            ReportFormat::Json => {
                let report_schema = if single_product {
                    json5::to_string(&reports.first().expect("Checked that one report exists"))?
                } else {
                    json5::to_string(&ShortReport {
                        product_reports: reports.clone(),
                    })?
                };

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

struct ProductReporter<'t, 'db> {
    transaction: &'t mut MantraTransaction<'db>,
    cfg_filepath: PathBuf,
    abs_cfg_file_parent_path: PathBuf,
    collect_nr: i64,
    product: ReportProduct,
    reported_at_utc: OffsetDateTime,
}

impl<'t, 'db> ProductReporter<'t, 'db> {
    async fn new(
        transaction: &'t mut MantraTransaction<'db>,
        cfg_filepath: PathBuf,
        product_id: ProductId,
    ) -> Result<Self, anyhow::Error> {
        let reported_at_utc = OffsetDateTime::now_utc();

        let product_record = sqlx::query!(
            r#"
                select
                    p.last_collect_nr,
                    p.id,
                    p.name,
                    p.base,
                    p.version,
                    p.homepage,
                    p.repository,
                    p.license,
                    gt.content as "description?"
                from Products p left join GeneralTexts gt on p.description_hash = gt.hash
                where id = $1
            "#,
            product_id
        )
        .fetch_optional(transaction.as_mut())
        .await?;

        match product_record {
            Some(entry) => {
                let mut product_properties = serde_json::Map::new();

                for entry in sqlx::query!(
                    r#"
                        select p.property_key, gj.content
                        from ProductProperties p, GeneralJson gj
                        where p.product_id = $1 and gj.hash = p.value_hash
                    "#,
                    product_id
                )
                .fetch_all(transaction.as_mut())
                .await?
                {
                    product_properties.insert(
                        entry.property_key,
                        serde_json::Value::from_str(&entry.content)?,
                    );
                }

                let product_properties = if product_properties.is_empty() {
                    None
                } else {
                    Some(product_properties)
                };

                Ok(Self {
                    transaction,
                    abs_cfg_file_parent_path: crate::io::abs_parent_path(&cfg_filepath)?,
                    cfg_filepath: cfg_filepath,
                    collect_nr: entry.last_collect_nr,
                    product: ReportProduct {
                        id: entry.id,
                        base: entry.base,
                        name: entry.name,
                        version: entry.version,
                        homepage: entry.homepage,
                        repository: entry.repository,
                        license: entry.license,
                        description: entry.description,
                        properties: product_properties,
                    },
                    reported_at_utc,
                })
            }
            None => anyhow::bail!("No data collected for product '{}'", product_id),
        }
    }

    fn connection_mut(&mut self) -> &mut MantraConnection {
        self.transaction.as_mut()
    }

    fn collect_nr(&self) -> i64 {
        self.collect_nr
    }

    fn product_id(&self) -> ProductId {
        self.product.id.clone()
    }
}
