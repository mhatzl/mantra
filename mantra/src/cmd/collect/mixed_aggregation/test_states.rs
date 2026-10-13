use anyhow::Context;
use mantra_schema::{report::tests::ResolvedLineCoverageState, test_runs::TestCaseState};

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
        self.update_test_run_trace_coverage()
            .await
            .context("Failed to update test run trace coverage")?;
        self.update_test_case_trace_coverage()
            .await
            .context("Failed to update test case trace coverage")?;

        Ok(())
    }

    async fn resolve_test_case_states(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        // TODO: detect and flag multiple overrides for the same test case

        sqlx::query!(
            "
            with LatestTestCaseOverrides (
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                state
            ) as (
                select
                    tco.test_collect_nr,
                    tco.product_id,
                    tco.test_run_name,
                    tco.test_run_date,
                    tco.test_case_name,
                    tco.state
                from TestCaseOverrides tco, ValidReviews vr
                where vr.review_collect_nr = tco.collect_nr
                and vr.product_id = tco.product_id
                and vr.review_name = tco.review_name
                and vr.review_date = tco.review_date
                and tco.test_collect_nr = (
                    select max(tr.collect_nr)
                    from TestRuns tr
                    where tr.product_id = tco.product_id
                )
            )
            insert into ResolvedTestCaseStates (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                state
            )
            select
                $1 as agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                state
            from LatestTestCaseOverrides

            union all

            select
                $1 as agg_collect_nr,
                t.collect_nr,
                t.product_id,
                t.test_run_name,
                t.test_run_date,
                t.name as test_case_name,
                t.state
            from TestCases t
            where t.collect_nr = (
                select max(tr.collect_nr)
                from TestRuns tr
                where tr.product_id = t.product_id
            )
            and not exists (
                select
                    o.test_case_name
                from LatestTestCaseOverrides o
                where t.collect_nr = o.test_collect_nr
                and t.product_id = o.product_id
                and t.test_run_name = o.test_run_name
                and t.test_run_date = o.test_run_date
                and t.name = o.test_case_name
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_test_cases(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into UsableTestCases (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name
            )
            select
                pc.agg_collect_nr,
                pc.test_collect_nr,
                pc.product_id,
                pc.test_run_name,
                pc.test_run_date,
                pc.test_case_name
            from PassedTestCases pc, ValidTestRuns vtr
            where pc.agg_collect_nr = $1
            and pc.test_collect_nr = vtr.test_collect_nr
            and pc.product_id = vtr.product_id
            and pc.test_run_name = vtr.test_run_name
            and pc.test_run_date = vtr.test_run_date
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn resolve_line_coverage(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        let covered_state = ResolvedLineCoverageState::Covered.as_nr();
        let excluded_state = ResolvedLineCoverageState::Excluded.as_nr();
        let overridden_covered_state = ResolvedLineCoverageState::OverriddenCovered.as_nr();
        let overridden_uncovered_state = ResolvedLineCoverageState::OverriddenUncovered.as_nr();
        let uncovered_state = ResolvedLineCoverageState::Uncovered.as_nr();

        sqlx::query!(
            "
            insert into ResolvedTestRunLineCoverage (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                cov_filepath,
                cov_file_hash,
                cov_line,
                state,
                hits
            )
            select
                $1 as agg_collect_nr,
                tro.test_collect_nr,
                tro.product_id,
                tro.test_run_name,
                tro.test_run_date,
                tro.cov_filepath,
                cfh.file_hash as cov_file_hash,
                tro.cov_line,
                case
                    when tro.hits is not null and tro.hits > 0 then $4
                    else $5
                end as state,
                tro.hits
            from TestRunLineCoverageOverrides tro
                left join CollectedFileHashes cfh on
                    tro.test_collect_nr = cfh.agg_collect_nr
                    and tro.cov_filepath = cfh.filepath,
                ValidReviews vr
            where tro.collect_nr = (
                select max(rv.collect_nr)
                from Reviews rv
                where rv.product_id = tro.product_id
            )
            and vr.review_collect_nr = tro.collect_nr
            and vr.product_id = tro.product_id
            and vr.review_name = tro.review_name
            and vr.review_date = tro.review_date

            union all

            select
                $1 as agg_collect_nr,
                t.collect_nr,
                t.product_id,
                t.test_run_name,
                t.test_run_date,
                t.cov_filepath,
                cfh.file_hash as cov_file_hash,
                t.cov_line,
                case
                    when exists (
                        select el.line
                        from ExcludedCoverageLines el
                        where cfh.file_hash is not null
                        and cfh.file_hash = el.file_hash
                        and t.cov_line = el.line
                    ) then $3
                    when t.hits is not null and t.hits > 0 then $2
                    else $6
                end as state,
                t.hits
            from TestRunLineCoverage t
                left join CollectedFileHashes cfh on
                    t.collect_nr = cfh.agg_collect_nr
                    and t.cov_filepath = cfh.filepath
            where t.collect_nr = (
                select max(tr.collect_nr)
                from TestRuns tr
                where tr.product_id = t.product_id
            )
            and not exists (
                select
                    cov_line
                from TestRunLineCoverageOverrides tro, ValidReviews vr
                where tro.collect_nr = (
                    select max(rv.collect_nr)
                    from Reviews rv
                    where rv.product_id = tro.product_id
                )
                and vr.review_collect_nr = tro.collect_nr
                and vr.product_id = tro.product_id
                and vr.review_name = tro.review_name
                and vr.review_date = tro.review_date
                and tro.test_collect_nr = t.collect_nr
                and tro.product_id = t.product_id
                and tro.test_run_name = t.test_run_name
                and tro.test_run_date = t.test_run_date
                and tro.cov_filepath = t.cov_filepath
                and tro.cov_line = t.cov_line
            )
            ",
            collect_nr,
            covered_state,
            excluded_state,
            overridden_covered_state,
            overridden_uncovered_state,
            uncovered_state,
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update ResolvedTestRunLineCoverage")?;

        sqlx::query!(
            "
            insert into ResolvedTestCaseLineCoverage (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                cov_filepath,
                cov_file_hash,
                cov_line,
                state,
                hits
            )
            select
                $1 as agg_collect_nr,
                tco.test_collect_nr,
                tco.product_id,
                tco.test_run_name,
                tco.test_run_date,
                tco.test_case_name,
                tco.cov_filepath,
                cfh.file_hash as cov_file_hash,
                tco.cov_line,
                case
                    when tco.hits is not null and tco.hits > 0 then $4
                    else $5
                end as state,
                tco.hits
            from TestCaseLineCoverageOverrides tco
                left join CollectedFileHashes cfh on
                    tco.test_collect_nr = cfh.agg_collect_nr
                    and tco.cov_filepath = cfh.filepath,
                ValidReviews vr
            where tco.collect_nr = (
                select max(rv.collect_nr)
                from Reviews rv
                where rv.product_id = tco.product_id
            )
            and vr.review_collect_nr = tco.collect_nr
            and vr.product_id = tco.product_id
            and vr.review_name = tco.review_name
            and vr.review_date = tco.review_date

            union all

            select
                $1 as agg_collect_nr,
                t.collect_nr,
                t.product_id,
                t.test_run_name,
                t.test_run_date,
                t.test_case_name,
                t.cov_filepath,
                cfh.file_hash as cov_file_hash,
                t.cov_line,
                case
                    when exists (
                        select el.line
                        from ExcludedCoverageLines el
                        where cfh.file_hash is not null
                        and cfh.file_hash = el.file_hash
                        and t.cov_line = el.line
                    ) then $3
                    when t.hits is not null and t.hits > 0 then $2
                    else $6
                end as state,
                t.hits
            from TestCaseLineCoverage t
                left join CollectedFileHashes cfh on
                    t.collect_nr = cfh.agg_collect_nr
                    and t.cov_filepath = cfh.filepath
            where t.collect_nr = (
                select max(tr.collect_nr)
                from TestRuns tr
                where tr.product_id = t.product_id
            )
            and not exists (
                select
                    cov_line
                from TestCaseLineCoverageOverrides tco, ValidReviews vr
                where tco.collect_nr = (
                    select max(rv.collect_nr)
                    from Reviews rv
                    where rv.product_id = tco.product_id
                )
                and vr.review_collect_nr = tco.collect_nr
                and vr.product_id = tco.product_id
                and vr.review_name = tco.review_name
                and vr.review_date = tco.review_date
                and tco.test_collect_nr = t.collect_nr
                and tco.product_id = t.product_id
                and tco.test_run_name = t.test_run_name
                and tco.test_run_date = t.test_run_date
                and tco.test_case_name = t.test_case_name
                and tco.cov_filepath = t.cov_filepath
                and tco.cov_line = t.cov_line
            )
            ",
            collect_nr,
            covered_state,
            excluded_state,
            overridden_covered_state,
            overridden_uncovered_state,
            uncovered_state,
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update ResolvedTestCaseLineCoverage")?;

        sqlx::query!(
            "
            insert into ResolvedLineCoverageStates (
                agg_collect_nr,
                product_id,
                cov_filepath,
                cov_file_hash,
                cov_line,
                state
            )
            with ResolvedCoveredLineHits (product_id, cov_filepath, cov_file_hash, cov_line, hits) as (
                select
                    product_id,
                    cov_filepath,
                    cov_file_hash,
                    cov_line,
                    max(hits)
                from (
                    select
                        product_id,
                        cov_filepath,
                        cov_file_hash,
                        cov_line,
                        max(hits) as hits
                    from ResolvedTestRunLineCoverage rtc
                    where agg_collect_nr = $1
                    group by product_id, cov_filepath, cov_file_hash, cov_line

                    union all

                    select
                        product_id,
                        cov_filepath,
                        cov_file_hash,
                        cov_line,
                        max(hits) as hits
                    from ResolvedTestCaseLineCoverage
                    where agg_collect_nr = $1
                    group by product_id, cov_filepath, cov_file_hash, cov_line
                )
                group by product_id, cov_filepath, cov_file_hash, cov_line
            )
            select
                $1 as agg_collect_nr,
                product_id,
                cov_filepath,
                cov_file_hash,
                cov_line,
                case when lh.hits is not null and lh.hits > 0 then $2
                when exists (
                    select el.line
                    from ExcludedCoverageLines el
                    where el.file_hash = lh.cov_file_hash
                    and el.line = lh.cov_line
                ) then $3
                else $4
                end as state
            from ResolvedCoveredLineHits lh
            ",
            collect_nr,
            covered_state,
            excluded_state,
            uncovered_state,
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update ResolvedLineCoverageStates")?;

        Ok(())
    }

    async fn update_failed_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let passed_test_nr = TestCaseState::Passed.as_nr();
        let skipped_test_nr = TestCaseState::Skipped.as_nr();

        sqlx::query!(
            "
            insert or ignore into FailedTestRuns (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            with TestRunsWithFailedTestCase (
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from ResolvedTestCaseStates
                where agg_collect_nr = $1
                and state != $2 and state != $3
            ), IndirectFailedTestRun (
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    d.collect_nr,
                    d.product_id,
                    d.test_run_name,
                    d.test_run_date
                from TestRunDescendants d, TestRunsWithFailedTestCase f
                where d.collect_nr = f.test_collect_nr
                and d.product_id = f.product_id
                and d.descendant_test_run_name = f.test_run_name
                and d.descendant_test_run_date = f.test_run_date
            )
            select
                $1 as agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from TestRunsWithFailedTestCase

            union all

            select
                $1 as agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from IndirectFailedTestRun
            ",
            collect_nr,
            passed_test_nr,
            skipped_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_skipped_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let skipped_test_nr = TestCaseState::Skipped.as_nr();

        sqlx::query!(
            "
            insert or ignore into SkippedTestRuns (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            with TestRunsWithSkippedTestCase (
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from ResolvedTestCaseStates
                where agg_collect_nr = $1
                and state = $2
            ), IndirectSkippedTestRun (
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    d.collect_nr,
                    d.product_id,
                    d.test_run_name,
                    d.test_run_date
                from TestRunDescendants d, TestRunsWithSkippedTestCase s
                where d.collect_nr = s.test_collect_nr
                and d.product_id = s.product_id
                and d.descendant_test_run_name = s.test_run_name
                and d.descendant_test_run_date = s.test_run_date
            )
            select
                $1 as agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from (
                select
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from TestRunsWithSkippedTestCase

                union all

                select
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from IndirectSkippedTestRun
            )

            except

            select
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from FailedTestRuns
            where agg_collect_nr = $1
            ",
            collect_nr,
            skipped_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_passed_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let passed_test_nr = TestCaseState::Passed.as_nr();

        sqlx::query!(
            "
            insert or ignore into PassedTestRuns (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            with TestRunsWithPassedTestCase (
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from ResolvedTestCaseStates
                where agg_collect_nr = $1
                and state = $2
            ), IndirectPassedTestRun (
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    d.collect_nr,
                    d.product_id,
                    d.test_run_name,
                    d.test_run_date
                from TestRunDescendants d, TestRunsWithPassedTestCase p
                where d.collect_nr = p.test_collect_nr
                and d.product_id = p.product_id
                and d.descendant_test_run_name = p.test_run_name
                and d.descendant_test_run_date = p.test_run_date
            )
            select
                $1 as agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from (
                select
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from TestRunsWithPassedTestCase

                union all

                select
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from IndirectPassedTestRun
            )

            except

            select
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from (
                select
                    agg_collect_nr,
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from FailedTestRuns
                where agg_collect_nr = $1

                union all

                select
                    agg_collect_nr,
                    test_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from SkippedTestRuns
                where agg_collect_nr = $1
            )
            ",
            collect_nr,
            passed_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into UsableTestRuns (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            select
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from PassedTestRuns
            where agg_collect_nr = $1

            except

            select
                $1 as agg_collect_nr,
                collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from ObsoleteTestRuns
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_test_run_trace_coverage(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
        insert or ignore into TraceCoveragePerTestRuns (
            agg_collect_nr,
            test_collect_nr,
            product_id,
            test_run_name,
            test_run_date,
            filepath,
            file_hash,
            traced_line,
            cov_line,
            hits
        )
        select
            rc.agg_collect_nr,
            rc.test_collect_nr,
            rc.product_id,
            rc.test_run_name,
            rc.test_run_date,
            rc.cov_filepath,
            rc.cov_file_hash,
            ts.traced_line,
            rc.cov_line,
            rc.hits
        from ResolvedTestRunLineCoverage rc, TraceSpans ts
        where rc.agg_collect_nr = $1
        and rc.cov_file_hash not null
        and rc.cov_file_hash = ts.file_hash
        and rc.cov_line >= ts.start_line
        and rc.cov_line <= ts.end_line
        -- if an excluded line has hits, the hits take priority
        and (
            (rc.hits not null and rc.hits > 0)
            or rc.cov_line not in (
                select line
                from ExcludedCoverageLines
                where file_hash = ts.file_hash
            )
        )
        ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_test_case_trace_coverage(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into TraceCoveragePerTestCases (
                agg_collect_nr,
                test_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                filepath,
                file_hash,
                traced_line,
                cov_line,
                hits
            )
            select
                rc.agg_collect_nr,
                rc.test_collect_nr,
                rc.product_id,
                rc.test_run_name,
                rc.test_run_date,
                rc.test_case_name,
                rc.cov_filepath,
                rc.cov_file_hash,
                ts.traced_line,
                rc.cov_line,
                rc.hits
            from ResolvedTestCaseLineCoverage rc, TraceSpans ts
            where rc.agg_collect_nr = $1
            and rc.cov_file_hash not null
            and rc.cov_file_hash = ts.file_hash
            and rc.cov_line >= ts.start_line
            and rc.cov_line <= ts.end_line
            -- if an excluded line has hits, the hits take priority
            and (
                (rc.hits not null and rc.hits > 0)
                or rc.cov_line not in (
                    select line
                    from ExcludedCoverageLines
                    where file_hash = ts.file_hash
                )
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
