use anyhow::Context;

use crate::cmd::collect::{cfg::CollectMode, products::collection::ProductCollection};

impl<'db, 'c> ProductCollection<'db, 'c> {
    pub(crate) async fn update_requirement_changes(
        &mut self,
        collect_mode: &CollectMode,
    ) -> Result<(), anyhow::Error> {
        self.update_added_requirements()
            .await
            .context("Updating added requirements")?;

        if collect_mode == &CollectMode::RemoveMissing {
            self.update_removed_requirements()
                .await
                .context("Updating removed requirements")?;
        }

        self.update_changed_requirements()
            .await
            .context("Updating changed requirements")?;

        self.update_existing_requirements(collect_mode)
            .await
            .context("Updating existing requirements")?;

        Ok(())
    }

    async fn update_added_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert into AddedRequirements (
                first_collect_nr,
                product_id,
                req_id
            )
            select r.collect_nr, r.product_id, r.id
            from Requirements r
            where r.collect_nr = $1
            and r.product_id = $2
            -- already existing
            -- since existing gets updated later, it must have been from a previous collection
            --
            -- Note: If it got deleted before and now added, it is considered as newly added.
            and not exists (
                select er.req_id
                from ExistingRequirements er
                where er.product_id = r.product_id
                and er.req_id = r.id
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_removed_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into RemovedRequirements (
                removed_collect_nr,
                last_collect_nr,
                product_id,
                req_id
            )
            select $1 as removed_collect_nr, r.collect_nr, r.product_id, r.id
            from Requirements r left join RemovedRequirements rr
                on rr.last_collect_nr = r.collect_nr
                and rr.product_id = r.product_id
                and rr.req_id = r.id
            where r.product_id = $2
            and r.collect_nr < $1
            -- only consider requirements not already removed
            and rr.removed_collect_nr is null
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_changed_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert into ChangedRequirements (
                new_collect_nr,
                old_collect_nr,
                product_id,
                req_id
            )
            select rn.collect_nr, ro.collect_nr, rn.product_id, rn.id
            from Requirements rn, Requirements ro
            where rn.collect_nr = $1
            and rn.product_id = $2
            and rn.product_id = ro.product_id
            and rn.id = ro.id
            and ro.collect_nr = (
                select max(ri.collect_nr)
                from Requirements ri
                where ri.collect_nr < rn.collect_nr
                and ri.product_id = ro.product_id
                and ri.id = ro.id
            )
            and rn.data_hash != ro.data_hash
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_existing_requirements(
        &mut self,
        collect_mode: &CollectMode,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id().clone();

        if collect_mode == &CollectMode::RemoveMissing {
            sqlx::query!(
                "
                delete from ExistingRequirements
                where exists (
                    select rr.req_id
                    from RemovedRequirements rr
                    where rr.removed_collect_nr = $1
                    and rr.product_id = $2
                )
                ",
                collect_nr,
                product_id
            )
            .execute(self.connection_mut())
            .await?;
        }

        sqlx::query!(
            "
            insert or replace into ExistingRequirements (
                latest_collect_nr,
                product_id,
                req_id
            )
            select ar.first_collect_nr, ar.product_id, ar.req_id
            from AddedRequirements ar
            where ar.first_collect_nr = $1
            and ar.product_id = $2

            union all

            -- already existed but collected again => bump collect
            select r.collect_nr, r.product_id, r.id
            from Requirements r, ExistingRequirements er
            where r.collect_nr = $1
            and r.product_id = $2
            and r.product_id = er.product_id
            and r.id = er.req_id
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
