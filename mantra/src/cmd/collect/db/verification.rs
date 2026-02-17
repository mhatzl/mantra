use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_verification_data(&mut self) -> Result<(), anyhow::Error> {
        self.update_traces_only_covered_by_passed_test_runs()
            .await?;
        self.update_traces_only_covered_by_passed_test_cases()
            .await?;

        Ok(())
    }

    async fn update_traces_only_covered_by_passed_test_runs(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TracesOnlyCoveredByPassedTestRuns (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            )
            select
                tc.last_collect_nr,
                tc.product_id,
                tc.test_run_name,
                tc.test_run_date,
                tc.filepath,
                tc.file_hash,
                tc.traced_line,
                tc.stmnt_line
            from TraceCoveragePerTestRuns tc, UsableTestRuns pt
            where tc.last_collect_nr = $1 and pt.last_collect_nr = $1
            and tc.product_id = $2 and pt.product_id = $2
            and tc.test_run_name = pt.test_run_name
            and tc.test_run_date = pt.test_run_date
            and tc.hits > 0
            and not exists (
                select sc.stmnt_filepath, sc.stmnt_file_hash, sc.stmnt_line
                from FailedTestRuns f, ResolvedTestRunStatementCoverage sc
                where f.last_collect_nr = $1 and f.product_id = $2
                and sc.last_collect_nr = $1 and sc.product_id = $2
                and f.test_run_name = sc.test_run_name
                and f.test_run_date = sc.test_run_date
                and sc.stmnt_filepath = tc.filepath
                and (sc.stmnt_file_hash is null or tc.file_hash is null
                    or sc.stmnt_file_hash = tc.file_hash)
                and sc.stmnt_line = tc.stmnt_line
            )
            and not exists (
                select sc.stmnt_filepath, sc.stmnt_file_hash, sc.stmnt_line
                from SkippedTestRuns s, ResolvedTestRunStatementCoverage sc
                where s.last_collect_nr = $1 and s.product_id = $2
                and sc.last_collect_nr = $1 and sc.product_id = $2
                and s.test_run_name = sc.test_run_name
                and s.test_run_date = sc.test_run_date
                and sc.stmnt_filepath = tc.filepath
                and (sc.stmnt_file_hash is null or tc.file_hash is null
                    or sc.stmnt_file_hash = tc.file_hash)
                and sc.stmnt_line = tc.stmnt_line
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TracesOnlyCoveredByPassedTestRuns
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_only_covered_by_passed_test_cases(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TracesOnlyCoveredByPassedTestCases (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            )
            select
                tc.last_collect_nr,
                tc.product_id,
                tc.test_run_name,
                tc.test_run_date,
                tc.test_case_name,
                tc.filepath,
                tc.file_hash,
                tc.traced_line,
                tc.stmnt_line
            from TraceCoveragePerTestCases tc, UsableTestCases uc
            where tc.last_collect_nr = $1 and uc.last_collect_nr = $1
            and tc.product_id = $2 and uc.product_id = $2
            and tc.test_run_name = uc.test_run_name
            and tc.test_run_date = uc.test_run_date
            and tc.test_case_name = uc.test_case_name
            and tc.hits > 0
            and not exists (
                select sc.stmnt_filepath, sc.stmnt_file_hash, sc.stmnt_line
                from FailedTestCases f, ResolvedTestCaseStatementCoverage sc
                where f.last_collect_nr = $1 and f.product_id = $2
                and sc.last_collect_nr = $1 and sc.product_id = $2
                and f.test_run_name = sc.test_run_name
                and f.test_run_date = sc.test_run_date
                and f.test_case_name = sc.test_case_name
                and sc.stmnt_filepath = tc.filepath
                and (sc.stmnt_file_hash is null or tc.file_hash is null
                    or sc.stmnt_file_hash = tc.file_hash)
                and sc.stmnt_line = tc.stmnt_line
            )
            and not exists (
                select sc.stmnt_filepath, sc.stmnt_file_hash, sc.stmnt_line
                from SkippedTestCases s, ResolvedTestCaseStatementCoverage sc
                where s.last_collect_nr = $1 and s.product_id = $2
                and sc.last_collect_nr = $1 and sc.product_id = $2
                and s.test_run_name = sc.test_run_name
                and s.test_run_date = sc.test_run_date
                and s.test_case_name = sc.test_case_name
                and sc.stmnt_filepath = tc.filepath
                and (sc.stmnt_file_hash is null or tc.file_hash is null
                    or sc.stmnt_file_hash = tc.file_hash)
                and sc.stmnt_line = tc.stmnt_line
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TracesOnlyCoveredByPassedTestCases
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
