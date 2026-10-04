use anyhow::Context;
use mantra_schema::{report::tests::ResolvedLineCoverageState, test_runs::TestCaseState};

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_test_run_data(&mut self) -> Result<(), anyhow::Error> {
        self.update_test_run_descendants()
            .await
            .context("Failed to update test run descendants")?;
        self.update_leaf_test_runs()
            .await
            .context("Failed to update leaf test runs")?;
        self.update_obsolete_test_runs()
            .await
            .context("Failed to update obsolete test runs")?;
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

    async fn update_test_run_descendants(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            with recursive TransitiveChildren(
                collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                descendant_test_run_name,
                descendant_test_run_date
            ) as
            (
                select
                    collect_nr,
                    product_id,
                    parent_name,
                    parent_date,
                    child_name,
                    child_date
                from TestRunHierarchies
                where collect_nr = $1

                union all

                select
                    th.collect_nr,
                    tc.product_id,
                    tc.test_run_name,
                    tc.test_run_date,
                    th.child_name,
                    th.child_date
                from TestRunHierarchies th, TransitiveChildren tc
                where tc.collect_nr = th.collect_nr and tc.product_id = th.product_id
                and tc.descendant_test_run_name = th.parent_name
                and tc.descendant_test_run_date = th.parent_date
                -- prevents endless recursion in case of test run cycles
                -- but includes self-references to detect a cycle
                and (
                    tc.test_run_name != th.parent_name
                    or tc.test_run_date != th.parent_date
                )
            )
            insert or ignore into TestRunDescendants (
                collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                descendant_test_run_name,
                descendant_test_run_date
            )
            select
                collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                descendant_test_run_name,
                descendant_test_run_date
            from TransitiveChildren
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        let test_run_cycle_exists = sqlx::query!(
            "
            select
                td.product_id,
                td.test_run_name,
                td.test_run_date
            from TestRunDescendants td
            where td.test_run_name = td.descendant_test_run_name
            and td.test_run_date = td.descendant_test_run_date
            "
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Failed checking for test run cycles")?;

        if !test_run_cycle_exists.is_empty() {
            for bad in test_run_cycle_exists {
                log::error!(
                    "Test run cycle detected for test run name='{}' date='{}' in product id='{}'",
                    bad.test_run_name,
                    bad.test_run_date,
                    bad.product_id
                );
            }
            anyhow::bail!("Test run cycle detected!");
        }

        Ok(())
    }

    async fn update_leaf_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into LeafTestRuns (
                collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            select collect_nr, product_id, name, utc_date
            from TestRuns
            where collect_nr = $1

            except

            select collect_nr, product_id, parent_name, parent_date
            from TestRunHierarchies
            where collect_nr = $1
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_obsolete_test_runs(&mut self) -> Result<(), anyhow::Error> {
        // TODO: Adapt collect cmd to collect reqs, annotations, tests, and reviews separately.
        // If reqs and annotations for the same product are collected in a later collection,
        // tests are marked as obsolete (same for reviews).
        // Would need to add additional collection numbers for tables that connect reqs, annotations, tests, reviews
        // e.g. DirectProductReqTraces or TestCaseOverrides
        //
        // Until then, all tests of older collections than the latest product collection are obsolete

        sqlx::query!(
            "
            insert or ignore into ObsoleteTestRuns (
                collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            select collect_nr, product_id, name, utc_date
            from TestRuns tr
            where
            -- requirements collected since test was added
            tr.collect_nr < (
                select max(r.collect_nr)
                from Requirements r
                where r.product_id = tr.product_id
            )
            -- annotations collected since test was added
            or tr.collect_nr < (
                select max(a.collect_nr)
                from ProductAnnotationSources a
                where a.product_id = tr.product_id
            )
            -- TODO: check test location and file hash changes
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn resolve_test_case_states(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into ResolvedTestCaseStates (
                collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                state
            )
            select
                collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name,
                state
            from TestCaseOverrides
            where collect_nr = $1

            union all

            select
                t.collect_nr,
                t.product_id,
                t.test_run_name,
                t.test_run_date,
                t.name as test_case_name,
                t.state
            from TestCases t
            where
            t.collect_nr = $1
            and not exists (
                select
                    o.collect_nr,
                    o.product_id,
                    o.test_run_name,
                    o.test_run_date,
                    o.test_case_name
                from TestCaseOverrides o
                where t.collect_nr = o.collect_nr
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
                collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name
            )
            select
                collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                test_case_name
            from PassedTestCases pc
            where collect_nr = $1
            and not exists (
                select
                    collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from ObsoleteTestRuns o
                where pc.collect_nr =  o.collect_nr
                and pc.product_id = o.product_id
                and pc.test_run_name = o.test_run_name
                and pc.test_run_date = o.test_run_date
            )
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
            insert or replace into ResolvedTestRunLineCoverage (
                collect_nr,
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
                tro.collect_nr,
                tro.product_id,
                tro.test_run_name,
                tro.test_run_date,
                tro.cov_filepath,
                pf.file_hash as cov_file_hash,
                tro.cov_line,
                case
                    when tro.hits is not null and tro.hits > 0 then $4
                    else $5
                end as state,
                tro.hits
            from TestRunLineCoverageOverrides tro
                left join ProductRelatedFiles pf on
                    tro.collect_nr = pf.collect_nr
                    and tro.product_id = pf.product_id
                    and tro.cov_filepath = pf.filepath
            where tro.collect_nr = $1

            union all

            select
                t.collect_nr,
                t.product_id,
                t.test_run_name,
                t.test_run_date,
                t.cov_filepath,
                t.cov_file_hash,
                t.cov_line,
                case
                    when exists (
                        select er.filepath, er.start_line, er.end_line
                        from ExcludedLineRanges er
                        where er.collect_nr = $1 and er.product_id = t.product_id
                        and t.cov_filepath = er.filepath
                        and t.cov_line >= er.start_line and t.cov_line <= er.end_line
                    ) then $3
                    when t.hits is not null and t.hits > 0 then $2
                    else $6
                end as state,
                t.hits
            from TestRunLineCoverage t
            where not exists (
                select
                    collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date,
                    cov_filepath,
                    cov_line
                from TestRunLineCoverageOverrides o
                where t.collect_nr = $1
                and t.collect_nr = o.collect_nr
                and t.product_id = o.product_id
                and t.test_run_name = o.test_run_name
                and t.test_run_date = o.test_run_date
                and t.cov_filepath = o.cov_filepath
                and t.cov_line = o.cov_line
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
            insert or replace into ResolvedTestCaseLineCoverage (
                collect_nr,
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
                tco.collect_nr,
                tco.product_id,
                tco.test_run_name,
                tco.test_run_date,
                tco.test_case_name,
                tco.cov_filepath,
                pf.file_hash as cov_file_hash,
                tco.cov_line,
                case
                    when tco.hits is not null and tco.hits > 0 then $4
                    else $5
                end as state,
                tco.hits
            from TestCaseLineCoverageOverrides tco
                left join ProductRelatedFiles pf on
                    tco.collect_nr = pf.collect_nr
                    and tco.product_id = pf.product_id
                    and tco.cov_filepath = pf.filepath
            where tco.collect_nr = $1

            union all

            select
                t.collect_nr,
                t.product_id,
                t.test_run_name,
                t.test_run_date,
                t.test_case_name,
                t.cov_filepath,
                t.cov_file_hash,
                t.cov_line,
                case
                    when exists (
                        select er.filepath, er.start_line, er.end_line
                        from ExcludedLineRanges er
                        where er.collect_nr = $1 and er.product_id = t.product_id
                        and t.cov_filepath = er.filepath
                        and t.cov_line >= er.start_line and t.cov_line <= er.end_line
                    ) then $3
                    when t.hits is not null and t.hits > 0 then $2
                    else $6
                end as state,
                t.hits
            from TestCaseLineCoverage t
            where not exists (
                select
                    collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date,
                    test_case_name,
                    cov_filepath,
                    cov_line
                from TestCaseLineCoverageOverrides o
                where t.collect_nr = $1
                and t.collect_nr = o.collect_nr
                and t.product_id = o.product_id
                and t.test_run_name = o.test_run_name
                and t.test_run_date = o.test_run_date
                and t.test_case_name = o.test_case_name
                and t.cov_filepath = o.cov_filepath
                and t.cov_line = o.cov_line
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
            insert or replace into ResolvedLineCoverageStates (
                last_collect_nr,
                product_id,
                cov_filepath,
                cov_file_hash,
                cov_line,
                state
            )
            with ResolvedCoveredLineHits (cov_filepath, cov_file_hash, cov_line, hits) as (
                select
                    cov_filepath,
                    cov_file_hash,
                    cov_line,
                    max(hits)
                from (
                    select
                        cov_filepath,
                        cov_file_hash,
                        cov_line,
                        max(hits) as hits
                    from ResolvedTestRunLineCoverage
                    where last_collect_nr = $1 and product_id = $2
                    group by cov_filepath, cov_file_hash, cov_line

                    union all

                    select
                        cov_filepath,
                        cov_file_hash,
                        cov_line,
                        max(hits) as hits
                    from ResolvedTestCaseLineCoverage
                    where last_collect_nr = $1 and product_id = $2
                    group by cov_filepath, cov_file_hash, cov_line
                )
                group by cov_filepath, cov_file_hash, cov_line
            ),
            OverriddenLines (cov_filepath, cov_line) as (
                select cov_filepath, cov_line
                from TestCaseLineCoverageOverrides
                where last_collect_nr = $1 and product_id = $2

                union

                select cov_filepath, cov_line
                from TestRunLineCoverageOverrides
                where last_collect_nr = $1 and product_id = $2
            )
            select
                $1 as last_collect_nr,
                $2 as product_id,
                cov_filepath,
                cov_file_hash,
                cov_line,
                case when exists (
                    select cov_filepath, cov_line
                    from OverriddenLines ol
                    where lh.cov_filepath = ol.cov_filepath
                    and lh.cov_line = ol.cov_line
                ) then case
                    when lh.hits is not null and lh.hits > 0 then $5
                    else $6
                    end
                when exists (
                    select er.filepath, er.start_line, er.end_line
                    from ExcludedLineRanges er
                    where er.last_collect_nr = $1 and er.product_id = $2
                    and lh.cov_filepath = er.filepath
                    and lh.cov_line >= er.start_line and lh.cov_line <= er.end_line
                ) then $3
                when lh.hits is not null and lh.hits > 0 then $2
                else $6
                end as state
            from ResolvedCoveredLineHits lh
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
        .context("Failed to update ResolvedLineCoverageStates")?;

        Ok(())
    }

    async fn update_failed_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let passed_test_nr = TestCaseState::Passed.as_nr();
        let skipped_test_nr = TestCaseState::Skipped.as_nr();

        sqlx::query!(
            "
            insert or replace into FailedTestRuns (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            with TestRunsWithFailedTestCase (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from ResolvedTestCaseStates
                where last_collect_nr = $1
                and product_id = $2
                and state != $3 and state != $4
            ), IndirectFailedTestRun (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                d.last_collect_nr,
                d.product_id,
                d.test_run_name,
                d.test_run_date
                from TestRunDescendants d, TestRunsWithFailedTestCase f
                where d.last_collect_nr = f.last_collect_nr
                and d.product_id = f.product_id
                and d.descendant_test_run_name = f.test_run_name
                and d.descendant_test_run_date = f.test_run_date
            )
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from TestRunsWithFailedTestCase
            union all
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from IndirectFailedTestRun
            ",
            collect_nr,
            product_id,
            passed_test_nr,
            skipped_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from FailedTestRuns
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }

    async fn update_skipped_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let skipped_test_nr = TestCaseState::Skipped.as_nr();

        sqlx::query!(
            "
            insert or replace into SkippedTestRuns (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            with TestRunsWithSkippedTestCase (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from ResolvedTestCaseStates
                where last_collect_nr = $1
                and product_id = $2
                and state = $3
            ), IndirectSkippedTestRun (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                d.last_collect_nr,
                d.product_id,
                d.test_run_name,
                d.test_run_date
                from TestRunDescendants d, TestRunsWithSkippedTestCase s
                where d.last_collect_nr = s.last_collect_nr
                and d.product_id = s.product_id
                and d.descendant_test_run_name = s.test_run_name
                and d.descendant_test_run_date = s.test_run_date
            )
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from (
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from TestRunsWithSkippedTestCase
                union all
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from IndirectSkippedTestRun
            )
            except
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from FailedTestRuns
            ",
            collect_nr,
            product_id,
            skipped_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from SkippedTestRuns
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }

    async fn update_passed_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let passed_test_nr = TestCaseState::Passed.as_nr();

        sqlx::query!(
            "
            insert or replace into PassedTestRuns (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            with TestRunsWithPassedTestCase (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from ResolvedTestCaseStates
                where last_collect_nr = $1
                and product_id = $2
                and state = $3
            ), IndirectPassedTestRun (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            ) as (
                select
                d.last_collect_nr,
                d.product_id,
                d.test_run_name,
                d.test_run_date
                from TestRunDescendants d, TestRunsWithPassedTestCase p
                where d.last_collect_nr = p.last_collect_nr
                and d.product_id = p.product_id
                and d.descendant_test_run_name = p.test_run_name
                and d.descendant_test_run_date = p.test_run_date
            )
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from (
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from TestRunsWithPassedTestCase
                union all
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from IndirectPassedTestRun
            )
            except
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from (
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from FailedTestRuns
                union all
                select
                    last_collect_nr,
                    product_id,
                    test_run_name,
                    test_run_date
                from SkippedTestRuns
            )
            ",
            collect_nr,
            product_id,
            passed_test_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from PassedTestRuns
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }

    async fn update_usable_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into UsableTestRuns (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from PassedTestRuns
            where last_collect_nr = $1 and product_id = $2
            except
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            from ObsoleteTestRuns
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from UsableTestRuns
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }

    async fn update_test_run_trace_coverage(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceCoveragePerTestRuns (
                last_collect_nr,
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
                sc.last_collect_nr,
                sc.product_id,
                sc.test_run_name,
                sc.test_run_date,
                sc.cov_filepath,
                ts.file_hash,
                ts.traced_line,
                sc.cov_line,
                sc.hits
            from ResolvedTestRunLineCoverage sc, ProductRelatedFiles pf, TraceSpans ts
            where sc.last_collect_nr = $1 and sc.last_collect_nr = pf.last_collect_nr
            and sc.product_id = $2 and sc.product_id = pf.product_id
            and sc.hits not null and sc.cov_filepath = pf.filepath
            and pf.file_hash = ts.file_hash
            and (sc.cov_file_hash is null or sc.cov_file_hash = ts.file_hash)
            and sc.cov_line >= ts.start_line
            and sc.cov_line <= ts.end_line
            and sc.cov_line not in (
                select line
                from ExcludedCoverageLines
                where file_hash = ts.file_hash
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TraceCoveragePerTestRuns
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }

    async fn update_test_case_trace_coverage(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into TraceCoveragePerTestCases (
                last_collect_nr,
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
                sc.last_collect_nr,
                sc.product_id,
                sc.test_run_name,
                sc.test_run_date,
                sc.test_case_name,
                sc.cov_filepath,
                ts.file_hash,
                ts.traced_line,
                sc.cov_line,
                sc.hits
            from ResolvedTestCaseLineCoverage sc, ProductRelatedFiles pf, TraceSpans ts
            where sc.last_collect_nr = $1 and sc.last_collect_nr = pf.last_collect_nr
            and sc.product_id = $2 and sc.product_id = pf.product_id
            and sc.hits not null and sc.cov_filepath = pf.filepath
            and pf.file_hash = ts.file_hash
            and (sc.cov_file_hash is null or sc.cov_file_hash = ts.file_hash)
            and sc.cov_line >= ts.start_line and sc.cov_line <= ts.end_line
            and sc.cov_line not in (
                select line
                from ExcludedCoverageLines
                where file_hash = ts.file_hash
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from TraceCoveragePerTestCases
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }
}
