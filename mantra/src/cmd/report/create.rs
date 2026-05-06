// Structure of output folder
//
// - index.html ... products schema
// - products/{checked product-id}.html ... per product schema
//   - products/{checked product-id}/
//     - evidence-matrix.html
//     - requirements.html
//     - requirements/{checked req-id}.html ... requirement schema
//     - reviews.html
//     - review/{date}_{checked name}.html
//     - test-runs.html
//     - test-run/{date}_{checked name}.html
//     - test-run/{date}_{name}/test-case/{checked name}.html
// - sources/
//   - {root path}.html ... either source file or folder schema
//   - {root path}/
//     - {sub path}.html ... source file only has an html file
//
// Note: "checked" means that the String is converted to an URL safe string
//
// Note: not using "index.html" inside folders, because users may name items as "index", which would break the structure

use std::collections::HashSet;

use mantra_schema::{
    SCHEMA_VERSION,
    product::ProductId,
    report::{
        product::{ProductMetadata, ProductReportSchema, ProductSummary},
        products::ProductsReportSchema,
    },
};

use crate::{
    cmd::report::{
        cfg::ReportFormat,
        db::schemas::{nav::generate_navigation_schema, product::generate_product_schemas},
        templates::MantraTemplates,
        writer::ReportWriter,
    },
    db::MantraTransaction,
};

pub async fn create_report<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    out_dir: &std::path::Path,
    formats: HashSet<ReportFormat>,
    templates: MantraTemplates<'templates>,
    product_ids: Option<&[ProductId]>,
) -> Result<(), anyhow::Error> {
    let products_path = out_dir.join("products");
    tokio::fs::create_dir_all(&products_path).await?;
    let sources_path = out_dir.join("sources");
    tokio::fs::create_dir_all(&sources_path).await?;

    let products = generate_product_schemas(transaction, product_ids).await?;

    let nav = generate_navigation_schema(transaction, &products).await?;

    let writer = ReportWriter::new(nav, formats, templates);

    create_sources_structure(transaction, &sources_path, &writer, &products).await?;

    let mut product_overviews = Vec::with_capacity(products.len());
    let mut products_summary = ProductSummary::default();

    for product in products {
        product_overviews.push(ProductMetadata {
            id: product.id.clone(),
            name: product.name.clone(),
            base: product.base.clone(),
            version: product.version.clone(),
            homepage: product.homepage.clone(),
            repository: product.repository.clone(),
            license: product.license.clone(),
        });

        let product_summary =
            create_product_structure(transaction, &products_path, &writer, product).await?;
        products_summary.add(&product_summary);
    }

    create_products_structure(
        out_dir,
        &writer,
        ProductsReportSchema {
            schema_version: Some(SCHEMA_VERSION.to_owned()),
            summary: products_summary,
            products: product_overviews,
        },
    )
    .await?;

    Ok(())
}

async fn create_products_structure<'db, 'templates>(
    out_dir: &std::path::Path,
    writer: &ReportWriter<'templates>,
    schema: ProductsReportSchema,
) -> Result<(), anyhow::Error> {
    let filepath = out_dir.join("index.html");

    writer
        .write_file(&filepath, schema, super::templates::TemplateName::Products)
        .await
}

async fn create_sources_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    out_dir: &std::path::Path,
    writer: &ReportWriter<'templates>,
    product_ids: &[ProductReportSchema],
) -> Result<(), anyhow::Error> {
    // TODO: create sources

    Ok(())
}

async fn create_product_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    out_dir: &std::path::Path,
    writer: &ReportWriter<'templates>,
    product: ProductReportSchema,
) -> Result<ProductSummary, anyhow::Error> {
    // TODO

    Ok(ProductSummary::default())
}
