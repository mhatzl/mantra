use std::path::Path;

use mantra_schema::{
    product::ProductId,
    report::overview::{ProductsOverviewReport, ProductsSummary},
};

use crate::{cmd::report::db::product_overview::ProductReporter, db::MantraTransaction};

pub mod product_overview;

pub async fn products_overview<'t, 'db>(
    transaction: &'t mut MantraTransaction<'db>,
    cfg_filepath: &Path,
    product_ids: Option<Vec<ProductId>>,
) -> Result<ProductsOverviewReport, anyhow::Error> {
    let product_ids = if let Some(product_ids) = product_ids
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

    let mut products_overview = ProductsOverviewReport {
        product_reports: Vec::with_capacity(product_ids.len()),
        summary: ProductsSummary::default(),
    };
    for product_id in product_ids {
        let reporter =
            ProductReporter::new(transaction, cfg_filepath.to_path_buf(), product_id).await?;
        let report = reporter.product_overview().await?;

        products_overview
            .summary
            .requirements
            .add(&report.requirements.summary);
        products_overview
            .summary
            .test_cases
            .add(&report.test_runs.test_cases_summary);

        products_overview.product_reports.push(report);
    }

    Ok(products_overview)
}
