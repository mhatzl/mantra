fn main() {
    let req_schema = schemars::schema_for!(mantra_schema::requirements::RequirementSchema);
    write_schema(
        &req_schema,
        &std::path::PathBuf::from("RequirementSchema.json"),
    );

    let annotation_schema = schemars::schema_for!(mantra_schema::annotations::AnnotationSchema);
    write_schema(
        &annotation_schema,
        &std::path::PathBuf::from("AnnotationSchema.json"),
    );

    let test_run_schema = schemars::schema_for!(mantra_schema::test_runs::TestRunSchema);
    write_schema(
        &test_run_schema,
        &std::path::PathBuf::from("TestRunSchema.json"),
    );

    let review_schema = schemars::schema_for!(mantra_schema::reviews::ReviewSchema);
    write_schema(
        &review_schema,
        &std::path::PathBuf::from("ReviewSchema.json"),
    );

    let short_report_schema = schemars::schema_for!(mantra_schema::report::short::ShortReport);
    write_schema(
        &short_report_schema,
        &std::path::PathBuf::from("ShortReportSchema.json"),
    );
}

fn write_schema(schema: &schemars::schema::RootSchema, path: &std::path::Path) {
    let content = serde_json::to_string_pretty(schema).expect("Schema is serializable.");
    if let Err(err) = std::fs::write(std::path::PathBuf::from("schema-gen").join(path), content) {
        eprintln!("Failed writing schema. Cause: {}", err);
    }
}
