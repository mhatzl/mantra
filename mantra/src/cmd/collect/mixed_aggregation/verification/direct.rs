use anyhow::Context;
use mantra_schema::{
    annotations::TraceKind, report::requirement::RequirementState, test_runs::TestCaseState,
};

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(super) async fn update_direct_req_verification_states(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let satisfies_trace_number = TraceKind::Satisfies.as_number();
        let verifies_trace_number = TraceKind::Verifies.as_number();
        let req_verified_number = RequirementState::Verified.as_number();
        let req_failed_number = RequirementState::Failed.as_number();
        let req_skipped_number = RequirementState::Skipped.as_number();
        let req_satisfied_number = RequirementState::Satisfied.as_number();
        let req_unknown_number = RequirementState::Unknown.as_number();
        let req_partial_verified_number = RequirementState::PartiallyVerified.as_number();
        let test_case_passed_number = TestCaseState::Passed.as_number();
        let test_case_skipped_number = TestCaseState::Skipped.as_number();

        sqlx::query!(
            "
            delete from RequirementsWithSatisfyTraces
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementsWithSatisfyTraces")?;

        sqlx::query!(
            "
            insert into RequirementsWithSatisfyTraces (
                product_id,
                req_id,
                file_hash,
                trace_line
            )
            select distinct rt.product_id, rt.req_id, t.file_hash, t.line
            from Traces t, DirectProductReqTraces rt
            where rt.file_hash = t.file_hash
            and rt.line = t.line
            and t.kind = $1
            ",
            satisfies_trace_number
        )
        .execute(self.connection_mut())
        .await
        .context("Updating RequirementsWithSatisfyTraces")?;

        sqlx::query!(
            "
            delete from RequirementsWithVerifyTraces
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementsWithVerifyTraces")?;

        sqlx::query!(
            "
            insert into RequirementsWithVerifyTraces (
                product_id,
                req_id,
                file_hash,
                trace_line
            )
            select distinct rt.product_id, rt.req_id, t.file_hash, t.line
            from Traces t, DirectProductReqTraces rt
            where rt.file_hash = t.file_hash
            and rt.line = t.line
            and t.kind = $1
            ",
            verifies_trace_number
        )
        .execute(self.connection_mut())
        .await
        .context("Updating RequirementsWithVerifyTraces")?;

        sqlx::query!(
            "
            delete from RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces")?;

        sqlx::query!(
            "
            insert into RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces (
                product_id,
                req_id,
                test_run_name
            )
            select distinct rt.product_id, rt.req_id, ct.test_run_name
            from TracesCoveredByTestRuns ct, Traces t, DirectProductReqTraces rt
            where ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $2
            -- satisfy trace covered by same test run
            and exists (
                select rti.req_id
                from TracesCoveredByTestRuns cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.product_id = rti.product_id
                and rt.req_id = rti.req_id
                -- same test run
                and ct.product_id = cti.product_id
                and ct.test_run_name = cti.test_run_name
                -- satisfy trace
                and ti.kind = $1
            )
            ",
            satisfies_trace_number,
            verifies_trace_number
        )
        .execute(self.connection_mut())
        .await
        .context("Updating RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces")?;

        sqlx::query!(
            "
            delete from RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces")?;

        sqlx::query!(
            "
            insert into RequirementsWithTestCasesCoveringSatisfyAndVerifyTraces (
                product_id,
                req_id,
                test_run_name,
                test_case_name
            )
            select distinct rt.product_id, rt.req_id, ct.test_run_name, ct.test_case_name
            from TracesCoveredByTestCases ct, Traces t, DirectProductReqTraces rt
            where ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $2
            -- satisfy trace covered by same test case
            and exists (
                select rti.req_id
                from TracesCoveredByTestCases cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.product_id = rti.product_id
                and rt.req_id = rti.req_id
                -- same test case
                and ct.product_id = cti.product_id
                and ct.test_run_name = cti.test_run_name
                and ct.test_case_name = cti.test_case_name
                -- satisfy trace
                and ti.kind = $1
            )
            ",
            satisfies_trace_number,
            verifies_trace_number
        )
        .execute(self.connection_mut())
        .await
        .context("Updating RequirementsWithTestCasesCoveringSatisfyAndVerifyTraces")?;

        sqlx::query!(
            "
            delete from RequirementsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces")?;

        sqlx::query!(
            "
            insert into RequirementsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces (
                product_id,
                req_id,
                test_run_name
            )
            select distinct rt.product_id, rt.req_id, ct.test_run_name
            from TracesCoveredByTestRuns ct, Traces t, DirectProductReqTraces rt
            where ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $2
            -- no satisfy trace exists for same test run
            and not exists (
                select rti.req_id
                from TracesCoveredByTestRuns cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.product_id = rti.product_id
                and rt.req_id = rti.req_id
                -- same test run
                and ct.product_id = cti.product_id
                and ct.test_run_name = cti.test_run_name
                -- satisfy trace
                and ti.kind = $1
            )
            ",
            satisfies_trace_number,
            verifies_trace_number
        )
        .execute(self.connection_mut())
        .await
        .context("Updating RequirementsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces")?;

        sqlx::query!(
            "
            delete from RequirementsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces")?;

        sqlx::query!(
            "
            insert into RequirementsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces (
                product_id,
                req_id,
                test_run_name,
                test_case_name
            )
            select distinct rt.product_id, rt.req_id, ct.test_run_name, ct.test_case_name
            from TracesCoveredByTestCases ct, Traces t, DirectProductReqTraces rt
            where ct.product_id = rt.product_id
            and ct.file_hash = rt.file_hash and ct.traced_line = rt.line
            and ct.file_hash = t.file_hash and ct.traced_line = t.line
            and ct.filepath = rt.filepath
            -- verify trace
            and t.kind = $2
            -- no satisfy trace exists for same test case
            and not exists (
                select rti.req_id
                from TracesCoveredByTestCases cti, Traces ti, DirectProductReqTraces rti
                where cti.product_id = rti.product_id
                and cti.file_hash = rti.file_hash and cti.traced_line = rti.line
                and cti.file_hash = ti.file_hash and cti.traced_line = ti.line
                and cti.filepath = rti.filepath
                -- same req
                and rt.product_id = rti.product_id
                and rt.req_id = rti.req_id
                -- same test case
                and ct.product_id = cti.product_id
                and ct.test_run_name = cti.test_run_name
                and ct.test_case_name = cti.test_case_name
                -- satisfy trace
                and ti.kind = $1
            )
            ",
            satisfies_trace_number,
            verifies_trace_number
        )
        .execute(self.connection_mut())
        .await
        .context("Updating RequirementsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces")?;

        sqlx::query!(
            "
            delete from DirectRequirementVerificationStates
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing DirectRequirementVerificationStates")?;

        sqlx::query!(
        "
        insert into DirectRequirementVerificationStates (
            product_id,
            req_id,
            state
        )
        with SatisfyTraceExists (
            product_id,
            req_id
        ) as (
            select distinct product_id, req_id
            from RequirementsWithSatisfyTraces
        ), VerifyTraceExists (
            product_id,
            req_id
        ) as (
            select distinct product_id, req_id
            from RequirementsWithVerifyTraces
        ), ReqsCoveredByPassedAndNoFailedOrUnknownTests (
            product_id,
            req_id
        ) as (
            -- skipped tests are fine as long as there are other passing tests
            select distinct rt.product_id, rt.req_id
            from TracesCoveredByPassedAndNoFailedOrUnknownTests ct, DirectProductReqTraces rt
            where ct.product_id = rt.product_id
            and ct.filepath = rt.filepath
            and ct.file_hash = rt.file_hash
            and ct.traced_line = rt.line
        ), ReqsCoveredByFailedTests (
            product_id,
            req_id
        ) as (
            select distinct rt.product_id, rt.req_id
            from TracesCoveredByFailedTests ct, DirectProductReqTraces rt
            where ct.product_id = rt.product_id
            and ct.filepath = rt.filepath
            and ct.file_hash = rt.file_hash
            and ct.traced_line = rt.line
        ), ReqsCoveredBySkippedTests (
            product_id,
            req_id
        ) as (
            select distinct rt.product_id, rt.req_id
            from TracesCoveredBySkippedTests ct, DirectProductReqTraces rt
            where ct.product_id = rt.product_id
            and ct.filepath = rt.filepath
            and ct.file_hash = rt.file_hash
            and ct.traced_line = rt.line
        ), ReqsWithTestRunsCoveringSatisfyAndVerifyTraces (
            product_id,
            req_id
        ) as (
            select distinct product_id, req_id
            from RequirementsWithTestRunsCoveringSatisfyAndVerifyTraces
        ), ReqsWithTestCasesCoveringSatisfyAndVerifyTraces (
            product_id,
            req_id
        ) as (
            select distinct product_id, req_id
            from RequirementsWithTestCasesCoveringSatisfyAndVerifyTraces
        ), ReqsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces (
            product_id,
            req_id
        ) as (
            select distinct product_id, req_id
            from RequirementsWithTestRunsCoveringVerifyTracesButNoSatisfyTraces
        ), ReqsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces (
            product_id,
            req_id
        ) as (
            select distinct product_id, req_id
            from RequirementsWithTestCasesCoveringVerifyTracesButNoSatisfyTraces
        ), TracedReqStates (
            product_id,
            req_id,
            state
        ) as (
            select
                rt.product_id,
                rt.req_id,
                case
                    when exists (
                        select f.req_id
                        from ReqsCoveredByFailedTests f
                        where rt.product_id = f.product_id
                        and rt.req_id = f.req_id
                    ) then $4
                    when exists (
                        select p.req_id
                        from ReqsCoveredByPassedAndNoFailedOrUnknownTests p
                        where rt.product_id = p.product_id
                        and rt.req_id = p.req_id
                    ) then case
                        when exists (
                            select vt.req_id
                            from RequirementsWithVerifyButNoSatisfyTraces vt, RequirementsWithTestsCoveringVerifyTracesButNoSatisfyTraces ct
                            where rt.product_id = vt.product_id
                            and rt.req_id = vt.req_id
                            and rt.product_id = ct.product_id
                            and rt.req_id = ct.req_id
                        ) or exists (
                            select svt.req_id
                            from RequirementsWithSatisfyAndVerifyTraces svt, RequirementsWithTestsCoveringSatisfyAndVerifyTraces ct
                            where rt.product_id = svt.product_id
                            and rt.req_id = svt.req_id
                            and rt.product_id = ct.product_id
                            and rt.req_id = ct.req_id
                            and not exists (
                                select bt.req_id
                                from RequirementsWithTestsCoveringVerifyTracesButNoSatisfyTraces bt
                                where bt.product_id = ct.product_id
                                and bt.req_id = ct.req_id
                            )
                        ) then $1
                        -- verified conditions not meet, even though tests passed => either satisfied or unknown
                        when exists (
                            select ds.req_id
                            from DirectlySatisfiedRequirements ds
                            where rt.product_id = ds.product_id
                            and rt.req_id = ds.req_id
                        ) then $4
                        -- unknown
                        else $5
                        end
                    -- covered by skipped test might be due to test case being overridden by a review,
                    -- or via static analysis mapping coverage to tests at compile time.
                    when exists (
                        select s.req_id
                        from ReqsCoveredBySkippedTests s
                        where rt.product_id = s.product_id
                        and rt.req_id = s.req_id
                    ) then $3
                    -- neither failed, skipped, nor passed => unknown
                    else $5
                    end
            from DirectProductReqTraces rt
        ), ReqsExplicitlyVerifiedByFailedOrUnknownTestCases (
            product_id,
            req_id
        ) as (
            -- Note: ResolvedTestCaseStates already filters on valid test runs
            select distinct vr.product_id, vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where vr.product_id = tc.product_id
            and vr.test_run_name = tc.test_run_name
            and vr.test_case_name = tc.test_case_name
            -- neither passed nor skipped => failed or unknown
            and tc.state != $6 and tc.state != $7
        ), ReqsExplicitlyVerifiedBySkippedTestCases (
            product_id,
            req_id
        ) as (
            -- Note: ResolvedTestCaseStates already filters on valid test runs
            select distinct vr.product_id, vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where vr.product_id = tc.product_id
            and vr.test_run_name = tc.test_run_name
            and vr.test_case_name = tc.test_case_name
            and tc.state = $7

            except

            select product_id, req_id
            from ReqsExplicitlyVerifiedByFailedOrUnknownTestCases
        ), ReqsExplicitlyVerifiedOnlyByPassedTestCases (
            product_id,
            req_id
        ) as (
            -- Note: ResolvedTestCaseStates already filters on valid test runs
            select distinct vr.product_id, vr.req_id
            from TestCaseVerifiedRequirements vr, ResolvedTestCaseStates tc
            where vr.product_id = tc.product_id
            and vr.test_run_name = tc.test_run_name
            and vr.test_case_name = tc.test_case_name
            and tc.state = $6

            except

            select product_id, req_id
            from (
                select product_id, req_id
                from ReqsExplicitlyVerifiedByFailedOrUnknownTestCases

                union all

                select product_id, req_id
                from ReqsExplicitlyVerifiedBySkippedTestCases
            )
        ), ReqsExplicitlyVerifiedByTestCasesState (
            product_id,
            req_id,
            state
        ) as (
            select
                vr.product_id,
                vr.req_id,
                case
                    -- passed -> verified
                    when exists (
                        select p.req_id
                        from ReqsExplicitlyVerifiedOnlyByPassedTestCases p
                        where vr.product_id = p.product_id
                        and vr.req_id = p.req_id
                    ) then $1
                    -- skipped
                    when exists (
                        select s.req_id
                        from ReqsExplicitlyVerifiedBySkippedTestCases s
                        where vr.product_id = s.product_id
                        and vr.req_id = s.req_id
                    ) then $3
                    -- failed/unknown -> failed
                    else $2
                end
            from (
                select distinct vri.product_id, vri.req_id
                from TestCaseVerifiedRequirements vri, ValidTestRuns vt
                where vri.product_id = vt.product_id
                and vri.test_run_name = vt.test_run_name
            ) vr
        ), ManualReqStates (
            product_id,
            req_id,
            state
        ) as (
            select
                mr.product_id,
                mr.req_id,
                case
                    -- related test failed
                    when exists (
                        select ts.req_id
                        from TracedReqStates ts
                        where mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $2
                    ) or exists (
                        select ts.req_id
                        from ReqsExplicitlyVerifiedByTestCasesState ts
                        where mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $2
                    ) then $2
                    -- manual valid review exists
                    when exists (
                        select mvr.req_id
                        from ManuallyVerifiedRequirements mvr, ValidReviews vr
                        where mvr.product_id = vr.product_id
                        and mvr.review_name = vr.review_name
                        and mr.product_id = mvr.product_id
                        and mr.req_id = mvr.req_id
                    ) then case
                        -- explicit verify test exists, but is skipped
                        when exists (
                            select ts.req_id
                            from ReqsExplicitlyVerifiedByTestCasesState ts
                            where mr.product_id = ts.product_id
                            and mr.req_id = ts.req_id
                            and ts.state = $3
                        ) then $3
                        -- verify trace exists, but wasn't verified => partially verified due to review
                        when exists (
                            select ts.req_id
                            from TracedReqStates ts, VerifyTraceExists vt
                            where mr.product_id = ts.product_id
                            and mr.req_id = ts.req_id
                            and mr.product_id = vt.product_id
                            and mr.req_id = vt.req_id
                            and ts.state != $1
                        ) then $8
                        -- verified
                        else $1
                        end
                    -- no manual review, but passed test => partially verified
                    when exists (
                        select ts.req_id
                        from TracedReqStates ts
                        where mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $1
                    ) or exists (
                        select ts.req_id
                        from ReqsExplicitlyVerifiedByTestCasesState ts
                        where mr.product_id = ts.product_id
                        and mr.req_id = ts.req_id
                        and ts.state = $1
                    ) then $8
                    -- unknown
                    else $5
                end
            from UsableManualRequirements mr
        )
        select
            r.product_id,
            r.req_id,
            case
                -- manual states take precedence
                when mr.state not null then mr.state
                -- failing through covered trace
                when exists (
                    select ts.req_id
                    from TracedReqStates ts
                    where r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $2
                )
                -- failing through direct test
                or exists (
                    select ts.req_id
                    from ReqsExplicitlyVerifiedByTestCasesState ts
                    where r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $2
                ) then $2
                -- skipped through covered trace
                when exists (
                    select ts.req_id
                    from TracedReqStates ts
                    where r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $3
                )
                -- skipped through direct test
                or exists (
                    select ts.req_id
                    from ReqsExplicitlyVerifiedByTestCasesState ts
                    where r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $3
                ) then $3
                -- verified through covered trace
                when exists (
                    select ts.req_id
                    from TracedReqStates ts
                    where r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $1
                )
                -- verified through direct test
                or exists (
                    select ts.req_id
                    from ReqsExplicitlyVerifiedByTestCasesState ts
                    where r.product_id = ts.product_id
                    and r.req_id = ts.req_id
                    and ts.state = $1
                ) then $1
                -- satisfied
                when exists (
                    select ds.req_id
                    from DirectlySatisfiedRequirements ds
                    where r.product_id = ds.product_id
                    and r.req_id = ds.req_id
                ) then $4
                -- unknown
                else $5
            end
        from UsableRequirements r
            left join ManualReqStates mr
                on r.product_id = mr.product_id
                and r.req_id = mr.req_id
        ",
        req_verified_number,
        req_failed_number,
        req_skipped_number,
        req_satisfied_number,
        req_unknown_number,
        test_case_passed_number,
        test_case_skipped_number,
        req_partial_verified_number
     )
     .execute(self.connection_mut())
     .await?;

        Ok(())
    }
}
