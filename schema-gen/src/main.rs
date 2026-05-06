fn main() {
    let base_path = std::path::PathBuf::from(
        &std::env::var("CARGO_MANIFEST_DIR").unwrap_or("schema-gen".to_owned()),
    )
    .join("generated");

    let _ = std::fs::remove_dir_all(&base_path); // clear if exists
    std::fs::create_dir(&base_path)
        .expect("Failed to create the /generated folder to store schemas");

    write_collect_schemas(&base_path);
    write_report_schemas(&base_path);
}

fn write_collect_schemas(base_path: &std::path::Path) {
    let collect_path = base_path.join("collect");
    std::fs::create_dir(&collect_path)
        .expect("Failed to create the /collect folder to store schemas");

    let req_schema = schemars::schema_for!(mantra_schema::requirements::RequirementSchema);
    write_schema(&req_schema, &collect_path.join("RequirementSchema.json"));

    let annotation_schema = schemars::schema_for!(mantra_schema::annotations::AnnotationSchema);
    write_schema(
        &annotation_schema,
        &collect_path.join("AnnotationSchema.json"),
    );

    let test_run_schema = schemars::schema_for!(mantra_schema::test_runs::TestRunSchema);
    write_schema(&test_run_schema, &collect_path.join("TestRunSchema.json"));

    let review_schema = schemars::schema_for!(mantra_schema::reviews::ReviewSchema);
    write_schema(&review_schema, &collect_path.join("ReviewSchema.json"));
}

fn write_report_schemas(base_path: &std::path::Path) {
    let report_path = base_path.join("report");
    std::fs::create_dir(&report_path)
        .expect("Failed to create the /report folder to store schemas");

    let evidence_matrix_schema =
        schemars::schema_for!(mantra_schema::report::evidence_matrix::EvidenceMatrixSchema);
    write_schema(
        &evidence_matrix_schema,
        &report_path.join("EvidenceMatrixSchema.json"),
    );
    let navigation_schema =
        schemars::schema_for!(mantra_schema::report::nav::ReportNavigationSchema);
    write_schema(
        &navigation_schema,
        &report_path.join("ReportNavigationSchema.json"),
    );
    let product_schema = schemars::schema_for!(mantra_schema::report::product::ProductReportSchema);
    write_schema(
        &product_schema,
        &report_path.join("ProductReportSchema.json"),
    );
    let products_schema =
        schemars::schema_for!(mantra_schema::report::products::ProductsReportSchema);
    write_schema(
        &products_schema,
        &report_path.join("ProductsReportSchema.json"),
    );
    let requirement_schema =
        schemars::schema_for!(mantra_schema::report::requirement::RequirementReportSchema);
    write_schema(
        &requirement_schema,
        &report_path.join("RequirementReportSchema.json"),
    );
    let requirements_schema =
        schemars::schema_for!(mantra_schema::report::requirements::RequirementsReportSchema);
    write_schema(
        &requirements_schema,
        &report_path.join("RequirementsReportSchema.json"),
    );
    let review_schema = schemars::schema_for!(mantra_schema::report::review::ReviewReportSchema);
    write_schema(&review_schema, &report_path.join("ReviewReportSchema.json"));
    let reviews_schema = schemars::schema_for!(mantra_schema::report::reviews::ReviewsReportSchema);
    write_schema(
        &reviews_schema,
        &report_path.join("ReviewsReportSchema.json"),
    );
    let source_file_schema =
        schemars::schema_for!(mantra_schema::report::source_file::SourceFileReportSchema);
    write_schema(
        &source_file_schema,
        &report_path.join("SourceFileReportSchema.json"),
    );
    let source_folder_schema =
        schemars::schema_for!(mantra_schema::report::source_folder::SourceFolderReportSchema);
    write_schema(
        &source_folder_schema,
        &report_path.join("SourceFolderReportSchema.json"),
    );
    let test_case_schema =
        schemars::schema_for!(mantra_schema::report::test_case::TestCaseReportSchema);
    write_schema(
        &test_case_schema,
        &report_path.join("TestCaseReportSchema.json"),
    );
    let test_run_schema =
        schemars::schema_for!(mantra_schema::report::test_run::TestRunReportSchema);
    write_schema(
        &test_run_schema,
        &report_path.join("TestRunReportSchema.json"),
    );
    let test_runs_schema =
        schemars::schema_for!(mantra_schema::report::test_runs::TestRunsReportSchema);
    write_schema(
        &test_runs_schema,
        &report_path.join("TestRunsReportSchema.json"),
    );
}

fn write_schema(schema: &schemars::schema::RootSchema, path: &std::path::Path) {
    let content = serde_json::to_string_pretty(schema).expect("Schema is serializable.");
    if let Err(err) = std::fs::write(path, content) {
        eprintln!("Failed writing schema. Cause: {}", err);
    }
}
