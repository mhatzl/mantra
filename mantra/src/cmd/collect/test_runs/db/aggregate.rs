use anyhow::Context;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_test_run_data(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        let latest_collected_tests_nr = sqlx::query!(
            r#"
            select max(latest_collection) as "latest_collection:i64"
            from TestRuns
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating test runs")?
        .and_then(|r| r.latest_collection);

        // if there is no test run data, we still must ensure that aggragation tables get cleared
        if latest_collected_tests_nr.is_some() && latest_collected_tests_nr != Some(collect_nr) {
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
        sqlx::query!(
            "
            delete from TestRunDescendants
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing TestRunDescendants")?;

        sqlx::query!(
            "
            with recursive TransitiveChildren(
                product_id,
                test_run_name,
                descendant_test_run_name
            ) as
            (
                select
                    product_id,
                    parent_name,
                    child_name
                from TestRunHierarchies

                union all

                select
                    tc.product_id,
                    tc.test_run_name,
                    th.child_name
                from TestRunHierarchies th, TransitiveChildren tc
                where tc.product_id = th.product_id
                and tc.descendant_test_run_name = th.parent_name
                -- prevents endless recursion in case of test run cycles
                -- but includes self-references to detect a cycle
                and (
                    tc.test_run_name != th.parent_name
                )
            )
            insert into TestRunDescendants (
                product_id,
                test_run_name,
                descendant_test_run_name
            )
            select distinct
                product_id,
                test_run_name,
                descendant_test_run_name
            from TransitiveChildren
            "
        )
        .execute(self.connection_mut())
        .await?;

        let test_run_cycle_exists = sqlx::query!(
            "
            select
                td.product_id,
                td.test_run_name
            from TestRunDescendants td
            where td.test_run_name = td.descendant_test_run_name
            "
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Failed checking for test run cycles")?;

        if !test_run_cycle_exists.is_empty() {
            for bad in test_run_cycle_exists {
                log::error!(
                    "Test run cycle detected for test run name='{}' in product id='{}'",
                    bad.test_run_name,
                    bad.product_id
                );
            }
            anyhow::bail!("Test run cycle detected!");
        }

        Ok(())
    }

    async fn update_leaf_test_runs(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from LeafTestRuns
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing LeafTestRuns")?;

        sqlx::query!(
            "
            insert into LeafTestRuns (
                product_id,
                test_run_name
            )
            select product_id, name
            from TestRuns

            except

            select product_id, parent_name
            from TestRunHierarchies
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
