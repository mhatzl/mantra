use mantra_schema::{
    annotations::TraceKind, report::requirement::RequirementState, test_runs::TestCaseState,
};

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(super) async fn update_direct_req_verification_states(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let satisfies_trace_nr = TraceKind::Satisfies.as_nr();
        let verifies_trace_nr = TraceKind::Verifies.as_nr();
        let req_verified_nr = RequirementState::Verified.as_nr();
        let req_failed_nr = RequirementState::Failed.as_nr();
        let req_skipped_nr = RequirementState::Skipped.as_nr();
        let req_satisfied_nr = RequirementState::Satisfied.as_nr();
        let req_untraced_nr = RequirementState::Untraced.as_nr();
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
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where vr.last_collect_nr = $1 and vr.product_id = $2
            and tc.last_collect_nr = $1 and tc.product_id = $2
            and vr.test_run_name = tc.test_run_name
            and vr.test_run_date = tc.test_run_date
            and vr.test_case_name = tc.test_case_name
            -- neither passed nor skipped => failed or unknown
            and tc.state != $9 and tc.state != $10
        ), ReqsExplicitlyVerifiedBySkippedTestCases (
            id
        ) as (
            select distinct vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where vr.last_collect_nr = $1 and vr.product_id = $2
            and tc.last_collect_nr = $1 and tc.product_id = $2
            and vr.test_run_name = tc.test_run_name
            and vr.test_run_date = tc.test_run_date
            and vr.test_case_name = tc.test_case_name
            and tc.state = $10
            except
            select id
            from ReqsExplicitlyVerifiedByFailedOrUnknownTestCases
        ), ReqsExplicitlyVerifiedOnlyByPassedTestCases (
            id
        ) as (
            select distinct
                vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where vr.last_collect_nr = $1 and vr.product_id = $2
            and tc.last_collect_nr = $1 and tc.product_id = $2
            and vr.test_run_name = tc.test_run_name
            and vr.test_run_date = tc.test_run_date
            and vr.test_case_name = tc.test_case_name
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

        Ok(())
    }
}
