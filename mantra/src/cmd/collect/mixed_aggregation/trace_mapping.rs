use anyhow::Context;
use mantra_schema::report::tests::TestState;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_trace_mappings(&mut self) -> Result<(), anyhow::Error> {
        self.update_traces_covered_by_failed_tests()
            .await
            .context("Failed to update traces covered by failing tests")?;

        self.update_traces_covered_by_unknown_tests()
            .await
            .context("Failed to update traces covered by unknown tests")?;

        self.update_traces_covered_by_skipped_tests()
            .await
            .context("Updating traces covered by skipped tests")?;

        self.update_traces_covered_by_passed_tests()
            .await
            .context("Failed to update traces covered by passed tests")?;

        Ok(())
    }

    async fn update_traces_covered_by_failed_tests(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from TracesCoveredByFailedTests
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing TracesCoveredByFailedTests")?;

        let failed = TestState::Failed.as_number();

        sqlx::query!(
            "
            insert into TracesCoveredByFailedTests (
                product_id,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                tlt.product_id,
                tlt.filepath,
                tlt.file_hash,
                tlt.traced_line
            from TracedLineCoveredByTests tlt
            where exists (
                select rlts.test_state
                from ResolvedLineCoverageTestStates rlts
                where rlts.product_id = tlt.product_id
                and rlts.cov_filepath = tlt.filepath
                and (
                    rlts.cov_file_hash is null
                    or rlts.cov_file_hash = tlt.file_hash
                )
                and rlts.cov_line = tlt.cov_line
                and rlts.test_state = $1
            )
            ",
            failed
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_covered_by_unknown_tests(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from TracesCoveredByUnknownTests
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing TracesCoveredByUnknownTests")?;

        let unknown = TestState::Unknown.as_number();

        sqlx::query!(
            "
            insert into TracesCoveredByUnknownTests (
                product_id,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                tlt.product_id,
                tlt.filepath,
                tlt.file_hash,
                tlt.traced_line
            from TracedLineCoveredByTests tlt
            where exists (
                select rlts.test_state
                from ResolvedLineCoverageTestStates rlts
                where rlts.product_id = tlt.product_id
                and rlts.cov_filepath = tlt.filepath
                and (
                    rlts.cov_file_hash is null
                    or rlts.cov_file_hash = tlt.file_hash
                )
                and rlts.cov_line = tlt.cov_line
                and rlts.test_state = $1
            )
            ",
            unknown
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_covered_by_skipped_tests(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from TracesCoveredBySkippedTests
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing TracesCoveredBySkippedTests")?;

        let skipped = TestState::Skipped.as_number();

        sqlx::query!(
            "
            insert into TracesCoveredBySkippedTests (
                product_id,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                tlt.product_id,
                tlt.filepath,
                tlt.file_hash,
                tlt.traced_line
            from TracedLineCoveredByTests tlt
            where exists (
                select rlts.test_state
                from ResolvedLineCoverageTestStates rlts
                where rlts.product_id = tlt.product_id
                and rlts.cov_filepath = tlt.filepath
                and (
                    rlts.cov_file_hash is null
                    or rlts.cov_file_hash = tlt.file_hash
                )
                and rlts.cov_line = tlt.cov_line
                and rlts.test_state = $1
            )
            ",
            skipped
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_covered_by_passed_tests(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from TracesCoveredByPassedTests
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing TracesCoveredByPassedTests")?;

        let passed = TestState::Passed.as_number();

        sqlx::query!(
            "
            insert into TracesCoveredByPassedTests (
                product_id,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                tlt.product_id,
                tlt.filepath,
                tlt.file_hash,
                tlt.traced_line
            from TracedLineCoveredByTests tlt
            where exists (
                select rlts.test_state
                from ResolvedLineCoverageTestStates rlts
                where rlts.product_id = tlt.product_id
                and rlts.cov_filepath = tlt.filepath
                and (
                    rlts.cov_file_hash is null
                    or rlts.cov_file_hash = tlt.file_hash
                )
                and rlts.cov_line = tlt.cov_line
                and rlts.test_state = $1
            )
            ",
            passed
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
