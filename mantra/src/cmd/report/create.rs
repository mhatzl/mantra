use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use mantra_schema::{
    SCHEMA_VERSION,
    product::ProductId,
    report::{
        product::{ProductMetadata, ProductReportSchema, ProductSummary},
        products::ProductsReportSchema,
        requirements::RequirementsSummary,
        reviews::ReviewsSummary,
        tests::TestsSummary,
    },
    requirements::ReqId,
};

use crate::{
    cmd::report::{
        cfg::ReportFormat,
        db::schemas::{
            evidence_matrix::generate_evidence_matrix_schema, nav::generate_navigation_schema,
            product::generate_product_schemas, requirement::generate_requirement_schema,
            requirements::generate_requirements_schema,
        },
        templates::MantraTemplates,
        writer::ReportWriter,
    },
    db::MantraTransaction,
};

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
// Note: "checked" means that the String is encoded to an URL safe string
//
// Note: not using "index.html" inside folders, because users may name items as "index", which would break the structure

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
    let filepath = out_dir.join("index");

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
    mut product: ProductReportSchema,
) -> Result<ProductSummary, anyhow::Error> {
    let encoded_product_id = urlencoding::encode(&product.id);
    let product_path = out_dir.join(encoded_product_id.to_string());

    tokio::fs::create_dir(&product_path).await?;

    let evidence_matrix_schema = generate_evidence_matrix_schema(transaction, &product).await?;
    let evidence_matrix_filepath = product_path.join("evidence-matrix");
    writer
        .write_file(
            &evidence_matrix_filepath,
            evidence_matrix_schema,
            super::templates::TemplateName::EvidenceMatrix,
        )
        .await?;

    let requirements_summary =
        create_requirements_structure(transaction, &product_path, writer, &product).await?;
    let reviews_summary =
        create_reviews_structure(transaction, &product_path, writer, &product).await?;
    let tests_summary =
        create_tests_structure(transaction, &product_path, writer, &product).await?;

    let product_summary = ProductSummary {
        requirements: requirements_summary,
        test_runs: tests_summary.test_runs,
        test_cases: tests_summary.test_cases,
        reviews: reviews_summary,
    };
    product.summary = product_summary;

    writer
        .write_file(
            &product_path,
            product,
            super::templates::TemplateName::Product,
        )
        .await?;

    Ok(product_summary)
}

async fn create_requirements_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    out_dir: &std::path::Path,
    writer: &ReportWriter<'templates>,
    product: &ProductReportSchema,
) -> Result<RequirementsSummary, anyhow::Error> {
    let requirements_path = out_dir.join("requirements");

    tokio::fs::create_dir(&requirements_path).await?;

    let requirements_schema = generate_requirements_schema(transaction, &product).await?;
    let requirements_summary = requirements_schema.summary;

    // Note: The requirements schema splits all requirements into their states.
    // Not ideal to iterate over all requirements this way,
    // but better than a new DB query or duplicating requirements in the requirements schema.
    for req in requirements_schema
        .failed
        .iter()
        .chain(requirements_schema.skipped.iter())
        .chain(requirements_schema.unverified.iter())
        .chain(requirements_schema.verified.iter())
        .chain(requirements_schema.ignored.iter())
        .chain(requirements_schema.deprecated.iter())
    {
        let req_path = prepare_requirement_path(&requirements_path, &req.id).await?;

        let requirement_schema =
            generate_requirement_schema(transaction, &product, &req.id).await?;
        writer
            .write_file(
                &req_path,
                requirement_schema,
                super::templates::TemplateName::Requirement,
            )
            .await?;
    }

    writer
        .write_file(
            &requirements_path,
            requirements_schema,
            super::templates::TemplateName::Requirements,
        )
        .await?;

    Ok(requirements_summary)
}

async fn prepare_requirement_path(base_path: &Path, id: &ReqId) -> Result<PathBuf, anyhow::Error> {
    if id.contains('.') {
        let mut req_path = base_path.to_path_buf();
        let id_parts: Vec<_> = id.split('.').collect();

        let mut parent_parts = id_parts.clone();
        parent_parts.truncate(id_parts.len() - 1);

        for part in parent_parts {
            req_path = req_path.join(urlencoding::encode(&part).to_string());

            if !tokio::fs::try_exists(&req_path).await.unwrap_or(false) {
                tokio::fs::create_dir(&req_path).await?;
            }
        }

        req_path = req_path.join(
            urlencoding::encode(
                &id_parts
                    .last()
                    .expect("Checked that ID contains '.', so at least one ID part exists"),
            )
            .to_string(),
        );

        Ok(req_path)
    } else {
        Ok(base_path.join(urlencoding::encode(id).to_string()))
    }
}

async fn create_reviews_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    out_dir: &std::path::Path,
    writer: &ReportWriter<'templates>,
    product: &ProductReportSchema,
) -> Result<ReviewsSummary, anyhow::Error> {
    // TODO

    Ok(ReviewsSummary::default())
}

async fn create_tests_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    out_dir: &std::path::Path,
    writer: &ReportWriter<'templates>,
    product: &ProductReportSchema,
) -> Result<TestSummary, anyhow::Error> {
    // TODO

    Ok(TestSummary {
        test_runs: TestsSummary::default(),
        test_cases: TestsSummary::default(),
    })
}

struct TestSummary {
    test_runs: TestsSummary,
    test_cases: TestsSummary,
}
