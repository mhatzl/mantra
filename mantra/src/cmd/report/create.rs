use mantra_schema::{
    PRODUCTS_FOLDER_NAME, REQUIREMENTS_FOLDER_NAME, REVIEWS_FOLDER_NAME, SCHEMA_VERSION,
    SOURCES_FOLDER_NAME, TEST_RUNS_FOLDER_NAME,
    report::{
        product::{ProductMetadata, ProductReportSchema, ProductSummary},
        products::ProductsReportSchema,
        requirements::RequirementsSummary,
        reviews::ReviewsSummary,
        test_run::{TestCasesOverview, TestRunReference},
        tests::TestsSummary,
    },
};

use crate::{
    cmd::report::{
        cfg::ReportConfig,
        db::schemas::{
            evidence_matrix::generate_evidence_matrix_schema, nav::generate_navigation_schema,
            product::generate_product_schemas, requirement::generate_requirement_schema,
            requirements::generate_requirements_schema, review::generate_review_schema,
            reviews::generate_reviews_schema, test_case::generate_test_case_schema,
            test_run::generate_test_run_schema, test_runs::generate_test_runs_schema,
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
    cfg: ReportConfig,
    templates: MantraTemplates<'templates>,
) -> Result<(), anyhow::Error> {
    let out_dir = cfg.out_dir();
    let products_path = out_dir.join(PRODUCTS_FOLDER_NAME);
    tokio::fs::create_dir_all(&products_path).await?;
    let sources_path = out_dir.join(SOURCES_FOLDER_NAME);
    tokio::fs::create_dir_all(&sources_path).await?;

    let products = generate_product_schemas(transaction, cfg.product_ids()).await?;

    let nav = generate_navigation_schema(transaction, &products).await?;

    let writer = ReportWriter::new(out_dir.to_path_buf(), nav, cfg, templates);

    create_sources_structure(transaction, &writer, &products).await?;

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

        let product_summary = create_product_structure(transaction, &writer, product).await?;
        products_summary.add(&product_summary);
    }

    create_products_structure(
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
    writer: &ReportWriter<'templates>,
    schema: ProductsReportSchema,
) -> Result<(), anyhow::Error> {
    let filepath = writer.base_path().join("index");

    writer
        .write_file(&filepath, schema, super::templates::TemplateName::Products)
        .await
}

async fn create_sources_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    writer: &ReportWriter<'templates>,
    products: &[ProductReportSchema],
) -> Result<(), anyhow::Error> {
    // TODO: create sources

    Ok(())
}

async fn create_product_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    writer: &ReportWriter<'templates>,
    mut product: ProductReportSchema,
) -> Result<ProductSummary, anyhow::Error> {
    let product_path = product.id.os_path().to_path(writer.base_path());

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

    let requirements_summary = create_requirements_structure(transaction, writer, &product).await?;
    let reviews_summary = create_reviews_structure(transaction, writer, &product).await?;
    let test_cases_summary = create_tests_structure(transaction, writer, &product).await?;

    let product_summary = ProductSummary {
        requirements: requirements_summary,
        test_cases: test_cases_summary,
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
    writer: &ReportWriter<'templates>,
    product: &ProductReportSchema,
) -> Result<RequirementsSummary, anyhow::Error> {
    let requirements_path = product
        .id
        .os_path()
        .join(REQUIREMENTS_FOLDER_NAME)
        .to_path(writer.base_path());

    tokio::fs::create_dir(&requirements_path).await?;

    let requirements_schema = generate_requirements_schema(transaction, product).await?;
    let requirements_summary = requirements_schema.requirements.summary;

    // Note: The requirements schema splits all requirements into their states.
    // Not ideal to iterate over all requirements this way,
    // but better than a new DB query or duplicating requirements in the requirements schema.
    for req in requirements_schema
        .requirements
        .failed
        .iter()
        .chain(requirements_schema.requirements.skipped.iter())
        .chain(requirements_schema.requirements.unverified.iter())
        .chain(requirements_schema.requirements.verified.iter())
        .chain(requirements_schema.requirements.ignored.iter())
        .chain(requirements_schema.requirements.deprecated.iter())
    {
        let req_path = req.os_path().to_path(writer.base_path());
        if let Some(parent_path) = req_path.parent() {
            tokio::fs::create_dir_all(parent_path).await?;
        }

        let requirement_schema = generate_requirement_schema(transaction, product, &req.id).await?;
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

async fn create_reviews_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    writer: &ReportWriter<'templates>,
    product: &ProductReportSchema,
) -> Result<ReviewsSummary, anyhow::Error> {
    let reviews_path = product
        .id
        .os_path()
        .join(REVIEWS_FOLDER_NAME)
        .to_path(writer.base_path());

    tokio::fs::create_dir(&reviews_path).await?;

    let reviews_schema = generate_reviews_schema(transaction, product).await?;
    let reviews_summary = reviews_schema.reviews.summary;

    for review in reviews_schema
        .reviews
        .valid
        .iter()
        .chain(reviews_schema.reviews.obsolete.iter())
    {
        let review_path = review.os_path().to_path(writer.base_path());

        let review_schema = generate_review_schema(transaction, product, review).await?;
        writer
            .write_file(
                &review_path,
                review_schema,
                super::templates::TemplateName::Review,
            )
            .await?;
    }

    writer
        .write_file(
            &reviews_path,
            reviews_schema,
            super::templates::TemplateName::Reviews,
        )
        .await?;

    Ok(reviews_summary)
}

async fn create_tests_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    writer: &ReportWriter<'templates>,
    product: &ProductReportSchema,
) -> Result<TestsSummary, anyhow::Error> {
    let test_runs_path = product
        .id
        .os_path()
        .join(TEST_RUNS_FOLDER_NAME)
        .to_path(writer.base_path());

    tokio::fs::create_dir(&test_runs_path).await?;

    let test_runs_schema = generate_test_runs_schema(transaction, product).await?;
    let test_runs = &test_runs_schema.test_runs;
    let test_cases_summary = test_runs_schema.test_cases_summary;

    for test_run in test_runs
        .failed
        .iter()
        .chain(test_runs.skipped.iter())
        .chain(test_runs.unknown.iter())
        .chain(test_runs.obsolete.iter())
        .chain(test_runs.passed.iter())
    {
        let test_run_path = test_run.os_path().to_path(writer.base_path());

        let test_run_schema = generate_test_run_schema(transaction, product, test_run).await?;

        if let Some(test_cases) = &test_run_schema.test_cases {
            create_test_cases_structure(transaction, writer, product, test_run, test_cases).await?;
        }

        writer
            .write_file(
                &test_run_path,
                test_run_schema,
                super::templates::TemplateName::TestRun,
            )
            .await?;
    }

    writer
        .write_file(
            &test_runs_path,
            test_runs_schema,
            super::templates::TemplateName::TestRuns,
        )
        .await?;

    Ok(test_cases_summary)
}

async fn create_test_cases_structure<'db, 'templates>(
    transaction: &mut MantraTransaction<'db>,
    writer: &ReportWriter<'templates>,
    product: &ProductReportSchema,
    test_run: &TestRunReference,
    test_cases: &TestCasesOverview,
) -> Result<(), anyhow::Error> {
    for test_case in test_cases
        .failed
        .iter()
        .chain(test_cases.skipped.iter())
        .chain(test_cases.unknown.iter())
        .chain(test_cases.obsolete.iter())
        .chain(test_cases.passed.iter())
    {
        let test_case_path = test_case.os_path().to_path(writer.base_path());
        if let Some(parent_path) = test_case_path.parent() {
            tokio::fs::create_dir_all(parent_path).await?;
        }

        let test_case_schema =
            generate_test_case_schema(transaction, product, test_run, test_case).await?;

        writer
            .write_file(
                &test_case_path,
                test_case_schema,
                super::templates::TemplateName::TestCase,
            )
            .await?;
    }

    Ok(())
}
