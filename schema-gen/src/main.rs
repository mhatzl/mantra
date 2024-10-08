fn main() {
    let req_schema = schemars::schema_for!(mantra_schema::requirements::RequirementSchema);
    write_schema(
        &req_schema,
        &std::path::PathBuf::from("RequirementSchema.json"),
    );

    let trace_schema = schemars::schema_for!(mantra_schema::traces::TraceSchema);
    write_schema(&trace_schema, &std::path::PathBuf::from("TraceSchema.json"));

    let coverage_schema = schemars::schema_for!(mantra_schema::coverage::CoverageSchema);
    write_schema(
        &coverage_schema,
        &std::path::PathBuf::from("CoverageSchema.json"),
    );

    let review_schema = schemars::schema_for!(mantra_schema::reviews::ReviewSchema);
    write_schema(
        &review_schema,
        &std::path::PathBuf::from("ReviewSchema.json"),
    );

    let report_schema = schemars::schema_for!(mantra::cmd::report::ReportContext);
    write_schema(
        &report_schema,
        &std::path::PathBuf::from("ReportContext.json"),
    );
}

fn write_schema(schema: &schemars::schema::RootSchema, path: &std::path::Path) {
    let content = serde_json::to_string_pretty(schema).expect("Schema is serializable.");
    if let Err(err) = std::fs::write(path, content) {
        eprintln!("Failed writing schema. Cause: {}", err);
    }
}
