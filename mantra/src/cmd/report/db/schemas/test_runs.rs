use mantra_schema::{
    SCHEMA_VERSION,
    report::{
        product::ProductReportSchema,
        test_run::TestRunReference,
        test_runs::{TestRunsOverview, TestRunsReportSchema},
        tests::{TestState, TestsSummary},
    },
};

use crate::db::MantraTransaction;

pub async fn generate_test_runs_schema<'db>(
    transaction: &mut MantraTransaction<'db>,
    product: &ProductReportSchema,
) -> Result<TestRunsReportSchema, anyhow::Error> {
    let test_run_records = sqlx::query!(
        r#"
        select test_run_name, test_run_date, state as "state!:i64"
        from TestRunStates
        where product_id = $1
        "#,
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut test_runs_summary = TestsSummary {
        total: test_run_records.len() as i64,
        ..Default::default()
    };

    let mut passed = Vec::new();
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    let mut unknown = Vec::new();
    let mut obsolete = Vec::new();

    for test_run in test_run_records {
        let test_run_reference = TestRunReference {
            product_id: product.id.clone(),
            name: test_run.test_run_name,
            utc_date: mantra_schema::test_runs::test_date_from_str(&test_run.test_run_date)?,
            state: test_run.state.try_into()?,
        };

        match test_run_reference.state {
            mantra_schema::report::tests::TestState::Failed => failed.push(test_run_reference),
            mantra_schema::report::tests::TestState::Passed => passed.push(test_run_reference),
            mantra_schema::report::tests::TestState::Skipped => skipped.push(test_run_reference),
            mantra_schema::report::tests::TestState::Unknown => unknown.push(test_run_reference),
            mantra_schema::report::tests::TestState::Obsolete => obsolete.push(test_run_reference),
        }
    }

    test_runs_summary.failed.cnt = failed.len() as i64;
    test_runs_summary.passed.cnt = passed.len() as i64;
    test_runs_summary.skipped.cnt = skipped.len() as i64;
    test_runs_summary.unknown.cnt = unknown.len() as i64;
    test_runs_summary.obsolete.cnt = obsolete.len() as i64;

    test_runs_summary.update_percentages();

    let test_cases_states = sqlx::query!(
        r#"
        select state, count(test_run_name) as "cnt!:i64"
        from ResolvedTestCaseStates
        where product_id = $1
        group by state
        "#,
        product.id
    )
    .fetch_all(transaction.as_mut())
    .await?;

    let mut test_cases_summary = TestsSummary::default();

    for state_record in test_cases_states {
        let state = TestState::try_from(state_record.state)?;

        match state {
            TestState::Failed => test_cases_summary.failed.cnt += state_record.cnt,
            TestState::Passed => test_cases_summary.passed.cnt += state_record.cnt,
            TestState::Skipped => test_cases_summary.skipped.cnt += state_record.cnt,
            TestState::Unknown => test_cases_summary.unknown.cnt += state_record.cnt,
            TestState::Obsolete => test_cases_summary.obsolete.cnt += state_record.cnt,
        }
    }

    test_cases_summary.total = test_cases_summary.failed.cnt
        + test_cases_summary.passed.cnt
        + test_cases_summary.skipped.cnt
        + test_cases_summary.unknown.cnt
        + test_cases_summary.obsolete.cnt;

    test_cases_summary.update_percentages();

    Ok(TestRunsReportSchema {
        schema_version: Some(SCHEMA_VERSION.to_owned()),
        product: product.metadata(),
        test_cases_summary,
        test_runs: TestRunsOverview {
            summary: test_runs_summary,
            passed,
            failed,
            skipped,
            unknown,
            obsolete,
        },
    })
}
