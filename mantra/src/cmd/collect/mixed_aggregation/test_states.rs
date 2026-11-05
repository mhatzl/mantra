use anyhow::Context;
use mantra_schema::{
    report::tests::{ResolvedCoverageState, TestState},
    test_runs::TestCaseState,
};

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(super) async fn resolve_tests(&mut self) -> Result<(), anyhow::Error> {
        self.resolve_test_case_states()
            .await
            .context("Failed to resolve test case states")?;
        self.update_usable_test_cases()
            .await
            .context("Failed to update usable test cases")?;
        self.resolve_line_coverage()
            .await
            .context("Failed to resolve line coverage")?;
        self.update_failed_test_runs()
            .await
            .context("Failed to update failed test runs")?;
        self.update_skipped_test_runs()
            .await
            .context("Failed to update skipped test runs")?;
        self.update_passed_test_runs()
            .await
            .context("Failed to update passed test runs")?;
        self.update_usable_test_runs()
            .await
            .context("Failed to update usable test runs")?;
        self.resolve_line_coverage_test_states()
            .await
            .context("Resolving covered line test states")?;
        self.update_test_run_trace_coverage()
            .await
            .context("Failed to update test run trace coverage")?;
        self.update_test_case_trace_coverage()
            .await
            .context("Failed to update test case trace coverage")?;

        Ok(())
    }

    async fn resolve_test_case_states(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from ResolvedTestCaseStates
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing ResolvedTestCaseStates")?;

        let obsolete = TestState::Obsolete.as_number();

        sqlx::query!(
            "
            with ValidTestCaseOverrides (
                product_id,
                test_run_name,
                test_case_name,
                state
            ) as (
                select
                    tco.product_id,
                    tco.test_run_name,
                    tco.test_case_name,
                    tco.state
                from TestCaseOverrides tco, ValidReviews vr
                where vr.product_id = tco.product_id
                and vr.review_name = tco.review_name
            )
            insert into ResolvedTestCaseStates (
                product_id,
                test_run_name,
                test_case_name,
                state
            )
            select
                o.product_id,
                o.test_run_name,
                o.test_case_name,
                o.state
            from ValidTestCaseOverrides o, ValidTestRuns vt
            where o.product_id = vt.product_id
            and o.test_run_name = vt.test_run_name

            union all

            select
                product_id,
                test_run_name,
                test_case_name,
                state
            from (
                -- valid non-overridden
                select
                    t.product_id,
                    t.test_run_name,
                    t.name as test_case_name,
                    t.state
                from TestCases t, ValidTestRuns vt
                where t.product_id = vt.product_id
                and t.test_run_name = vt.test_run_name
                and not exists (
                    select
                        o.test_case_name
                    from ValidTestCaseOverrides o
                    where t.product_id = o.product_id
                    and t.test_run_name = o.test_run_name
                    and t.name = o.test_case_name
                )

                union all

                -- obsolete
                select
                    t.product_id,
                    t.test_run_name,
                    t.name as test_case_name,
                    $1
                from TestCases t, ObsoleteTestRuns ot
                where t.product_id = ot.product_id
                and t.test_run_name = ot.test_run_name
            )
            ",
            obsolete
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_test_cases(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from UsableTestCases
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing UsableTestCases")?;

        sqlx::query!(
            "
            insert into UsableTestCases (
                product_id,
                test_run_name,
                test_case_name
            )
            select
                pc.product_id,
                pc.test_run_name,
                pc.test_case_name
            from PassedTestCases pc, ValidTestRuns vtr
            where pc.product_id = vtr.product_id
            and pc.test_run_name = vtr.test_run_name
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn resolve_line_coverage(&mut self) -> Result<(), anyhow::Error> {
        let collection_id = self.collect_nr();

        sqlx::query!(
            "
            delete from ResolvedTestRunLineCoverage
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing ResolvedTestRunLineCoverage")?;

        let uncovered_state = ResolvedCoverageState::Uncovered.as_number();
        let covered_state = ResolvedCoverageState::Covered.as_number();
        let excluded_state = ResolvedCoverageState::Excluded.as_number();
        let unknown_state = ResolvedCoverageState::Unknown.as_number();
        let overridden_uncovered_state = ResolvedCoverageState::OverriddenUncovered.as_number();
        let overridden_covered_state = ResolvedCoverageState::OverriddenCovered.as_number();
        let overridden_excluded_state = ResolvedCoverageState::OverriddenExcluded.as_number();

        sqlx::query!(
            "
            insert into ResolvedTestRunLineCoverage (
                product_id,
                test_run_name,
                cov_filepath,
                cov_file_hash,
                cov_line,
                cov_state
            )
            select
                tro.product_id,
                tro.test_run_name,
                tro.cov_filepath,
                cf.file_hash as cov_file_hash,
                tro.cov_line,
                case
                    when tro.covered is null then $7
                    when tro.covered = true then $6
                    else $5
                end as cov_state
            from TestRunLineCoverageOverrides tro
                left join CollectedFiles cf on
                    tro.cov_filepath = cf.filepath,
                ValidReviews vr
            where vr.product_id = tro.product_id
            and vr.review_name = tro.review_name

            union all

            select
                t.product_id,
                t.test_run_name,
                t.cov_filepath,
                cf.file_hash as cov_file_hash,
                t.cov_line,
                case
                    when exists (
                        select el.line
                        from ExcludedCoverageLines el
                        where cf.file_hash is not null
                        and cf.file_hash = el.file_hash
                        and t.cov_line = el.line
                    ) then $3
                    when t.hits is null then $4
                    when t.hits > 0 then $2
                    else $1
                end as cov_state
            from TestRunLineCoverage t
                left join CollectedFiles cf on
                    t.cov_filepath = cf.filepath
            where not exists (
                select
                    cov_line
                from TestRunLineCoverageOverrides tro, ValidReviews vr
                where vr.product_id = tro.product_id
                and vr.review_name = tro.review_name
                and tro.product_id = t.product_id
                and tro.test_run_name = t.test_run_name
                and tro.cov_filepath = t.cov_filepath
                and tro.cov_line = t.cov_line
            )
            ",
            uncovered_state,
            covered_state,
            excluded_state,
            unknown_state,
            overridden_uncovered_state,
            overridden_covered_state,
            overridden_excluded_state,
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update ResolvedTestRunLineCoverage")?;

        sqlx::query!(
            "
            delete from ResolvedTestCaseLineCoverage
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing ResolvedTestCaseLineCoverage")?;

        sqlx::query!(
            "
            insert into ResolvedTestCaseLineCoverage (
                product_id,
                test_run_name,
                test_case_name,
                cov_filepath,
                cov_file_hash,
                cov_line,
                cov_state
            )
            select
                tco.product_id,
                tco.test_run_name,
                tco.test_case_name,
                tco.cov_filepath,
                cf.file_hash as cov_file_hash,
                tco.cov_line,
                case
                    when tco.covered is null then $7
                    when tco.covered = true then $6
                    else $5
                end as cov_state
            from TestCaseLineCoverageOverrides tco
                left join CollectedFiles cf on
                    tco.cov_filepath = cf.filepath,
                ValidReviews vr
            where vr.product_id = tco.product_id
            and vr.review_name = tco.review_name

            union all

            select
                t.product_id,
                t.test_run_name,
                t.test_case_name,
                t.cov_filepath,
                cf.file_hash as cov_file_hash,
                t.cov_line,
                case
                    when exists (
                        select el.line
                        from ExcludedCoverageLines el
                        where cf.file_hash is not null
                        and cf.file_hash = el.file_hash
                        and t.cov_line = el.line
                    ) then $3
                    when t.hits is null then $4
                    when t.hits > 0 then $2
                    else $1
                end as cov_state
            from TestCaseLineCoverage t
                left join CollectedFiles cf on
                    t.cov_filepath = cf.filepath
            where not exists (
                select
                    cov_line
                from TestCaseLineCoverageOverrides tco, ValidReviews vr
                where vr.product_id = tco.product_id
                and vr.review_name = tco.review_name
                and tco.product_id = t.product_id
                and tco.test_run_name = t.test_run_name
                and tco.test_case_name = t.test_case_name
                and tco.cov_filepath = t.cov_filepath
                and tco.cov_line = t.cov_line
            )
            ",
            uncovered_state,
            covered_state,
            excluded_state,
            unknown_state,
            overridden_uncovered_state,
            overridden_covered_state,
            overridden_excluded_state,
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update ResolvedTestCaseLineCoverage")?;

        sqlx::query!(
            "
            insert into ResolvedLineCoverageStates (
                latest_collection,
                product_id,
                cov_filepath,
                cov_file_hash,
                cov_line,
                cov_state
            )
            with CombinedResolvedLineCoverageStates (
                product_id,
                cov_filepath,
                cov_file_hash,
                cov_line,
                cov_state
            ) as (
                select product_id, cov_filepath, cov_file_hash, cov_line, cov_state
                from ResolvedTestRunLineCoverage

                union all

                select product_id, cov_filepath, cov_file_hash, cov_line, cov_state
                from ResolvedTestCaseLineCoverage
            ),
            ResolvedCoveredLines (product_id, cov_filepath, cov_file_hash, cov_line) as (
                select distinct product_id, cov_filepath, cov_file_hash, cov_line
                from CombinedResolvedLineCoverageStates
            )
            select
                $1 as latest_collection,
                rl.product_id,
                rl.cov_filepath,
                rl.cov_file_hash,
                rl.cov_line,
                case
                -- covered line in at least one test wins
                when exists (
                    select cls.cov_line
                    from CombinedResolvedLineCoverageStates cls
                    where cls.product_id = rl.product_id
                    and cls.cov_filepath = rl.cov_filepath
                    and cls.cov_file_hash = rl.cov_file_hash
                    and cls.cov_line = rl.cov_line
                    and cls.cov_state = $3
                ) then $3
                -- excluded is next, because it must have been set explicitly.
                -- either via annotation or review
                when exists (
                    select cls.cov_line
                    from CombinedResolvedLineCoverageStates cls
                    where cls.product_id = rl.product_id
                    and cls.cov_filepath = rl.cov_filepath
                    and cls.cov_file_hash = rl.cov_file_hash
                    and cls.cov_line = rl.cov_line
                    and cls.cov_state = $4
                ) then $4
                -- uncovered is next, because it leaves actual code that was not covered or excluded
                when exists (
                    select cls.cov_line
                    from CombinedResolvedLineCoverageStates cls
                    where cls.product_id = rl.product_id
                    and cls.cov_filepath = rl.cov_filepath
                    and cls.cov_file_hash = rl.cov_file_hash
                    and cls.cov_line = rl.cov_line
                    and cls.cov_state = $2
                ) then $2
                -- anything else is unknown
                else $5
                end as cov_state
            from ResolvedCoveredLines rl
            where true

            on conflict (
                product_id,
                cov_filepath,
                cov_line
            ) do update
            set
                latest_collection = excluded.latest_collection,
                cov_file_hash = excluded.cov_file_hash,
                cov_state = excluded.cov_state
            ",
            collection_id,
            uncovered_state,
            covered_state,
            excluded_state,
            unknown_state,
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update ResolvedLineCoverageStates")?;

        sqlx::query!(
            "
            delete from ResolvedLineCoverageStates
            where latest_collection < $1
            ",
            collection_id
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing ResolvedLineCoverageStates")?;

        Ok(())
    }

    async fn update_failed_test_runs(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from FailedTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing FailedTestRuns")?;

        let passed_test_nr = TestState::Passed.as_number();
        let skipped_test_nr = TestState::Skipped.as_number();
        let obsolete_test_nr = TestState::Obsolete.as_number();

        sqlx::query!(
            "
            insert into FailedTestRuns (
                product_id,
                test_run_name
            )
            with TestRunsWithFailedTestCase (
                product_id,
                test_run_name
            ) as (
                select
                    product_id,
                    test_run_name
                from ResolvedTestCaseStates
                -- neither passed, skipped, or obsolete
                where state != $2 and state != $3 and state != $4
            ), IndirectFailedTestRun (
                product_id,
                test_run_name
            ) as (
                select
                    d.product_id,
                    d.test_run_name
                from TestRunDescendants d, TestRunsWithFailedTestCase f
                where d.product_id = f.product_id
                and d.descendant_test_run_name = f.test_run_name
            )
            select
                product_id,
                test_run_name
            from TestRunsWithFailedTestCase

            union all

            select
                product_id,
                test_run_name
            from IndirectFailedTestRun
            ",
            passed_test_nr,
            skipped_test_nr,
            obsolete_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_skipped_test_runs(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from SkippedTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing SkippedTestRuns")?;

        let skipped_test_nr = TestCaseState::Skipped.as_number();

        sqlx::query!(
            "
            insert into SkippedTestRuns (
                product_id,
                test_run_name
            )
            with TestRunsWithSkippedTestCase (
                product_id,
                test_run_name
            ) as (
                select
                    product_id,
                    test_run_name
                from ResolvedTestCaseStates
                where state = $1
            ), IndirectSkippedTestRun (
                product_id,
                test_run_name
            ) as (
                select
                    d.product_id,
                    d.test_run_name
                from TestRunDescendants d, TestRunsWithSkippedTestCase s
                where d.product_id = s.product_id
                and d.descendant_test_run_name = s.test_run_name
            )
            select
                product_id,
                test_run_name
            from (
                select
                    product_id,
                    test_run_name
                from TestRunsWithSkippedTestCase

                union all

                select
                    product_id,
                    test_run_name
                from IndirectSkippedTestRun
            )

            except

            select
                product_id,
                test_run_name
            from FailedTestRuns
            ",
            skipped_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_passed_test_runs(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from PassedTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing PassedTestRuns")?;

        let passed_test_nr = TestCaseState::Passed.as_number();

        sqlx::query!(
            "
            insert into PassedTestRuns (
                product_id,
                test_run_name
            )
            with TestRunsWithPassedTestCase (
                product_id,
                test_run_name
            ) as (
                select
                    product_id,
                    test_run_name
                from ResolvedTestCaseStates
                where state = $1
            ), IndirectPassedTestRun (
                product_id,
                test_run_name
            ) as (
                select
                    d.product_id,
                    d.test_run_name
                from TestRunDescendants d, TestRunsWithPassedTestCase p
                where d.product_id = p.product_id
                and d.descendant_test_run_name = p.test_run_name
            )
            select
                product_id,
                test_run_name
            from (
                select
                    product_id,
                    test_run_name
                from TestRunsWithPassedTestCase

                union all

                select
                    product_id,
                    test_run_name
                from IndirectPassedTestRun
            )

            except

            select
                product_id,
                test_run_name
            from (
                select
                    product_id,
                    test_run_name
                from FailedTestRuns

                union all

                select
                    product_id,
                    test_run_name
                from SkippedTestRuns
            )
            ",
            passed_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_test_runs(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from UsableTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing UsableTestRuns")?;

        sqlx::query!(
            "
            insert into UsableTestRuns (
                product_id,
                test_run_name
            )
            select
                ptr.product_id,
                ptr.test_run_name
            from PassedTestRuns ptr, ValidTestRuns vtr
            where ptr.product_id = vtr.product_id
            and ptr.test_run_name = vtr.test_run_name
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn resolve_line_coverage_test_states(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from ResolvedLineCoverageTestStates
            ",
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing ResolvedLineCoverageTestStates")?;

        let failed = TestState::Failed.as_number();
        let passed = TestState::Passed.as_number();
        let skipped = TestState::Skipped.as_number();
        let unknown = TestState::Unknown.as_number();

        sqlx::query!(
            "
            insert into ResolvedLineCoverageTestStates (
                product_id,
                cov_filepath,
                cov_file_hash,
                cov_line,
                test_state
            )
            with CombinedLineCoveragePerTestRun (
                product_id,
                test_run_name,
                cov_filepath,
                cov_file_hash,
                cov_line
            ) as (
                select
                    product_id,
                    test_run_name,
                    cov_filepath,
                    cov_file_hash,
                    cov_line
                from TestRunLineCoverage

                union

                select
                    product_id,
                    test_run_name,
                    cov_filepath,
                    cov_file_hash,
                    cov_line
                from TestCaseLineCoverage
            )
            select
                cpt.product_id,
                cpt.cov_filepath,
                cpt.cov_file_hash,
                cpt.cov_line,
                case
                    when exists (
                        select ftc.test_run_name
                        from FailedTestCases ftc
                        where ftc.product_id = cpt.product_id
                        and ftc.test_run_name = cpt.test_run_name
                    ) or exists (
                        select ftr.test_run_name
                        from FailedTestRuns ftr
                        where ftr.product_id = cpt.product_id
                        and ftr.test_run_name = cpt.test_run_name
                    ) then $1
                    when exists (
                        select stc.test_run_name
                        from SkippedTestCases stc
                        where stc.product_id = cpt.product_id
                        and stc.test_run_name = cpt.test_run_name
                    ) or exists (
                        select str.test_run_name
                        from SkippedTestRuns str
                        where str.product_id = cpt.product_id
                        and str.test_run_name = cpt.test_run_name
                    ) then $3
                    when exists (
                        select ptc.test_run_name
                        from PassedTestCases ptc
                        where ptc.product_id = cpt.product_id
                        and ptc.test_run_name = cpt.test_run_name
                    ) or exists (
                        select ptr.test_run_name
                        from PassedTestRuns ptr
                        where ptr.product_id = cpt.product_id
                        and ptr.test_run_name = cpt.test_run_name
                    ) then $2
                    -- unknown
                    else $5
                    end as test_state
            from CombinedLineCoveragePerTestRun cpt, ValidTestRuns vt
            where cpt.product_id = vt.product_id
            and cpt.test_run_name = vt.test_run_name

            on conflict (
                product_id,
                cov_filepath,
                cov_line
            ) do update
            set
                cov_file_hash = excluded.cov_file_hash,
                test_state = excluded.test_state
            ",
            failed,
            passed,
            skipped,
            unknown,
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update ResolvedLineCoverageTestStates")?;

        Ok(())
    }

    async fn update_test_run_trace_coverage(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from TraceCoveragePerTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing TraceCoveragePerTestRuns")?;

        sqlx::query!(
            "
        insert into TraceCoveragePerTestRuns (
            product_id,
            test_run_name,
            filepath,
            file_hash,
            traced_line,
            cov_line,
            cov_state
        )
        select
            rc.product_id,
            rc.test_run_name,
            rc.cov_filepath,
            rc.cov_file_hash,
            ts.traced_line,
            rc.cov_line,
            rc.cov_state
        from ResolvedTestRunLineCoverage rc, TraceSpans ts
        where rc.cov_file_hash not null
        and rc.cov_file_hash = ts.file_hash
        and rc.cov_line >= ts.start_line
        and rc.cov_line <= ts.end_line
        "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_test_case_trace_coverage(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from TraceCoveragePerTestCases
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing TraceCoveragePerTestCases")?;

        sqlx::query!(
            "
            insert into TraceCoveragePerTestCases (
                product_id,
                test_run_name,
                test_case_name,
                filepath,
                file_hash,
                traced_line,
                cov_line,
                cov_state
            )
            select
                rc.product_id,
                rc.test_run_name,
                rc.test_case_name,
                rc.cov_filepath,
                rc.cov_file_hash,
                ts.traced_line,
                rc.cov_line,
                rc.cov_state
            from ResolvedTestCaseLineCoverage rc, TraceSpans ts
            where rc.cov_file_hash not null
            and rc.cov_file_hash = ts.file_hash
            and rc.cov_line >= ts.start_line
            and rc.cov_line <= ts.end_line
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
