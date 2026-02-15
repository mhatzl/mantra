use mantra_schema::test_runs::TestCaseState;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_test_run_data(&mut self) -> Result<(), anyhow::Error> {
        self.update_test_run_descendants().await?;
        self.update_leaf_test_runs().await?;
        self.update_obsolete_test_runs().await?;
        self.update_failed_test_runs().await?;
        self.update_skipped_test_runs().await?;
        self.update_passed_test_runs().await?;
        self.update_usable_test_runs().await?;

        Ok(())
    }

    async fn update_test_run_descendants(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            with recursive TransitiveChildren(
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                descendant_test_run_name,
                descendant_test_run_date
            ) as
            (
                select
                    last_collect_nr,
                    product_id,
                    parent_name,
                    parent_date,
                    child_name,
                    child_date
                from TestRunHierarchies
                union all
                select
                    th.last_collect_nr,
                    tc.product_id,
                    tc.test_run_name,
                    tc.test_run_date,
                    th.child_name,
                    th.child_date
                from TestRunHierarchies th, TransitiveChildren tc
                where tc.product_id = th.product_id and tc.descendant_test_run_name = th.parent_name
                and tc.descendant_test_run_date = th.parent_date
                -- prevents endless recursion in case of test run cycles
                -- but includes self-references to detect a cycle
                and tc.test_run_name != th.parent_name and tc.test_run_date != th.parent_date
            )
            -- replacing, because 'on conflict' seems to break with select instead of value
            -- and the important info is insert and delete for such aggregated tables anyway
            insert or replace into TestRunDescendants (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                descendant_test_run_name,
                descendant_test_run_date
            )
            select
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date,
                descendant_test_run_name,
                descendant_test_run_date
            from TransitiveChildren
            "
        )
        .execute(self.connection_mut())
        .await?;

        let test_run_cycle_exists = sqlx::query!(
            "
            select
                p.id as product_id,
                p.name as product_name,
                p.base as product_base,
                td.test_run_name,
                td.test_run_date
            from TestRunDescendants td, Products p
            where td.test_run_name = td.descendant_test_run_name
            and td.test_run_date = td.descendant_test_run_date
            and td.product_id = p.id
            "
        )
        .fetch_all(self.connection_mut())
        .await?;

        if !test_run_cycle_exists.is_empty() {
            for bad in test_run_cycle_exists {
                eprintln!(
                    "Test run cycle detected for test run name='{}' date='{}' in product id='{}' name='{}' base='{}'",
                    bad.test_run_name,
                    bad.test_run_date,
                    bad.product_id,
                    bad.product_name,
                    bad.product_base
                );
            }
            anyhow::bail!("Test run cycle detected!");
        }

        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            delete from TestRunDescendants
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_leaf_test_runs(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into LeafTestRuns (
                last_collect_nr,
                product_id,
                test_run_name,
                test_run_date
            )
            select last_collect_nr, product_id, name, utc_date
            from TestRuns
            where last_collect_nr = $1 and product_id = $2
            except
            select last_collect_nr, product_id, parent_name, parent_date
            from TestRunHierarchies
            where last_collect_nr = $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from LeafTestRuns
            where last_collect_nr != $1
            and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_obsolete_test_runs(&mut self) -> Result<(), anyhow::Error> {
        // TODO: update obsolete test run table

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
                from TestCases
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
        .await?;

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
                from TestCases
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
        .await?;

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
                from TestCases
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
        .await?;

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
        .await?;

        Ok(())
    }
}
