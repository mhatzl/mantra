use mantra_schema::{
    SCHEMA_VERSION,
    report::{
        nav::{ProductNavigation, ReportNavigationSchema},
        product::ProductReportSchema,
    },
};

use crate::{
    cmd::report::db::schemas::product::{
        product_reviews, product_root_requirements, product_root_test_runs,
    },
    db::MantraTransaction,
};

pub async fn generate_navigation_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    products: &[ProductReportSchema],
) -> Result<ReportNavigationSchema, anyhow::Error> {
    let mut product_nav = Vec::with_capacity(products.len());

    for product in products {
        product_nav.push(generate_product_navigation(transaction, product).await?);
    }

    Ok(ReportNavigationSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        products: product_nav,
        root_sources: Vec::new(), // TODO
    })
}

async fn generate_product_navigation<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<ProductNavigation, anyhow::Error> {
    let root_requirements = product_root_requirements(transaction, &product.id).await?;
    let reviews = product_reviews(transaction, &product.id).await?;

    let root_test_runs = product_root_test_runs(transaction, &product.id).await?;

    Ok(ProductNavigation {
        product: product.metadata(),
        root_requirements,
        root_test_runs,
        reviews,
    })
}
