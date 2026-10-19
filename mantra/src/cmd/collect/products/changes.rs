use anyhow::Context;
use mantra_schema::product::ProductId;

use crate::cmd::collect::{Collection, cfg::CollectMode};

impl<'db> Collection<'db> {
    pub(crate) async fn update_product_changes(
        &mut self,
        existing_product_ids: impl Iterator<Item = &ProductId>,
        collect_mode: &CollectMode,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        for product_id in existing_product_ids {
            let latest_collect_nr = sqlx::query!(
                "
                select latest_collect_nr
                from ExistingProducts
                where product_id = $1
                ",
                product_id
            )
            .fetch_optional(self.connection_mut())
            .await
            .context("Failed to get existing products")?
            .map(|r| r.latest_collect_nr);

            if let Some(prev_collect_nr) = latest_collect_nr {
                let changed = sqlx::query!(
                    "
                    select p.collect_nr
                    from Products p, Products ep
                    where p.collect_nr = $1
                    and p.id = $3
                    and ep.collect_nr = $2
                    and ep.id = $3
                    and p.data_hash != ep.data_hash
                    ",
                    collect_nr,
                    prev_collect_nr,
                    product_id
                )
                .fetch_optional(self.connection_mut())
                .await
                .context("Failed to get existing products")?
                .is_some();

                if changed {
                    sqlx::query!(
                        "
                        insert into ChangedProducts (
                            new_collect_nr,
                            old_collect_nr,
                            product_id
                        )
                        values (
                            $1,
                            $2,
                            $3
                        )
                        ",
                        collect_nr,
                        prev_collect_nr,
                        product_id
                    )
                    .execute(self.connection_mut())
                    .await
                    .context("Updating changed products")?;
                }
            } else {
                sqlx::query!(
                    "
                    insert into AddedProducts (
                        first_collect_nr,
                        product_id
                    )
                    values (
                        $1,
                        $2
                    )
                    ",
                    collect_nr,
                    product_id
                )
                .execute(self.connection_mut())
                .await
                .context("Updating added products")?;
            }

            sqlx::query!(
                "
                insert into ExistingProducts (
                    latest_collect_nr,
                    product_id
                )
                values (
                    $1,
                    $2
                )
                on conflict (product_id) do update
                set latest_collect_nr = excluded.latest_collect_nr
                ",
                collect_nr,
                product_id
            )
            .execute(self.connection_mut())
            .await
            .context("Updating existing products")?;
        }

        if collect_mode == &CollectMode::RemoveMissing {
            sqlx::query!(
                "
                insert into RemovedProducts (
                    removed_collect_nr,
                    last_collect_nr,
                    product_id
                )
                select $1 as removed_collect_nr, latest_collect_nr, product_id
                from ExistingProducts
                where latest_collect_nr < $1
                ",
                collect_nr
            )
            .execute(self.connection_mut())
            .await
            .context("Updating removed products")?;

            sqlx::query!(
                "
                delete from ExistingProducts
                where latest_collect_nr < $1
                ",
                collect_nr
            )
            .execute(self.connection_mut())
            .await
            .context("Deleting missing products from existing products")?;

            // Delete existing entries for removed products

            sqlx::query!(
                "
                delete from ExistingRequirements
                where product_id in (
                    select product_id
                    from RemovedProducts
                    where removed_collect_nr = $1
                )
                ",
                collect_nr
            )
            .execute(self.connection_mut())
            .await
            .context("Deleting existing requirements for removed product")?;

            sqlx::query!(
                "
                delete from ExistingAnnotationSources
                where product_id in (
                    select product_id
                    from RemovedProducts
                    where removed_collect_nr = $1
                )
                ",
                collect_nr
            )
            .execute(self.connection_mut())
            .await
            .context("Deleting existing annotation sources for removed product")?;

            sqlx::query!(
                "
                delete from ExistingTestRuns
                where product_id in (
                    select product_id
                    from RemovedProducts
                    where removed_collect_nr = $1
                )
                ",
                collect_nr
            )
            .execute(self.connection_mut())
            .await
            .context("Deleting existing test runs for removed product")?;

            sqlx::query!(
                "
                delete from ExistingReviews
                where product_id in (
                    select product_id
                    from RemovedProducts
                    where removed_collect_nr = $1
                )
                ",
                collect_nr
            )
            .execute(self.connection_mut())
            .await
            .context("Deleting existing reviews for removed product")?;
        }

        Ok(())
    }
}
