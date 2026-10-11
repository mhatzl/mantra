use anyhow::Context;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_test_run_data(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        let latest_collected_tests_nr = sqlx::query!(
            r#"
            select max(collect_nr) as "latest_collect_nr:i64"
            from TestRuns
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating test runs")?
        .and_then(|r| r.latest_collect_nr);

        if latest_collected_tests_nr != Some(collect_nr) {
            log::info!("No test data collected, skipping test aggregation");

            return Ok(());
        }

        self.update_test_run_descendants()
            .await
            .context("Failed to update test run descendants")?;
        self.update_leaf_test_runs()
            .await
            .context("Failed to update leaf test runs")?;

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
}
