use anyhow::Context;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_trace_mappings(&mut self) -> Result<(), anyhow::Error> {
        self.update_trace_mapped_lines_covered_by_failed_test_runs()
            .await
            .context("Failed to update trace mapped lines covered by failing test runs")?;
        self.update_trace_mapped_lines_covered_by_failed_test_cases()
            .await
            .context("Failed to update trace mapped lines covered by failing test cases")?;
        self.update_trace_mapped_lines_covered_by_failed_tests()
            .await
            .context("Failed to update trace mapped lines covered by failing tests")?;
        self.update_traces_covered_by_failed_tests()
            .await
            .context("Failed to update traces covered by failing tests")?;

        self.update_trace_mapped_lines_covered_by_skipped_tests()
            .await
            .context("Updating trace mapped lines covered by skipped tests")?;
        self.update_traces_covered_by_skipped_tests()
            .await
            .context("Updating traces covered by skipped tests")?;

        self.update_trace_mapped_lines_only_covered_by_passed_test_runs()
            .await
            .context("Failed to update trace mapped lines covered only by passed test runs")?;
        self.update_trace_mapped_lines_only_covered_by_passed_test_cases()
            .await
            .context("Failed to update trace mapped lines covery only by passed test cases")?;
        self.update_trace_mapped_lines_only_covered_by_passed_tests()
            .await
            .context("Failed to update trace mapped lines covered only by passed tests")?;
        self.update_traces_only_covered_by_passed_tests()
            .await
            .context("Failed to update traces covered only by passed tests")?;

        Ok(())
    }

    async fn update_trace_mapped_lines_covered_by_failed_test_runs(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TraceMappedLinesCoveredByFailedTestRuns (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            )
            select
                tc.agg_collect_nr,
                tc.product_id,
                tc.test_collect_nr,
                tc.test_run_name,
                tc.test_run_date,
                tc.file_collect_nr,
                tc.filepath,
                tc.file_hash,
                tc.traced_line,
                tc.cov_line
            from TraceCoveragePerTestRuns tc, FailedTestRuns ftr
            where tc.agg_collect_nr = $1 and ftr.agg_collect_nr = $1
            and tc.product_id = ftr.product_id
            and tc.test_collect_nr = ftr.test_collect_nr
            and tc.test_run_name = ftr.test_run_name
            and tc.test_run_date = ftr.test_run_date
            and tc.hits > 0

            on conflict (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                filepath,
                traced_line,
                cov_line
            ) do update
            set
                file_collect_nr = max(TraceMappedLinesCoveredByFailedTestRuns.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_lines_covered_by_failed_test_cases(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TraceMappedLinesCoveredByFailedTestCases (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                test_case_name,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            )
            select
                tc.agg_collect_nr,
                tc.product_id,
                tc.test_collect_nr,
                tc.test_run_name,
                tc.test_run_date,
                tc.test_case_name,
                tc.file_collect_nr,
                tc.filepath,
                tc.file_hash,
                tc.traced_line,
                tc.cov_line
            from TraceCoveragePerTestCases tc, FailedTestCases ftc
            where tc.agg_collect_nr = $1 and ftc.agg_collect_nr = $1
            and tc.product_id = ftc.product_id
            and tc.test_collect_nr = ftc.test_collect_nr
            and tc.test_run_name = ftc.test_run_name
            and tc.test_run_date = ftc.test_run_date
            and tc.test_case_name = ftc.test_case_name
            and tc.hits > 0

            on conflict (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                test_case_name,
                filepath,
                traced_line,
                cov_line
            ) do update
            set
                file_collect_nr = max(TraceMappedLinesCoveredByFailedTestCases.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_lines_covered_by_failed_tests(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TraceMappedLinesCoveredByFailedTests (
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            )
            -- union, because tables may contain duplicate covered line entries
            select
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            from TraceMappedLinesCoveredByFailedTestRuns
            where agg_collect_nr = $1

            union

            select
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            from TraceMappedLinesCoveredByFailedTestCases
            where agg_collect_nr = $1

            on conflict (
                agg_collect_nr,
                product_id,
                filepath,
                traced_line,
                cov_line
            ) do update
            set
                file_collect_nr = max(TraceMappedLinesCoveredByFailedTests.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_covered_by_failed_tests(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TracesCoveredByFailedTests (
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line
            from TraceMappedLinesCoveredByFailedTests
            where agg_collect_nr = $1

            on conflict (
                agg_collect_nr,
                product_id,
                filepath,
                traced_line
            ) do update
            set
                file_collect_nr = max(TracesCoveredByFailedTests.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_lines_covered_by_skipped_tests(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TraceMappedLinesCoveredBySkippedTests (
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            )
            -- union, because tables may contain duplicate covered line entries
            select
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            from (
                select
                    tc.agg_collect_nr,
                    tc.product_id,
                    tc.file_collect_nr,
                    tc.filepath,
                    tc.file_hash,
                    tc.traced_line,
                    tc.cov_line
                from TraceCoveragePerTestRuns tc, SkippedTestRuns str
                where tc.agg_collect_nr = $1 and str.agg_collect_nr = $1
                and tc.product_id = str.product_id
                and tc.test_collect_nr = str.test_collect_nr
                and tc.test_run_name = str.test_run_name
                and tc.test_run_date = str.test_run_date
                and tc.hits > 0

                union

                select
                    tc.agg_collect_nr,
                    tc.product_id,
                    tc.file_collect_nr,
                    tc.filepath,
                    tc.file_hash,
                    tc.traced_line,
                    tc.cov_line
                from TraceCoveragePerTestCases tc, SkippedTestCases stc
                where tc.agg_collect_nr = $1 and stc.agg_collect_nr = $1
                and tc.product_id = stc.product_id
                and tc.test_collect_nr = stc.test_collect_nr
                and tc.test_run_name = stc.test_run_name
                and tc.test_run_date = stc.test_run_date
                and tc.test_case_name = stc.test_case_name
                and tc.hits > 0
            )
            -- hack to prevent on-conflict bellow from being taken as join
            where true

            on conflict (
                agg_collect_nr,
                product_id,
                filepath,
                traced_line,
                cov_line
            ) do update
            set
                file_collect_nr = max(TraceMappedLinesCoveredBySkippedTests.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_covered_by_skipped_tests(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TracesCoveredBySkippedTests (
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line
            from TraceMappedLinesCoveredBySkippedTests
            where agg_collect_nr = $1

            on conflict (
                agg_collect_nr,
                product_id,
                filepath,
                traced_line
            ) do update
            set
                file_collect_nr = max(TracesCoveredBySkippedTests.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_lines_only_covered_by_passed_test_runs(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TraceMappedLinesOnlyCoveredByPassedTestRuns (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            )
            select
                tc.agg_collect_nr,
                tc.product_id,
                tc.test_collect_nr,
                tc.test_run_name,
                tc.test_run_date,
                tc.file_collect_nr,
                tc.filepath,
                tc.file_hash,
                tc.traced_line,
                tc.cov_line
            from TraceCoveragePerTestRuns tc, UsableTestRuns ut
            where tc.agg_collect_nr = $1 and ut.agg_collect_nr = $1
            and tc.product_id = ut.product_id
            and tc.test_run_name = ut.test_run_name
            and tc.test_run_date = ut.test_run_date
            and tc.hits > 0
            and not exists (
                select fst.cov_line
                from TraceMappedLinesCoveredByFailedOrSkippedTests fst
                where tc.agg_collect_nr = fst.agg_collect_nr
                and tc.product_id = fst.product_id
                and tc.filepath = fst.filepath
                and tc.cov_line = fst.cov_line
            )

            on conflict (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                filepath,
                traced_line,
                cov_line
            ) do update
            set
                file_collect_nr = max(TraceMappedLinesOnlyCoveredByPassedTestRuns.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_lines_only_covered_by_passed_test_cases(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TraceMappedLinesOnlyCoveredByPassedTestCases (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                test_case_name,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            )
            select
                tc.agg_collect_nr,
                tc.product_id,
                tc.test_collect_nr,
                tc.test_run_name,
                tc.test_run_date,
                tc.test_case_name,
                tc.file_collect_nr,
                tc.filepath,
                tc.file_hash,
                tc.traced_line,
                tc.cov_line
            from TraceCoveragePerTestCases tc, UsableTestCases uc
            where tc.agg_collect_nr = $1 and uc.agg_collect_nr = $1
            and tc.product_id = uc.product_id
            and tc.test_run_name = uc.test_run_name
            and tc.test_run_date = uc.test_run_date
            and tc.test_case_name = uc.test_case_name
            and tc.hits > 0
            and not exists (
                select fst.cov_line
                from TraceMappedLinesCoveredByFailedOrSkippedTests fst
                where tc.agg_collect_nr = fst.agg_collect_nr
                and tc.product_id = fst.product_id
                and tc.filepath = fst.filepath
                and tc.cov_line = fst.cov_line
            )

            on conflict (
                agg_collect_nr,
                product_id,
                test_collect_nr,
                test_run_name,
                test_run_date,
                test_case_name,
                filepath,
                traced_line,
                cov_line
            ) do update
            set
                file_collect_nr = max(TraceMappedLinesOnlyCoveredByPassedTestCases.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_lines_only_covered_by_passed_tests(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TraceMappedLinesOnlyCoveredByPassedTests (
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            )
            -- union, because tables may contain duplicate covered line entries
            select
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            from TraceMappedLinesOnlyCoveredByPassedTestRuns
            where agg_collect_nr = $1

            union

            select
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line,
                cov_line
            from TraceMappedLinesOnlyCoveredByPassedTestCases
            where agg_collect_nr = $1

            on conflict (
                agg_collect_nr,
                product_id,
                filepath,
                traced_line,
                cov_line
            ) do update
            set
                file_collect_nr = max(TraceMappedLinesOnlyCoveredByPassedTests.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_only_covered_by_passed_tests(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into TracesOnlyCoveredByPassedTests (
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                agg_collect_nr,
                product_id,
                file_collect_nr,
                filepath,
                file_hash,
                traced_line
            from TraceMappedLinesOnlyCoveredByPassedTests pt
            where agg_collect_nr = $1
            and not exists (
                select ft.traced_line
                from TraceMappedLinesCoveredByFailedTests ft
                where ft.agg_collect_nr = $1
                and pt.product_id = ft.product_id
                and pt.filepath = ft.filepath
                and pt.file_hash = ft.file_hash
                and pt.traced_line = ft.traced_line
            )
            and not exists (
                select st.traced_line
                from TraceMappedLinesCoveredBySkippedTests st
                where st.agg_collect_nr = $1
                and pt.product_id = st.product_id
                and pt.filepath = st.filepath
                and pt.file_hash = st.file_hash
                and pt.traced_line = st.traced_line
            )

            on conflict (
                agg_collect_nr,
                product_id,
                filepath,
                traced_line
            ) do update
            set
                file_collect_nr = max(TracesOnlyCoveredByPassedTests.file_collect_nr, excluded.file_collect_nr),
                file_hash = excluded.file_hash
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
