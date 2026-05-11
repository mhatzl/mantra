use mantra_schema::{
    SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        test_case::{TestCaseReference, TestCaseReportSchema},
        test_run::TestRunReference,
    },
};

use crate::db::MantraTransaction;

pub async fn generate_test_case_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
    test_run: &TestRunReference,
    test_case: &TestCaseReference,
) -> Result<TestCaseReportSchema, anyhow::Error> {
    // TODO

    Ok(TestCaseReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        test_run: test_run.clone(),
        name: test_case.test_case_name.clone(),
        description: None,
        state: test_case.state,
        state_properties: None,
        location: None,
        utc_date: None,
        duration_sec: None,
        properties: None,
        logs: None,
        coverage: None,
        related_reqs: None,
    })
}
