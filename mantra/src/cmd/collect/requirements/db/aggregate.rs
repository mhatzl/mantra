use anyhow::{Context, anyhow};
use mantra_schema::annotations::TraceKind;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_requirements_data(&mut self) -> Result<(), anyhow::Error> {
        // Note: order is important, because later queries build on updated tables
        self.update_root_requirements()
            .await
            .context("Failed to update root requirements")?;
        self.update_requirement_descendants()
            .await
            .context("Failed to update requirement descendants")?;
        self.update_leaf_requirements()
            .await
            .context("Failed to update leaf requirements")?;
        self.update_deprecated_requirements()
            .await
            .context("Failed to update deprecated requirements")?;
        self.update_excluded_requirements()
            .await
            .context("Failed to update excluded requirements")?;

        self.check_replacing_requirements()
            .await
            .context("Failed checking requirement replacements")?;

        self.update_optional_requirements()
            .await
            .context("Failed to update optional requirements")?;
        self.update_manual_requirements()
            .await
            .context("Failed to update manual requirements")?;
        self.update_usable_requirements()
            .await
            .context("Failed to update usable requirements")?;
        self.update_usable_manual_requirements()
            .await
            .context("Failed to update usable manual requirements")?;
        self.update_usable_non_manual_requirements()
            .await
            .context("Failed to update usable non-manual requirements")?;
        self.update_directly_satisfied_requirements()
            .await
            .context("Failed to update directly satisfied requirements")?;

        Ok(())
    }

    async fn update_root_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into RootRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select $1 as last_collect_nr, product_id, id
            from Requirements r
            where r.last_collect_nr = $1 and r.product_id = $2
            and not exists (
                select *
                from RequirementHierarchies rh
                where rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from RootRequirements
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }

    async fn update_directly_satisfied_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();
        let satisfies_kind = TraceKind::Satisfies.as_nr();

        sqlx::query!(
            "
            insert or replace into DirectlySatisfiedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            with NonManualSatisfyTraced (product_id, id) as (
                select ur.product_id, ur.id
                from UsableNonManualRequirements ur, DirectProductReqTraces dt, Traces t
                where ur.last_collect_nr = $1 and ur.product_id = $2
                and dt.last_collect_nr = $1 and dt.product_id = $2
                and ur.id = dt.req_id and dt.file_hash = t.file_hash
                and dt.line = t.line
                and t.kind = $3
            ),
            ManualReviewed (product_id, id) as (
                select mr.product_id, mr.id
                from UsableManualRequirements mr, ManuallyVerifiedRequirements vr
                where mr.last_collect_nr = $1 and vr.last_collect_nr = $1
                and mr.product_id = $2 and vr.product_id = $2
                and mr.id = vr.req_id
            )
            select $1 as last_collect_nr, product_id, id
            from NonManualSatisfyTraced
            union
            select $1 as last_collect_nr, product_id, id
            from ManualReviewed
            ",
            collect_nr,
            product_id,
            satisfies_kind
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from DirectlySatisfiedRequirements
            where last_collect_nr != $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to delete outdated data")?;

        Ok(())
    }

    async fn update_requirement_descendants(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            with recursive TransitiveChildren(last_collect_nr, product_id, id, descendant_product_id, descendant_id) as
            (
                select
                    $1,
                    parent_product_id, parent_req_id,
                    child_product_id, child_req_id
                from RequirementHierarchies
                union all
                select $1, tc.product_id, tc.id, rh.child_product_id, rh.child_req_id
                from RequirementHierarchies rh, TransitiveChildren tc
                where tc.descendant_product_id = rh.parent_product_id and tc.descendant_id = rh.parent_req_id
                -- prevents endless recursion in case of requirement cycles
                -- match on parent to have the cycle entry in the descendants,
                -- which is then detected in a separate query.
                and (tc.id != rh.parent_req_id or tc.product_id != rh.parent_product_id)
            )
            -- replacing, because 'on conflict' seems to break with select instead of value
            -- and the important info is insert and delete for such aggregated tables anyway
            insert or replace into RequirementDescendants (
                last_collect_nr,
                product_id,
                id,
                descendant_product_id,
                descendant_id
            )
            select last_collect_nr, product_id, id, descendant_product_id, descendant_id
            from TransitiveChildren
            ",
            collect_nr
        ).execute(self.connection_mut()).await?;

        let req_cycle_exists = sqlx::query!(
            "
            select
                rd.product_id,
                rd.id as req_id
            from RequirementDescendants rd
            where rd.product_id = rd.descendant_product_id and rd.id = rd.descendant_id
            "
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Failed to check if hierarchy cycle exists")?;

        if !req_cycle_exists.is_empty() {
            for bad in req_cycle_exists {
                log::error!(
                    "Requirement cycle detected for req '{}' in product id='{}'",
                    bad.req_id,
                    bad.product_id
                );
            }
            anyhow::bail!("Requirement cycle detected!");
        }

        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            delete from RequirementDescendants
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

    async fn check_replacing_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        let bad_replacements = sqlx::query!(
            "
            select rr.req_id, rr.replaced_req_id
            from RequirementReplacements rr, RequirementDescendants rd
            where rr.last_collect_nr = $1 and rr.product_id = $2
            and rr.last_collect_nr = rd.last_collect_nr
            and rr.product_id = rd.product_id
            and rr.product_id = rd.descendant_product_id
            and rr.replaced_req_id = rd.id
            and rr.req_id = rd.descendant_id
            ",
            collect_nr,
            product_id
        )
        .fetch_all(self.connection_mut())
        .await?;

        for bad_record in &bad_replacements {
            log::error!(
                "Requirement '{}' cannot replace its ancestor '{}'",
                bad_record.req_id,
                bad_record.replaced_req_id
            );
        }

        if bad_replacements.is_empty() {
            Ok(())
        } else {
            Err(anyhow!("Requirement tried to replace its ancestor"))
        }
    }

    async fn update_leaf_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into LeafRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select last_collect_nr, product_id, id
            from Requirements
            where last_collect_nr = $1 and product_id = $2
            and id not in (
                select parent_req_id
                from RequirementHierarchies
                where parent_product_id = $2
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from LeafRequirements
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

    async fn update_deprecated_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into DeprecatedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            with MarkedDeprecated(product_id, id) as (
                select product_id, id
                from Requirements
                where deprecated = true
                and last_collect_nr = $1
                and product_id = $2

                union

                select product_id, replaced_req_id
                from RequirementReplacements
                where last_collect_nr = $1 and product_id = $2
            ),
            ParentMarkedDeprecated(product_id, id) as (
                select rd.descendant_product_id, rd.descendant_id
                from RequirementDescendants rd, MarkedDeprecated md
                where rd.product_id = md.product_id and rd.id = md.id
            )
            select $1 as last_collect_nr, product_id, id
            from MarkedDeprecated
            union all
            select $1 as last_collect_nr, product_id, id
            from ParentMarkedDeprecated
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from DeprecatedRequirements
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

    async fn update_excluded_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into ExcludedRequirements (
                last_collect_nr,
                product_id,
                id
            )
            with recursive IsIncluded(last_collect_nr, product_id, id) as (
                select r.last_collect_nr, r.product_id, r.id
                from Requirements r, RootRequirements rr
                where r.last_collect_nr = $1 and r.last_collect_nr = rr.last_collect_nr
                and r.product_id = rr.product_id
                and r.id = rr.id
                and r.exclude = false

                union all

                -- TODO: fix last_collect_nr check for rh
                select ii.last_collect_nr, rh.child_product_id, rh.child_req_id
                from IsIncluded ii, RequirementHierarchies rh, Requirements r
                where ii.product_id = rh.parent_product_id
                and ii.id = rh.parent_req_id
                and ii.last_collect_nr = r.last_collect_nr
                and rh.child_product_id = r.product_id and rh.child_req_id = r.id
                and r.exclude = false
            ),
            IsExcluded(last_collect_nr, product_id, id) as (
                select last_collect_nr, product_id, id
                from Requirements r
                where not exists (
                    select *
                    from IsIncluded ii
                    where ii.last_collect_nr = r.last_collect_nr
                    and ii.product_id = r.product_id
                    and ii.id = r.id
                )
            )
            select distinct $1 as last_collect_nr, product_id, id
            from IsExcluded
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from ExcludedRequirements
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

    async fn update_optional_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into OptionalRequirements (
                last_collect_nr,
                product_id,
                id
            )
            with recursive IsMandatory(last_collect_nr, product_id, id) as (
                select r.last_collect_nr, r.product_id, r.id
                from Requirements r, RootRequirements rr
                where r.last_collect_nr = $1 and r.last_collect_nr = rr.last_collect_nr
                and r.product_id = rr.product_id
                and r.id = rr.id
                and r.optional = false

                union all

                -- TODO: fix last_collect_nr check for rh
                select im.last_collect_nr, rh.child_product_id, rh.child_req_id
                from IsMandatory im, RequirementHierarchies rh, Requirements r
                where im.product_id = rh.parent_product_id
                and im.id = rh.parent_req_id
                and im.last_collect_nr = r.last_collect_nr
                and rh.child_product_id = r.product_id and rh.child_req_id = r.id
                and r.optional = false
            ),
            IsOptional(last_collect_nr, product_id, id) as (
                select last_collect_nr, product_id, id
                from Requirements r
                where not exists (
                    select *
                    from IsMandatory im
                    where im.last_collect_nr = r.last_collect_nr
                    and im.product_id = r.product_id
                    and im.id = r.id
                )
            )
            select distinct $1 as last_collect_nr, product_id, id
            from IsOptional
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from OptionalRequirements
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

    async fn update_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into ManualRequirements (
                last_collect_nr,
                product_id,
                id
            )
            with MarkedManual(product_id, id) as (
                select product_id, id
                from Requirements
                where manual_verification = true
                and last_collect_nr = $1
                and product_id = $2
            ),
            ParentMarkedManual(product_id, id) as (
                select rd.descendant_product_id, rd.descendant_id
                from RequirementDescendants rd, MarkedManual md
                where rd.product_id = md.product_id and rd.id = md.id
            )
            select $1 as last_collect_nr, product_id, id
            from MarkedManual
            union all
            select $1 as last_collect_nr, product_id, id
            from ParentMarkedManual
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from ManualRequirements
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

    async fn update_usable_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into UsableRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select last_collect_nr, product_id, id
            from Requirements
            where last_collect_nr = $1 and product_id = $2
            except
            select last_collect_nr, product_id, id
            from
            (
                select last_collect_nr, product_id, id
                from DeprecatedRequirements
                where last_collect_nr = $1 and product_id = $2
                union all
                select last_collect_nr, product_id, id
                from ExcludedRequirements
                where last_collect_nr = $1 and product_id = $2
            )
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from UsableRequirements
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

    async fn update_usable_non_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into UsableNonManualRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select last_collect_nr, product_id, id
            from UsableRequirements
            where last_collect_nr = $1 and product_id = $2
            except
            select last_collect_nr, product_id, id
            from ManualRequirements
            where last_collect_nr = $1 and product_id = $2
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from UsableNonManualRequirements
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

    async fn update_usable_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let product_id = self.product_id();

        sqlx::query!(
            "
            insert or replace into UsableManualRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select ur.last_collect_nr, ur.product_id, ur.id
            from UsableRequirements ur, ManualRequirements mr
            where ur.last_collect_nr = $1 and ur.product_id = $2
            and mr.last_collect_nr = $1 and mr.product_id = $2
            and ur.id = mr.id
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        sqlx::query!(
            "
            delete from UsableManualRequirements
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
