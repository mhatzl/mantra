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
        let req_unknown_nr = RequirementState::Unknown.as_nr();
        let req_partial_verified_nr = RequirementState::PartiallyVerified.as_nr();
        let test_case_passed_nr = TestCaseState::Passed.as_nr();
        let test_case_skipped_nr = TestCaseState::Skipped.as_nr();

        sqlx::query!(
        "
        insert into DirectRequirementVerificationStates (
            agg_collect_nr,
            req_collect_nr,
            product_id,
            req_id,
            state
        )
        with SatisfyTraceExists (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from Traces t, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and rt.file_hash = t.file_hash
            and rt.line = t.line
            and t.kind = $2
        ), VerifyTraceExists (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from Traces t, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and rt.file_hash = t.file_hash
            and rt.line = t.line
            and t.kind = $3
        ), SatisfyAndVerifyTraceExists (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select st.req_collect_nr, st.product_id, st.req_id
            from SatisfyTraceExists st, VerifyTraceExists vt
            where st.req_collect_nr = vt.req_collect_nr
            and st.product_id = vt.product_id
            and st.req_id = vt.req_id
        ), NoSatisfyTraceButVerifyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select req_collect_nr, product_id, req_id
            from VerifyTraceExists
            except
            select req_collect_nr, product_id, req_id
            from SatisfyTraceExists
        ), NoVerifyTraceButSatisfyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select req_collect_nr, product_id, req_id
            from SatisfyTraceExists
            except
            select req_collect_nr, product_id, req_id
            from VerifyTraceExists
        ) , ReqsOnlyCoveredByPassedTests (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from TracesOnlyCoveredByPassedTests ct, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and ct.agg_collect_nr = (
                select max(cct.agg_collect_nr)
                from TracesOnlyCoveredByPassedTests cct
                where cct.product_id = ct.product_id
            )
            and ct.product_id = rt.product_id
            and ct.filepath = rt.filepath
            and ct.file_hash = rt.file_hash
            and ct.traced_line = rt.line
        ), ReqsCoveredByFailedTests (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from TracesCoveredByFailedTests ct, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and ct.agg_collect_nr = (
                select max(cct.agg_collect_nr)
                from TracesCoveredByFailedTests cct
                where cct.product_id = ct.product_id
            )
            and ct.product_id = rt.product_id
            and ct.filepath = rt.filepath
            and ct.file_hash = rt.file_hash
            and ct.traced_line = rt.line
        ), ReqsCoveredBySkippedTests (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from TracesCoveredBySkippedTests ct, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and ct.agg_collect_nr = (
                select max(cct.agg_collect_nr)
                from TracesCoveredBySkippedTests cct
                where cct.product_id = ct.product_id
            )
            and ct.product_id = rt.product_id
            and ct.filepath = rt.filepath
            and ct.file_hash = rt.file_hash
            and ct.traced_line = rt.line
        ), ReqsWithTestRunsCoveringSatisfyAndVerifyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from TracesCoveredByTestRuns ct, Traces t, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and ct.agg_collect_nr = (
                select max(cct.agg_collect_nr)
                from TracesCoveredByTestRuns cct
                where cct.product_id = ct.product_id
            )
            and ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $3
            -- satisfy trace covered by same test run
            and exists (
                select rti.req_id
                from TracesCoveredByTestRuns cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.agg_collect_nr = rti.agg_collect_nr
                and rt.product_id = rti.product_id
                and rt.req_collect_nr = rti.req_collect_nr
                and rt.req_id = rti.req_id
                -- same test run
                and ct.agg_collect_nr = cti.agg_collect_nr
                and ct.product_id = cti.product_id
                and ct.test_collect_nr = cti.test_collect_nr
                and ct.test_run_name = cti.test_run_name
                and ct.test_run_date = cti.test_run_date
                -- satisfy trace
                and ti.kind = $2
            )
        ), ReqsWithTestCasesCoveringSatisfyAndVerifyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from TracesCoveredByTestCases ct, Traces t, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and ct.agg_collect_nr = (
                select max(cct.agg_collect_nr)
                from TracesCoveredByTestCases cct
                where cct.product_id = ct.product_id
            )
            and ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $3
            -- satisfy trace covered by same test case
            and exists (
                select rti.req_id
                from TracesCoveredByTestCases cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.agg_collect_nr = rti.agg_collect_nr
                and rt.product_id = rti.product_id
                and rt.req_collect_nr = rti.req_collect_nr
                and rt.req_id = rti.req_id
                -- same test case
                and ct.agg_collect_nr = cti.agg_collect_nr
                and ct.product_id = cti.product_id
                and ct.test_collect_nr = cti.test_collect_nr
                and ct.test_run_name = cti.test_run_name
                and ct.test_run_date = cti.test_run_date
                and ct.test_case_name = cti.test_case_name
                -- satisfy trace
                and ti.kind = $2
            )
        ), ReqsWithTestsCoveringSatisfyAndVerifyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select req_collect_nr, product_id, req_id
            from ReqsWithTestRunsCoveringSatisfyAndVerifyTraces
            union
            select req_collect_nr, product_id, req_id
            from ReqsWithTestCasesCoveringSatisfyAndVerifyTraces
        ), ReqsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from TracesCoveredByTestRuns ct, Traces t, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and ct.agg_collect_nr = (
                select max(cct.agg_collect_nr)
                from TracesCoveredByTestRuns cct
                where cct.product_id = ct.product_id
            )
            and ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $3
            -- no satisfy trace exists for same test run
            and not exists (
                select rti.req_id
                from TracesCoveredByTestRuns cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.agg_collect_nr = rti.agg_collect_nr
                and rt.product_id = rti.product_id
                and rt.req_collect_nr = rti.req_collect_nr
                and rt.req_id = rti.req_id
                -- same test run
                and ct.agg_collect_nr = cti.agg_collect_nr
                and ct.product_id = cti.product_id
                and ct.test_collect_nr = cti.test_collect_nr
                and ct.test_run_name = cti.test_run_name
                and ct.test_run_date = cti.test_run_date
                -- satisfy trace
                and ti.kind = $2
            )
        ), ReqsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select distinct rt.req_collect_nr, rt.product_id, rt.req_id
            from TracesCoveredByTestCases ct, Traces t, DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
            and ct.agg_collect_nr = (
                select max(cct.agg_collect_nr)
                from TracesCoveredByTestCases cct
                where cct.product_id = ct.product_id
            )
            and ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $3
            -- no satisfy trace exists for same test case
            and not exists (
                select rti.req_id
                from TracesCoveredByTestCases cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.agg_collect_nr = rti.agg_collect_nr
                and rt.product_id = rti.product_id
                and rt.req_collect_nr = rti.req_collect_nr
                and rt.req_id = rti.req_id
                -- same test case
                and ct.agg_collect_nr = cti.agg_collect_nr
                and ct.product_id = cti.product_id
                and ct.test_collect_nr = cti.test_collect_nr
                and ct.test_run_name = cti.test_run_name
                and ct.test_run_date = cti.test_run_date
                and ct.test_case_name = cti.test_case_name
                -- satisfy trace
                and ti.kind = $2
            )
        ), ReqsWithTestsCoveringVerifyTracesButNoSatisfyTraces (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            select req_collect_nr, product_id, req_id
            from ReqsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces
            union
            select req_collect_nr, product_id, req_id
            from ReqsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces
        ), TracedReqStates (
            req_collect_nr,
            product_id,
            req_id,
            state
        ) as (
            select
                rt.req_collect_nr,
                rt.product_id,
                rt.req_id,
                case
                    when exists (
                        select f.req_id
                        from ReqsCoveredByFailedTests f
                        where rt.req_collect_nr = f.req_collect_nr
                        and rt.product_id = f.product_id
                        and rt.req_id = f.req_id
                    ) then $5
                    when exists (
                        select p.req_id
                        from ReqsOnlyCoveredByPassedTests p
                        where rt.req_collect_nr = p.req_collect_nr
                        and rt.product_id = p.product_id
                        and rt.req_id = p.req_id
                    ) then case
                        when exists (
                            select vt.req_id
                            from NoSatisfyTraceButVerifyTraces vt, ReqsWithTestsCoveringVerifyTracesButNoSatisfyTraces ct
                            where rt.req_collect_nr = vt.req_collect_nr
                            and rt.product_id = vt.product_id
                            and rt.req_id = vt.req_id
                            and rt.req_collect_nr = ct.req_collect_nr
                            and rt.product_id = ct.product_id
                            and rt.req_id = ct.req_id
                        ) then $4
                        when exists (
                            select svt.req_id
                            from SatisfyAndVerifyTraceExists svt, ReqsWithTestsCoveringSatisfyAndVerifyTraces ct
                            where rt.req_collect_nr = svt.req_collect_nr
                            and rt.product_id = svt.product_id
                            and rt.req_id = svt.req_id
                            and rt.req_collect_nr = ct.req_collect_nr
                            and rt.product_id = ct.product_id
                            and rt.req_id = ct.req_id
                            and not exists (
                                select bt.req_id
                                from ReqsWithTestsCoveringVerifyTracesButNoSatisfyTraces bt
                                where bt.req_collect_nr = ct.req_collect_nr
                                and bt.product_id = ct.product_id
                                and bt.req_id = ct.req_id
                            )
                        ) then $4
                        -- verified conditions not meet, even though tests passed => either satisfied or unknown
                        when exists (
                            select ds.req_id
                            from DirectlySatisfiedRequirements ds
                            where ds.agg_collect_nr = (
                                select max(cds.agg_collect_nr)
                                from DirectlySatisfiedRequirements cds
                                where cds.product_id = ds.product_id
                            )
                            and rt.req_collect_nr = ds.req_collect_nr
                            and rt.product_id = ds.product_id
                            and rt.req_id = ds.req_id
                        ) then $7
                        -- unknown
                        else $8
                        end
                    -- covered by skipped test might be due to test case being overridden by a review,
                    -- or via static analysis mapping coverage to tests at compile time.
                    when exists (
                        select s.req_id
                        from ReqsCoveredBySkippedTests s
                        where rt.req_collect_nr = s.req_collect_nr
                        and rt.product_id = s.product_id
                        and rt.req_id = s.req_id
                    ) then $6
                    -- neither failed, skipped, nor passed => unknown
                    else $8
                    end
            from DirectProductReqTraces rt
            where rt.agg_collect_nr = (
                select max(crt.agg_collect_nr)
                from DirectProductReqTraces crt
                where crt.product_id = rt.product_id
            )
        ), ReqsExplicitlyVerifiedByFailedOrUnknownTestCases (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            -- Note: ResolvedTestCaseStates already filters on valid test runs
            select distinct vr.req_collect_nr, vr.product_id, vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where tc.agg_collect_nr = (
                select max(ctc.agg_collect_nr)
                from ResolvedTestCaseStates ctc
                where ctc.product_id = tc.product_id
            )
            and vr.product_id = tc.product_id
            and vr.collect_nr = tc.test_collect_nr
            and vr.test_run_name = tc.test_run_name
            and vr.test_run_date = tc.test_run_date
            and vr.test_case_name = tc.test_case_name
            -- neither passed nor skipped => failed or unknown
            and tc.state != $9 and tc.state != $10
        ), ReqsExplicitlyVerifiedBySkippedTestCases (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            -- Note: ResolvedTestCaseStates already filters on valid test runs
            select distinct vr.req_collect_nr, vr.product_id, vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where tc.agg_collect_nr = (
                select max(ctc.agg_collect_nr)
                from ResolvedTestCaseStates ctc
                where ctc.product_id = tc.product_id
            )
            and vr.product_id = tc.product_id
            and vr.collect_nr = tc.test_collect_nr
            and vr.test_run_name = tc.test_run_name
            and vr.test_run_date = tc.test_run_date
            and vr.test_case_name = tc.test_case_name
            and tc.state = $10
            except
            select req_collect_nr, product_id, req_id
            from ReqsExplicitlyVerifiedByFailedOrUnknownTestCases
        ), ReqsExplicitlyVerifiedOnlyByPassedTestCases (
            req_collect_nr,
            product_id,
            req_id
        ) as (
            -- Note: ResolvedTestCaseStates already filters on valid test runs
            select distinct vr.req_collect_nr, vr.product_id, vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where tc.agg_collect_nr = (
                select max(ctc.agg_collect_nr)
                from ResolvedTestCaseStates ctc
                where ctc.product_id = tc.product_id
            )
            and vr.product_id = tc.product_id
            and vr.collect_nr = tc.test_collect_nr
            and vr.test_run_name = tc.test_run_name
            and vr.test_run_date = tc.test_run_date
            and vr.test_case_name = tc.test_case_name
            and tc.state = $9
            except
            select req_collect_nr, product_id, req_id
            from (
                select req_collect_nr, product_id, req_id
                from ReqsExplicitlyVerifiedByFailedOrUnknownTestCases
                union all
                select req_collect_nr, product_id, req_id
                from ReqsExplicitlyVerifiedBySkippedTestCases
            )
        ), ReqsExplicitlyVerifiedByTestCasesState (
            req_collect_nr,
            product_id,
            req_id,
            state
        ) as (
            select
                vr.req_collect_nr,
                vr.product_id,
                vr.req_id,
                case
                    -- passed -> verified
                    when exists (
                        select p.req_id
                        from ReqsExplicitlyVerifiedOnlyByPassedTestCases p
                        where vr.req_collect_nr = p.req_collect_nr
                        and vr.product_id = p.product_id
                        and vr.req_id = p.req_id
                    ) then $4
                    -- skipped
                    when exists (
                        select s.req_id
                        from ReqsExplicitlyVerifiedBySkippedTestCases s
                        where vr.req_collect_nr = s.req_collect_nr
                        and vr.product_id = s.product_id
                        and vr.req_id = s.req_id
                    ) then $6
                    -- failed/unknown -> failed
                    else $5
                end
            from (
                select distinct vri.req_collect_nr, vri.product_id, vri.req_id
                from TestCaseVerifiedRequirements vri, ValidTestRuns vt
                where vt.agg_collect_nr = (
                    select max(vti.agg_collect_nr)
                    from ValidTestRuns vti
                    where vti.product_id = vt.product_id
                )
                and vri.collect_nr = vt.test_collect_nr
                and vri.product_id = vt.product_id
                and vri.test_run_name = vt.test_run_name
                and vri.test_run_date = vt.test_run_date
            ) vr
        ), ManualReqStates (
            req_collect_nr,
            product_id,
            req_id,
            state
        ) as (
            select
                mr.req_collect_nr,
                mr.product_id,
                mr.req_id,
                case
                    -- related test failed
                    when exists (
                        select ts.req_id
                        from TracedReqStates ts
                        where mr.req_collect_nr = ts.req_collect_nr
                        and mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $5
                    ) or exists (
                        select ts.req_id
                        from ReqsExplicitlyVerifiedByTestCasesState ts
                        where mr.req_collect_nr = ts.req_collect_nr
                        and mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $5
                    ) then $5
                    -- manual valid review exists
                    when exists (
                        select mvr.req_id
                        from ManuallyVerifiedRequirements mvr, ValidReviews vr
                        where vr.agg_collect_nr = (
                            select max(cvr.agg_collect_nr)
                            from ValidReviews cvr
                            where cvr.product_id = vr.product_id
                        )
                        and mvr.collect_nr = vr.review_collect_nr
                        and mvr.product_id = vr.product_id
                        and mvr.review_name = vr.review_name
                        and mvr.review_date = vr.review_date
                        and mr.req_collect_nr = mvr.req_collect_nr
                        and mr.product_id = mvr.product_id
                        and mr.req_id = mvr.req_id
                    ) then case
                        -- explicit verify test exists, but is skipped
                        when exists (
                            select ts.req_id
                            from ReqsExplicitlyVerifiedByTestCasesState ts
                            where mr.req_collect_nr = ts.req_collect_nr
                            and mr.product_id = ts.product_id
                            and mr.req_id = ts.req_id
                            and ts.state = $6
                        ) then $6
                        -- verify trace exists, but wasn't verified => partially verified due to review
                        when exists (
                            select ts.req_id
                            from TracedReqStates ts, VerifyTraceExists vt
                            where mr.req_collect_nr = ts.req_collect_nr
                            and mr.product_id = ts.product_id
                            and mr.req_id = ts.req_id
                            and mr.req_collect_nr = vt.req_collect_nr
                            and mr.product_id = vt.product_id
                            and mr.req_id = vt.req_id
                            and ts.state != $4
                        ) then $11
                        -- verified
                        else $4
                        end
                    -- no manual review, but passed test => partially verified
                    when exists (
                        select ts.req_id
                        from TracedReqStates ts
                        where mr.req_collect_nr = ts.req_collect_nr
                        and mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $4
                    ) or exists (
                        select ts.req_id
                        from ReqsExplicitlyVerifiedByTestCasesState ts
                        where mr.req_collect_nr = ts.req_collect_nr
                        and mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $4
                    ) then $11
                    -- unknown
                    else $8
                end
            from UsableManualRequirements mr
            where mr.agg_collect_nr = (
                select max(mri.agg_collect_nr)
                from UsableManualRequirements mri
                where mri.product_id = mr.product_id
            )
        )
        select
            $1 as agg_collect_nr,
            r.req_collect_nr,
            r.product_id,
            r.req_id,
            case
                -- manual states take precedence
                when mr.state not null then mr.state
                -- failing through covered trace
                when exists (
                    select ts.req_id
                    from TracedReqStates ts
                    where r.req_collect_nr = ts.req_collect_nr
                    and r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $5
                )
                -- failing through direct test
                or exists (
                    select ts.req_id
                    from ReqsExplicitlyVerifiedByTestCasesState ts
                    where r.req_collect_nr = ts.req_collect_nr
                    and r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $5
                ) then $5
                -- skipped through covered trace
                when exists (
                    select ts.req_id
                    from TracedReqStates ts
                    where r.req_collect_nr = ts.req_collect_nr
                    and r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $6
                )
                -- skipped through direct test
                or exists (
                    select ts.req_id
                    from ReqsExplicitlyVerifiedByTestCasesState ts
                    where r.req_collect_nr = ts.req_collect_nr
                    and r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $6
                ) then $6
                -- verified through covered trace
                when exists (
                    select ts.req_id
                    from TracedReqStates ts
                    where r.req_collect_nr = ts.req_collect_nr
                    and r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $4
                )
                -- verified through direct test
                or exists (
                    select ts.req_id
                    from ReqsExplicitlyVerifiedByTestCasesState ts
                    where r.req_collect_nr = ts.req_collect_nr
                    and r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $4
                ) then $4
                -- unknown
                else $8
            end
        from UsableRequirements r
            left join ManualReqStates mr on
                r.req_collect_nr = mr.req_collect_nr
                and r.product_id = mr.product_id
                and r.req_id = mr.req_id
        where r.agg_collect_nr = (
            select max(ri.agg_collect_nr)
            from UsableRequirements ri
            where ri.product_id = r.product_id
        )
        ",
        collect_nr,
        satisfies_trace_nr,
        verifies_trace_nr,
        req_verified_nr,
        req_failed_nr,
        req_skipped_nr,
        req_satisfied_nr,
        req_unknown_nr,
        test_case_passed_nr,
        test_case_skipped_nr,
        req_partial_verified_nr
     )
     .execute(self.connection_mut())
     .await?;

        Ok(())
    }
}
