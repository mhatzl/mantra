use mantra_schema::{annotations::TraceKind, test_runs::TestCaseState};

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_verification_data(&mut self) -> Result<(), anyhow::Error> {
        self.update_trace_mapped_stmnts_only_covered_by_passed_test_runs()
            .await?;
        self.update_trace_mapped_stmnts_only_covered_by_passed_test_cases()
            .await?;
        self.update_trace_mapped_stmnts_only_covered_by_passed_tests()
            .await?;

        self.update_trace_mapped_stmnts_covered_by_failed_test_runs()
            .await?;
        self.update_trace_mapped_stmnts_covered_by_failed_test_cases()
            .await?;
        self.update_trace_mapped_stmnts_covered_by_failed_tests()
            .await?;

        self.update_traces_only_covered_by_passed_tests().await?;
        self.update_traces_covered_by_failed_tests().await?;

        self.update_direct_req_verification_states().await?;
        self.update_indirect_req_verification_states().await?;

        self.update_verified_reqs().await?;
        self.update_failed_reqs().await?;
        self.update_skipped_reqs().await?;
        self.update_unverified_reqs().await?;

        Ok(())
    }

    async fn update_trace_mapped_stmnts_only_covered_by_passed_test_runs(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceMappedStmntsOnlyCoveredByPassedTestRuns (
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
            from TraceCoveragePerTestRuns tc, UsableTestRuns ut
            where tc.last_collect_nr = $1 and ut.last_collect_nr = $1
            and tc.product_id = $2 and ut.product_id = $2
            and tc.test_run_name = ut.test_run_name
            and tc.test_run_date = ut.test_run_date
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
            delete from TraceMappedStmntsOnlyCoveredByPassedTestRuns
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_stmnts_only_covered_by_passed_test_cases(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceMappedStmntsOnlyCoveredByPassedTestCases (
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
            delete from TraceMappedStmntsOnlyCoveredByPassedTestCases
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_stmnts_only_covered_by_passed_tests(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceMappedStmntsOnlyCoveredByPassedTests (
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            )
            -- union, because tables may contain duplicate statement entries
            select
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            from TraceMappedStmntsOnlyCoveredByPassedTestRuns
            where last_collect_nr = $1 and product_id = $2
            union
            select
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            from TraceMappedStmntsOnlyCoveredByPassedTestCases
            where last_collect_nr = $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TraceMappedStmntsOnlyCoveredByPassedTests
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_only_covered_by_passed_tests(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TracesOnlyCoveredByPassedTests (
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line
            )
            select
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line
            from TraceMappedStmntsOnlyCoveredByPassedTests pt
            where last_collect_nr = $1 and product_id = $2
            and not exists (
                select
                    last_collect_nr,
                    product_id,
                    filepath,
                    file_hash,
                    traced_line
                    stmnt_line
                from TraceMappedStmntsCoveredByFailedTests ft
                where ft.last_collect_nr = $1 and ft.product_id = $2
                and pt.filepath = ft.filepath
                and pt.file_hash = ft.file_hash
                and pt.traced_line = ft.traced_line
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TracesOnlyCoveredByPassedTests
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_stmnts_covered_by_failed_test_runs(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceMappedStmntsCoveredByFailedTestRuns (
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
            from TraceCoveragePerTestRuns tc, FailedTestRuns ft
            where tc.last_collect_nr = $1 and ft.last_collect_nr = $1
            and tc.product_id = $2 and ft.product_id = $2
            and tc.test_run_name = ft.test_run_name
            and tc.test_run_date = ft.test_run_date
            and tc.hits > 0
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TraceMappedStmntsCoveredByFailedTestRuns
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_stmnts_covered_by_failed_test_cases(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceMappedStmntsCoveredByFailedTestCases (
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
            from TraceCoveragePerTestCases tc, FailedTestCases fc
            where tc.last_collect_nr = $1 and fc.last_collect_nr = $1
            and tc.product_id = $2 and fc.product_id = $2
            and tc.test_run_name = fc.test_run_name
            and tc.test_run_date = fc.test_run_date
            and tc.test_case_name = fc.test_case_name
            and tc.hits > 0
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TraceMappedStmntsCoveredByFailedTestCases
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_trace_mapped_stmnts_covered_by_failed_tests(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceMappedStmntsCoveredByFailedTests (
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            )
            -- union, because tables may contain duplicate statement entries
            select
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            from TraceMappedStmntsCoveredByFailedTestRuns
            where last_collect_nr = $1 and product_id = $2
            union
            select
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line,
                stmnt_line
            from TraceMappedStmntsCoveredByFailedTestCases
            where last_collect_nr = $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TraceMappedStmntsCoveredByFailedTests
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_traces_covered_by_failed_tests(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TracesCoveredByFailedTests (
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line
            )
            select distinct
                last_collect_nr,
                product_id,
                filepath,
                file_hash,
                traced_line
            from TraceMappedStmntsCoveredByFailedTests
            where last_collect_nr = $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TracesCoveredByFailedTests
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_direct_req_verification_states(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let satisfies_trace_nr = TraceKind::Satisfies.as_nr();
        let verifies_trace_nr = TraceKind::Verifies.as_nr();
        let req_verified_nr = RequirementVerificationState::Verified.as_nr();
        let req_failed_nr = RequirementVerificationState::Failed.as_nr();
        let req_skipped_nr = RequirementVerificationState::Skipped.as_nr();
        let req_unverified_nr = RequirementVerificationState::Unverified.as_nr();
        let test_case_passed_nr = TestCaseState::Passed.as_nr();
        let test_case_skipped_nr = TestCaseState::Skipped.as_nr();

        sqlx::query!(
            "
            insert or replace into DirectRequirementVerificationStates (
                last_collect_nr,
                product_id,
                id,
                state
            )
            with SatisfyTraceExists (
                req_id
            ) as (
                select distinct rt.req_id
                from Traces t, DirectProductReqTraces rt
                where rt.last_collect_nr = $1 and rt.product_id = $2
                and rt.file_hash = t.file_hash
                and rt.line = t.line
                and t.kind = $3
            ) , VerifyTraceExists (
                req_id
            ) as (
                select rt.req_id
                from Traces t, DirectProductReqTraces rt
                where rt.last_collect_nr = $1 and rt.product_id = $2
                and rt.file_hash = t.file_hash
                and rt.line = t.line
                and t.kind = $4
            ), SatisfyAndVerifyTraceExists (
                req_id
            ) as (
                select req_id
                from SatisfyTraceExists st
                where exists (
                    select vt.req_id
                    from VerifyTraceExists vt
                    where st.req_id = vt.req_id
                )
            ), NoSatisfyTraceButVerifyTraces (
                req_id
            ) as (
                select req_id
                from VerifyTraceExists
                except
                select req_id
                from SatisfyTraceExists
            ), NoVerifyTraceButSatisfyTraces (
                req_id
            ) as (
                select req_id
                from SatisfyTraceExists
                except
                select req_id
                from VerifyTraceExists
            ) , ReqsOnlyCoveredByPassedTests (
                id
            ) as (
                select distinct dt.req_id
                from TracesOnlyCoveredByPassedTests ct, Traces t, DirectProductReqTraces dt
                where dt.last_collect_nr = $1 and dt.product_id = $2
                and ct.filepath = dt.filepath
                and ct.file_hash = dt.file_hash
                and ct.traced_line = dt.line
                and ct.file_hash = t.file_hash
                and ct.traced_line = t.line
            ), ReqsCoveredByFailedTests (
                id
            ) as (
                select distinct dt.req_id
                from TracesCoveredByFailedTests ct, Traces t, DirectProductReqTraces dt
                where dt.last_collect_nr = $1 and dt.product_id = $2
                and ct.filepath = dt.filepath
                and ct.file_hash = dt.file_hash
                and ct.traced_line = dt.line
                and ct.file_hash = t.file_hash
                and ct.traced_line = t.line
            ), ReqsWithTestRunsCoveringSatisfyAndVerifyTraces (
                id
            ) as (
                select distinct rt.req_id
                from TracesCoveredByTestRuns ct, Traces t, DirectProductReqTraces rt
                where ct.last_collect_nr = $1 and rt.product_id = $2
                and ct.last_collect_nr = rt.last_collect_nr
                and ct.product_id = rt.product_id
                and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
                and ct.file_hash = t.file_hash and ct.traced_line = t.line
                and ct.filepath = rt.filepath
                -- verify trace
                and t.kind = $4
                -- satisfy trace covered by same test run
                and exists (
                    select dt.req_id
                    from TracesCoveredByTestRuns st, Traces tr, DirectProductReqTraces dt
                    where st.last_collect_nr = $1 and dt.product_id = $2
                    and st.last_collect_nr = dt.last_collect_nr
                    and st.product_id = dt.product_id
                    and st.file_hash = dt.file_hash and st.traced_line = dt.line
                    and st.file_hash = tr.file_hash and st.traced_line = tr.line
                    and st.filepath = dt.filepath
                    and dt.req_id = rt.req_id
                    -- same test run
                    and ct.test_run_name = st.test_run_name
                    and ct.test_run_date = st.test_run_date
                    -- satisfy trace
                    and tr.kind = $3
                )
            ), ReqsWithTestCasesCoveringSatisfyAndVerifyTraces (
                id
            ) as (
                select distinct rt.req_id
                from TracesCoveredByTestCases ct, Traces t, DirectProductReqTraces rt
                where ct.last_collect_nr = $1 and rt.product_id = $2
                and ct.last_collect_nr = rt.last_collect_nr
                and ct.product_id = rt.product_id
                and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
                and ct.file_hash = t.file_hash and ct.traced_line = t.line
                and ct.filepath = rt.filepath
                -- verify trace
                and t.kind = $4
                -- satisfy trace exists
                and exists (
                    select dt.req_id
                    from TracesCoveredByTestCases st, Traces tr, DirectProductReqTraces dt
                    where st.last_collect_nr = $1 and dt.product_id = $2
                    and st.last_collect_nr = dt.last_collect_nr
                    and st.product_id = dt.product_id
                    and st.file_hash = dt.file_hash and st.traced_line = dt.line
                    and st.file_hash = tr.file_hash and st.traced_line = tr.line
                    and st.filepath = dt.filepath
                    and dt.req_id = rt.req_id
                    -- same test case
                    and ct.test_run_name = st.test_run_name
                    and ct.test_run_date = st.test_run_date
                    and ct.test_case_name = st.test_case_name
                    -- satisfy trace
                    and tr.kind = $3
                )
            ), ReqsWithTestsCoveringSatisfyAndVerifyTraces (
                id
            ) as (
                select id
                from ReqsWithTestRunsCoveringSatisfyAndVerifyTraces
                union
                select id
                from ReqsWithTestCasesCoveringSatisfyAndVerifyTraces
            ), ReqsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces (
                id
            ) as (
                select distinct rt.req_id
                from TracesCoveredByTestRuns ct, Traces t, DirectProductReqTraces rt
                where ct.last_collect_nr = $1 and rt.product_id = $2
                and ct.last_collect_nr = rt.last_collect_nr
                and ct.product_id = rt.product_id
                and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
                and ct.file_hash = t.file_hash and ct.traced_line = t.line
                and ct.filepath = rt.filepath
                -- verify trace
                and t.kind = $4
                -- no satisfy trace covered by same test run
                and not exists (
                    select dt.req_id
                    from TracesCoveredByTestRuns st, Traces tr, DirectProductReqTraces dt
                    where st.last_collect_nr = $1 and dt.product_id = $2
                    and st.last_collect_nr = dt.last_collect_nr
                    and st.product_id = dt.product_id
                    and st.file_hash = dt.file_hash and st.traced_line = dt.line
                    and st.file_hash = tr.file_hash and st.traced_line = tr.line
                    and st.filepath = dt.filepath
                    and dt.req_id = rt.req_id
                    -- same test run
                    and ct.test_run_name = st.test_run_name
                    and ct.test_run_date = st.test_run_date
                    -- satisfy trace
                    and tr.kind = $3
                )
            ), ReqsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces (
                id
            ) as (
                select distinct rt.req_id
                from TracesCoveredByTestCases ct, Traces t, DirectProductReqTraces rt
                where ct.last_collect_nr = $1 and rt.product_id = $2
                and ct.last_collect_nr = rt.last_collect_nr
                and ct.product_id = rt.product_id
                and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
                and ct.file_hash = t.file_hash and ct.traced_line = t.line
                and ct.filepath = rt.filepath
                -- verify trace
                and t.kind = $4
                -- no satisfy trace exists
                and not exists (
                    select dt.req_id
                    from TracesCoveredByTestCases st, Traces tr, DirectProductReqTraces dt
                    where st.last_collect_nr = $1 and dt.product_id = $2
                    and st.last_collect_nr = dt.last_collect_nr
                    and st.product_id = dt.product_id
                    and st.file_hash = dt.file_hash and st.traced_line = dt.line
                    and st.file_hash = tr.file_hash and st.traced_line = tr.line
                    and st.filepath = dt.filepath
                    and dt.req_id = rt.req_id
                    -- same test case
                    and ct.test_run_name = st.test_run_name
                    and ct.test_run_date = st.test_run_date
                    and ct.test_case_name = st.test_case_name
                    -- satisfy trace
                    and tr.kind = $3
                )
            ), ReqsWithTestsCoveringVerifyTracesButNoSatisfyTraces (
                id
            ) as (
                select id
                from ReqsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces
                union
                select id
                from ReqsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces
            ), ReqsWithAllSatisfyTracesCovered (
                id
            ) as (
                select req_id
                from SatisfyTraceExists
                -- remove reqs that have at least one uncovered satisfy trace
                except
                select dt.req_id
                from DirectProductReqTraces dt, Traces t
                where dt.last_collect_nr = $1 and dt.product_id = $2
                and dt.file_hash = t.file_hash
                and dt.line = t.line
                and t.kind = $3
                and not exists (
                    select *
                    from TracesCoveredByTests ct
                    where ct.last_collect_nr = $1 and ct.product_id = $2
                    and dt.filepath = ct.filepath
                    and dt.file_hash = ct.file_hash
                    and dt.line = ct.traced_line
                )
            ), TracedReqStates (
                id,
                state
            ) as (
                select
                    dt.req_id,
                    case
                        when exists (
                            select f.id
                            from ReqsCoveredByFailedTests f
                            where dt.req_id = f.id
                        ) then $6
                        when exists (
                            select p.id
                            from ReqsOnlyCoveredByPassedTests p
                            where dt.req_id = p.id
                        ) then case
                            when exists (
                                select vt.req_id
                                from NoSatisfyTraceButVerifyTraces vt, ReqsWithTestsCoveringVerifyTracesButNoSatisfyTraces ct
                                where vt.req_id = dt.req_id and dt.req_id = ct.id
                            ) then $5
                            when exists (
                                select vt.req_id
                                from NoVerifyTraceButSatisfyTraces vt, ReqsWithAllSatisfyTracesCovered ct
                                where vt.req_id = dt.req_id and dt.req_id = ct.id
                            ) then $5
                            when exists (
                                select svt.req_id
                                from SatisfyAndVerifyTraceExists svt, ReqsWithTestsCoveringSatisfyAndVerifyTraces ct
                                where svt.req_id = dt.req_id and dt.req_id = ct.id
                                and not exists (
                                    select bt.id
                                    from ReqsWithTestsCoveringVerifyTracesButNoSatisfyTraces bt
                                    where ct.id = bt.id
                                )
                            ) then $5
                            -- verified conditions not meet, even though tests passed => unverified
                            else $8
                            end
                        -- neither failed nor passed and since coverage cannot (realistically) be captured
                        -- from skipped tests, the state is unverified
                        else $8
                        end
                from DirectProductReqTraces dt
                where dt.last_collect_nr = $1 and dt.product_id = $2
            ), ReqsExplicitlyVerifiedByFailedOrUnknownTestCases (
                id
            ) as (
                select distinct vr.req_id
                from TestCaseVerifiedRequirements vr, TestCases tc
                where vr.last_collect_nr = $1 and vr.product_id = $2
                and tc.last_collect_nr = $1 and tc.product_id = $2
                and vr.test_run_name = tc.test_run_name
                and vr.test_run_date = tc.test_run_date
                and vr.test_case_name = tc.name
                -- neither passed nor skipped => failed or unknown
                and tc.state != $9 and tc.state != $10
            ), ReqsExplicitlyVerifiedBySkippedTestCases (
                id
            ) as (
                select distinct vr.req_id
                from TestCaseVerifiedRequirements vr, TestCases tc
                where vr.last_collect_nr = $1 and vr.product_id = $2
                and tc.last_collect_nr = $1 and tc.product_id = $2
                and vr.test_run_name = tc.test_run_name
                and vr.test_run_date = tc.test_run_date
                and vr.test_case_name = tc.name
                and tc.state = $10
                except
                select id
                from ReqsExplicitlyVerifiedByFailedOrUnknownTestCases
            ), ReqsExplicitlyVerifiedOnlyByPassedTestCases (
                id
            ) as (
                select distinct
                    vr.req_id
                from TestCaseVerifiedRequirements vr, TestCases tc
                where vr.last_collect_nr = $1 and vr.product_id = $2
                and tc.last_collect_nr = $1 and tc.product_id = $2
                and vr.test_run_name = tc.test_run_name
                and vr.test_run_date = tc.test_run_date
                and vr.test_case_name = tc.name
                and tc.state = $9
                except
                select id
                from (
                    select id
                    from ReqsExplicitlyVerifiedByFailedOrUnknownTestCases
                    union all
                    select id
                    from ReqsExplicitlyVerifiedBySkippedTestCases
                )
            ), ReqsExplicitlyVerifiedByTestCasesState (
                id,
                state
            ) as (
                select
                    vr.req_id,
                    case
                        -- passed -> verified
                        when exists (
                            select p.id
                            from ReqsExplicitlyVerifiedOnlyByPassedTestCases p
                            where p.id = vr.req_id
                        ) then $5
                        -- skipped
                        when exists (
                            select s.id
                            from ReqsExplicitlyVerifiedBySkippedTestCases s
                            where s.id = vr.req_id
                        ) then $7
                        -- failed/unknown -> failed
                        else $6
                    end
                from (
                    select distinct req_id
                    from TestCaseVerifiedRequirements
                    where last_collect_nr = $1 and product_id = $2
                ) vr
            ), ManualReqStates (
                id,
                state
            ) as (
                select
                    mr.id,
                    case
                        when exists (
                            select ts.id
                            from TracedReqStates ts
                            where mr.id = ts.id and ts.state = $6
                        ) or exists (
                            select ts.id
                            from ReqsExplicitlyVerifiedByTestCasesState ts
                            where mr.id = ts.id and ts.state = $6
                        ) then $6
                        when exists (
                            select vr.req_id
                            from ManuallyVerifiedRequirements vr
                            where vr.last_collect_nr = $1 and vr.product_id = $2
                            and mr.id = vr.req_id
                        ) then case
                            -- verify trace exists, but wasn't verified => unverified
                            -- independent of review
                            when exists (
                                select ts.id
                                from TracedReqStates ts, VerifyTraceExists vt
                                where mr.id = ts.id and mr.id = vt.req_id
                                and ts.state = $8
                            ) then $8
                            -- explicit verify test exists, but is skipped
                            when exists (
                                select ts.id
                                from ReqsExplicitlyVerifiedByTestCasesState ts
                                where mr.id = ts.id and ts.state = $7
                            ) then $7
                            else $5
                            end
                        else $8
                    end
                from UsableManualRequirements mr
                where mr.last_collect_nr = $1 and mr.product_id = $2
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id,
                case
                    when mr.id not null then mr.state
                    when exists (
                        select ts.id
                        from TracedReqStates ts
                        where r.id = ts.id and ts.state = $6
                    ) then $6
                    when exists (
                        select ts.id
                        from ReqsExplicitlyVerifiedByTestCasesState ts
                        where r.id = ts.id and ts.state = $6
                    ) then $6
                    when exists (
                        select ts.id
                        from ReqsExplicitlyVerifiedByTestCasesState ts
                        where r.id = ts.id and ts.state = $7
                    ) then $7
                    when exists (
                        select ts.id
                        from TracedReqStates ts
                        where r.id = ts.id and ts.state = $5
                    ) then $5
                    when exists (
                        select ts.id
                        from ReqsExplicitlyVerifiedByTestCasesState ts
                        where r.id = ts.id and ts.state = $5
                    ) then $5
                    else $8
                end
            from UsableRequirements r left join ManualReqStates mr on r.id = mr.id
            where r.last_collect_nr = $1 and r.product_id = $2
            ",
            collect_nr,
            product_id,
            satisfies_trace_nr,
            verifies_trace_nr,
            req_verified_nr,
            req_failed_nr,
            req_skipped_nr,
            req_unverified_nr,
            test_case_passed_nr,
            test_case_skipped_nr
         )
         .execute(self.connection_mut())
         .await?;

        sqlx::query!(
            "
             delete from DirectRequirementVerificationStates
             where last_collect_nr != $1 and product_id = $2
             ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_indirect_req_verification_states(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let req_verified_nr = RequirementVerificationState::Verified.as_nr();
        let req_failed_nr = RequirementVerificationState::Failed.as_nr();
        let req_skipped_nr = RequirementVerificationState::Skipped.as_nr();
        let req_unverified_nr = RequirementVerificationState::Unverified.as_nr();

        sqlx::query!(
            "
            insert or replace into IndirectRequirementVerificationStates (
                last_collect_nr,
                product_id,
                id,
                state
            )
            with UsableNonLeafRequirements (
                last_collect_nr,
                product_id,
                id
            ) as (
                select
                    ur.last_collect_nr,
                    ur.product_id,
                    ur.id
                from UsableRequirements ur, RequirementDescendants rd
                where ur.last_collect_nr = $1 and ur.product_id = $2
                and rd.last_collect_nr = $1 and rd.product_id = $2
                and ur.id = rd.id
            ), ReqsWithFailedDescendants (id) as (
                select r.id
                from UsableNonLeafRequirements r
                where exists (
                    select rd.id
                    from RequirementDescendants rd, DirectRequirementVerificationStates s
                    where rd.last_collect_nr = $1 and rd.product_id = $2
                    and rd.id = r.id and rd.descendant_id = s.id
                    and rd.descendant_product_id = s.product_id
                    and s.last_collect_nr = (
                        select max(last_collect_nr)
                        from Products p
                        where p.id = s.product_id
                    )
                    and s.state = $4
                )
            ), ReqsWithUnverifiedNonOptionalDescendants (id) as (
                select r.id
                from UsableNonLeafRequirements r
                where exists (
                    select rd.id
                    from RequirementDescendants rd, DirectRequirementVerificationStates s
                        left join OptionalRequirements opt on
                            s.last_collect_nr = opt.last_collect_nr
                            and s.product_id = opt.product_id
                            and s.id = opt.id
                    where rd.last_collect_nr = $1 and rd.product_id = $2
                    and rd.id = r.id and rd.descendant_id = s.id
                    and rd.descendant_product_id = s.product_id
                    and s.last_collect_nr = (
                        select max(last_collect_nr)
                        from Products p
                        where p.id = s.product_id
                    )
                    and opt.id is null
                    and s.state = $6
                )
            ), ReqsWithSkippedNonOptionalDescendants (id) as (
                select r.id
                from UsableNonLeafRequirements r
                where exists (
                    select rd.id
                    from RequirementDescendants rd, DirectRequirementVerificationStates s
                        left join OptionalRequirements opt on
                            s.last_collect_nr = opt.last_collect_nr
                            and s.product_id = opt.product_id
                            and s.id = opt.id
                    where rd.last_collect_nr = $1 and rd.product_id = $2
                    and rd.id = r.id and rd.descendant_id = s.id
                    and rd.descendant_product_id = s.product_id
                    and s.last_collect_nr = (
                        select max(last_collect_nr)
                        from Products p
                        where p.id = s.product_id
                    )
                    and opt.id is null
                    and s.state = $5
                )
            ), ReqsWithVerifiedNonOptionalDescendants (id) as (
                select r.id
                from UsableNonLeafRequirements r
                where exists (
                    select rd.id
                    from RequirementDescendants rd, DirectRequirementVerificationStates s
                        left join OptionalRequirements opt on
                            s.last_collect_nr = opt.last_collect_nr
                            and s.product_id = opt.product_id
                            and s.id = opt.id
                    where rd.last_collect_nr = $1 and rd.product_id = $2
                    and rd.id = r.id and rd.descendant_id = s.id
                    and rd.descendant_product_id = s.product_id
                    and s.last_collect_nr = (
                        select max(last_collect_nr)
                        from Products p
                        where p.id = s.product_id
                    )
                    and opt.id is null
                    and s.state = $3
                )
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id,
                case
                    when exists (
                        select f.id
                        from ReqsWithFailedDescendants f
                        where r.id = f.id
                    ) then $4
                    when exists (
                        select f.id
                        from ReqsWithUnverifiedNonOptionalDescendants f
                        where r.id = f.id
                    ) then $6
                    when exists (
                        select f.id
                        from ReqsWithSkippedNonOptionalDescendants f
                        where r.id = f.id
                    ) then $5
                    when exists (
                        select f.id
                        from ReqsWithVerifiedNonOptionalDescendants f
                        where r.id = f.id
                    ) then $3
                    else $6
                end
            from UsableNonLeafRequirements r
            ",
            collect_nr,
            product_id,
            req_verified_nr,
            req_failed_nr,
            req_skipped_nr,
            req_unverified_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
             delete from IndirectRequirementVerificationStates
             where last_collect_nr != $1 and product_id = $2
             ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_verified_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let req_verified_nr = RequirementVerificationState::Verified.as_nr();
        let req_failed_nr = RequirementVerificationState::Failed.as_nr();
        let req_skipped_nr = RequirementVerificationState::Skipped.as_nr();
        let req_unverified_nr = RequirementVerificationState::Unverified.as_nr();

        sqlx::query!(
            "
            insert or replace into VerifiedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id
            from UsableRequirements r, DirectRequirementVerificationStates ds
                left join IndirectRequirementVerificationStates ids on
                r.last_collect_nr = ids.last_collect_nr
                and r.product_id = ids.product_id
                and r.id = ids.id
            where r.last_collect_nr = $1 and r.product_id = $2
            and ds.last_collect_nr = $1 and ds.product_id = $2
            and r.id = ds.id
            and (ids.state is null or ids.state != $4)
            and (ds.state = $3
                or (ds.state = $5 and (ids.state is not null and ids.state = $3))
            )
            ",
            collect_nr,
            product_id,
            req_verified_nr,
            req_failed_nr,
            req_unverified_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
             delete from VerifiedRequirements
             where last_collect_nr != $1 and product_id = $2
             ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_skipped_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let req_verified_nr = RequirementVerificationState::Verified.as_nr();
        let req_failed_nr = RequirementVerificationState::Failed.as_nr();
        let req_skipped_nr = RequirementVerificationState::Skipped.as_nr();
        let req_unverified_nr = RequirementVerificationState::Unverified.as_nr();

        sqlx::query!(
            "
            insert or replace into SkippedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id
            from UsableRequirements r, DirectRequirementVerificationStates ds
                left join IndirectRequirementVerificationStates ids on
                r.last_collect_nr = ids.last_collect_nr
                and r.product_id = ids.product_id
                and r.id = ids.id
            where r.last_collect_nr = $1 and r.product_id = $2
            and ds.last_collect_nr = $1 and ds.product_id = $2
            and r.id = ds.id
            and (ids.state is null or ids.state != $4)
            and (ds.state = $3
                or (ds.state = $5 and (ids.state is not null and ids.state = $3))
            )
            ",
            collect_nr,
            product_id,
            req_skipped_nr,
            req_failed_nr,
            req_unverified_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
             delete from SkippedRequirements
             where last_collect_nr != $1 and product_id = $2
             ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_failed_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let req_verified_nr = RequirementVerificationState::Verified.as_nr();
        let req_failed_nr = RequirementVerificationState::Failed.as_nr();
        let req_skipped_nr = RequirementVerificationState::Skipped.as_nr();
        let req_unverified_nr = RequirementVerificationState::Unverified.as_nr();

        sqlx::query!(
            "
            insert or replace into FailedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id
            from UsableRequirements r, DirectRequirementVerificationStates ds
                left join IndirectRequirementVerificationStates ids on
                r.last_collect_nr = ids.last_collect_nr
                and r.product_id = ids.product_id
                and r.id = ids.id
            where r.last_collect_nr = $1 and r.product_id = $2
            and ds.last_collect_nr = $1 and ds.product_id = $2
            and r.id = ds.id
            and ((ids.state is not null and ids.state = $3) or ds.state = $3)
            ",
            collect_nr,
            product_id,
            req_failed_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
             delete from FailedRequirements
             where last_collect_nr != $1 and product_id = $2
             ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_unverified_reqs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let req_verified_nr = RequirementVerificationState::Verified.as_nr();
        let req_failed_nr = RequirementVerificationState::Failed.as_nr();
        let req_skipped_nr = RequirementVerificationState::Skipped.as_nr();
        let req_unverified_nr = RequirementVerificationState::Unverified.as_nr();

        sqlx::query!(
            "
            insert or replace into UnverifiedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id
            from UsableRequirements r, DirectRequirementVerificationStates ds
                left join IndirectRequirementVerificationStates ids on
                r.last_collect_nr = ids.last_collect_nr
                and r.product_id = ids.product_id
                and r.id = ids.id
            where r.last_collect_nr = $1 and r.product_id = $2
            and ds.last_collect_nr = $1 and ds.product_id = $2
            and r.id = ds.id
            and ds.state = $3 and (ids.state is null or ids.state = $3 or ids.state = $4)
            ",
            collect_nr,
            product_id,
            req_unverified_nr,
            req_skipped_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
             delete from UnverifiedRequirements
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequirementVerificationState {
    Failed = 0,
    Verified = 1,
    Skipped = 2,
    Unverified = 3,
}

impl RequirementVerificationState {
    fn as_nr(&self) -> i32 {
        *self as i32
    }
}
