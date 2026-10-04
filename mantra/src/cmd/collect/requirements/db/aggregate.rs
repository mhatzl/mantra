use anyhow::{Context, anyhow};
use mantra_schema::annotations::TraceKind;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_requirements_data(&mut self) -> Result<(), anyhow::Error> {
        // Note: order is nmportant, because later queries build on updated tables
        self.update_requirement_hierarchies()
            .await
            .context("Updating requirement hierarchies")?;
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

    async fn update_requirement_hierarchies(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert into RequirementHierarchies (
                child_collect_nr,
                child_product_id,
                child_req_id,
                parent_collect_nr,
                parent_product_id,
                parent_req_id,
                optional
            )
            select
                $1 as child_collect_nr,
                rp.product_id as child_product_id,
                rp.req_id as child_req_id,
                r.collect_nr as parent_collect_nr,
                rp.parent_product_id,
                rp.parent_req_id,
                rp.optional
            from Requirements r, RequirementParents rp
            where rp.collect_nr = $1 and r.product_id = rp.parent_product_id
            and r.id = rp.parent_req_id and r.collect_nr = (
                select max(p.collect_nr)
                from Products p
                where p.id = r.product_id
            )

            union

            select
                r.collect_nr as child_collect_nr,
                rc.child_product_id,
                rc.child_req_id,
                $1 as parent_collect_nr,
                rc.product_id as parent_product_id,
                rc.req_id as parent_req_id,
                rc.optional
            from Requirements r, RequirementChildren rc
            where rc.collect_nr = $1 and r.product_id = rc.child_product_id
            and r.id = rc.child_req_id and r.collect_nr = (
                select max(p.collect_nr)
                from Products p
                where p.id = r.product_id
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        let missing_parents = sqlx::query!(
            "
            select rp.product_id, rp.req_id, rp.parent_product_id, rp.parent_req_id
            from RequirementParents rp
            where rp.collect_nr = $1 and not exists (
                select rh.parent_req_id
                from RequirementHierarchies rh
                where rh.child_collect_nr = $1
                and rp.product_id = rh.child_product_id
                and rp.req_id = rh.child_req_id
            )
            ",
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Querying for missing requirement parents")?;

        // TODO: do not fail on first error
        for missing_parent in missing_parents {
            anyhow::bail!(
                "Missing requirement parent product-id='{}' req-id='{}' for child product-id='{}' req-id='{}'",
                missing_parent.parent_product_id,
                missing_parent.parent_req_id,
                missing_parent.product_id,
                missing_parent.req_id
            )
        }

        let missing_children = sqlx::query!(
            "
            select rc.product_id, rc.req_id, rc.child_product_id, rc.child_req_id
            from RequirementChildren rc
            where rc.collect_nr = $1 and not exists (
                select rh.child_req_id
                from RequirementHierarchies rh
                where rh.parent_collect_nr = $1
                and rc.product_id = rh.parent_product_id
                and rc.req_id = rh.parent_req_id
            )
            ",
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Querying for missing requirement children")?;

        // TODO: do not fail on first error
        for missing_child in missing_children {
            anyhow::bail!(
                "Missing requirement child product-id='{}' req-id='{}' for parent product-id='{}' req-id='{}'",
                missing_child.child_product_id,
                missing_child.child_req_id,
                missing_child.product_id,
                missing_child.req_id
            )
        }

        Ok(())
    }

    async fn update_root_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        // We have to consider all requirements, also those not collected in this collection,
        // so we check that we only consider requirements that were last collected for their product.
        sqlx::query!(
            "
            insert or ignore into RootRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select $1 as agg_collect_nr, r.collect_nr, r.product_id, r.id
            from Requirements r
            where r.collect_nr = (
                select max(cr.collect_nr)
                from Requirements cr
                where cr.product_id = r.product_id
            )
            and not exists (
                select *
                from RequirementHierarchies rh
                where rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and rh.child_collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = rh.child_product_id
                )
                and rh.parent_collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = rh.parent_product_id
                )
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_requirement_descendants(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        // We have to consider all requirements, also those not collected in this collection,
        // so we check that we only consider requirements that were last collected for their product.
        //
        // Optional relations are propagated to descendants.
        sqlx::query!(
            "
            with recursive TransitiveChildren(
                collect_nr,
                product_id,
                req_id,
                descendant_collect_nr,
                descendant_product_id,
                descendant_req_id,
                optional
            ) as
            (
                select
                    parent_collect_nr as collect_nr,
                    parent_product_id as product_id,
                    parent_req_id as req_id,
                    child_collect_nr as descendant_collect_nr,
                    child_product_id as descendant_product_id,
                    child_req_id as descendant_req_id,
                    optional
                from RequirementHierarchies rh
                where rh.child_collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = rh.child_product_id
                )
                and rh.parent_collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = rh.parent_product_id
                )

                union all

                select
                    tc.collect_nr,
                    tc.product_id,
                    tc.req_id,
                    rh.child_collect_nr as descendant_collect_nr,
                    rh.child_product_id as descendant_product_id,
                    rh.child_req_id as descendant_req_id,
                    rh.optional or tc.optional
                from RequirementHierarchies rh, TransitiveChildren tc
                where tc.descendant_collect_nr = rh.parent_collect_nr
                and tc.descendant_product_id = rh.parent_product_id
                and tc.descendant_req_id = rh.parent_req_id
                and rh.child_collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = rh.child_product_id
                )
                -- prevents endless recursion in case of requirement cycles
                -- match on parent to have the cycle entry in the descendants,
                -- which is then detected in a separate query.
                and (
                    tc.collect_nr != rh.parent_collect_nr
                    or tc.product_id != rh.parent_product_id
                    or tc.req_id != rh.parent_req_id
                )
            )
            insert into RequirementDescendants (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id,
                descendant_collect_nr,
                descendant_product_id,
                descendant_req_id,
                optional
            )
            select
                $1 as agg_collect_nr,
                collect_nr,
                product_id,
                req_id,
                descendant_collect_nr,
                descendant_product_id,
                descendant_req_id,
                optional
            from TransitiveChildren
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        let req_cycle_exists = sqlx::query!(
            "
            select
                rd.product_id,
                rd.req_id
            from RequirementDescendants rd
            where
                rd.agg_collect_nr = $1
                and rd.req_collect_nr = rd.descendant_collect_nr
                and rd.product_id = rd.descendant_product_id
                and rd.req_id = rd.descendant_req_id
            ",
            collect_nr
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

        Ok(())
    }

    async fn check_replacing_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        let bad_replacements = sqlx::query!(
            "
            select rr.product_id, rr.req_id, rr.replaced_req_id
            from RequirementReplacements rr, RequirementDescendants rd
            where rd.agg_collect_nr = $1
                and rr.collect_nr = rd.req_collect_nr
                and rr.collect_nr = rd.descendant_collect_nr
                and rr.product_id = rd.product_id
                and rr.product_id = rd.descendant_product_id
                and rr.replaced_req_id = rd.req_id
                and rr.req_id = rd.descendant_req_id
            ",
            collect_nr
        )
        .fetch_all(self.connection_mut())
        .await?;

        for bad_record in &bad_replacements {
            log::error!(
                "Requirement '{}' in product '{}' cannot replace its ancestor '{}'",
                bad_record.req_id,
                bad_record.product_id,
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

        sqlx::query!(
            "
            insert or ignore into LeafRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select $1 as agg_collect_nr, collect_nr, product_id, id
            from Requirements r
            where r.collect_nr = (
                select max(p.collect_nr)
                from Products p
                where p.id = r.product_id
            )
            and not exists (
                select *
                from RequirementHierarchies rh
                where r.collect_nr = rh.parent_collect_nr
                and r.product_id = rh.parent_product_id
                and r.id = rh.parent_req_id
                and rh.child_collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = rh.child_product_id
                )
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_deprecated_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into DeprecatedRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with recursive IsIncluded(collect_nr, product_id, id) as (
                select r.collect_nr, r.product_id, r.id
                from (
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where rr.agg_collect_nr = $1
                    -- no need to check that requirement is from latest product collection,
                    -- because otherwise it would not be listed as latest aggregated root requirement
                    and r.collect_nr = rr.req_collect_nr
                    and r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly deprecated
                    and (r.deprecated is null or r.deprecated = false)

                    union all

                    -- include reqs explicitly set to 'deprecated = false' in case related root is deprecated
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r
                    where r.collect_nr = (
                        select max(p.collect_nr)
                        from Products p
                        where p.id = r.product_id
                    ) and r.deprecated is not null and r.deprecated = false
                )

                union all

                select rh.child_collect_nr, rh.child_product_id, rh.child_req_id
                from IsIncluded nm, RequirementHierarchies rh, Requirements r
                where nm.collect_nr = rh.parent_collect_nr
                and nm.product_id = rh.parent_product_id
                and nm.id = rh.parent_req_id
                and r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and rh.child_collect_nr = r.collect_nr
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.deprecated is null or r.deprecated = false)
            ),
            IsDeprecated(collect_nr, product_id, id) as (
                select collect_nr, product_id, id
                from Requirements r
                where r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and not exists (
                    select *
                    from IsIncluded nm
                    where nm.collect_nr = r.collect_nr
                    and nm.product_id = r.product_id
                    and nm.id = r.id
                )
            )
            select distinct $1 as agg_collect_nr, collect_nr, product_id, id
            from IsDeprecated
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_excluded_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into ExcludedRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with recursive IsIncluded(collect_nr, product_id, id) as (
                select r.collect_nr, r.product_id, r.id
                from (
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where rr.agg_collect_nr = $1
                    -- no need to check that requirement is from latest product collection,
                    -- because otherwise it would not be listed as latest aggregated root requirement
                    and r.collect_nr = rr.req_collect_nr
                    and r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly excluded
                    and (r.exclude is null or r.exclude = false)

                    union all

                    -- include reqs explicitly set to 'exclude = false' in case related root is excluded
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r
                    where r.collect_nr = (
                        select max(p.collect_nr)
                        from Products p
                        where p.id = r.product_id
                    ) and r.exclude is not null and r.exclude = false
                )

                union all

                select rh.child_collect_nr, rh.child_product_id, rh.child_req_id
                from IsIncluded nm, RequirementHierarchies rh, Requirements r
                where nm.collect_nr = rh.parent_collect_nr
                and nm.product_id = rh.parent_product_id
                and nm.id = rh.parent_req_id
                and r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and rh.child_collect_nr = r.collect_nr
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.exclude is null or r.exclude = false)
            ),
            IsExcluded(collect_nr, product_id, id) as (
                select collect_nr, product_id, id
                from Requirements r
                where r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and not exists (
                    select *
                    from IsIncluded nm
                    where nm.collect_nr = r.collect_nr
                    and nm.product_id = r.product_id
                    and nm.id = r.id
                )
            )
            select distinct $1 as agg_collect_nr, collect_nr, product_id, id
            from IsExcluded
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_optional_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into OptionalRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with recursive IsMandatory(collect_nr, product_id, id) as (
                select r.collect_nr, r.product_id, r.id
                from (
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where rr.agg_collect_nr = $1
                    -- no need to check that requirement is from latest product collection,
                    -- because otherwise it would not be listed as latest aggregated root requirement
                    and r.collect_nr = rr.req_collect_nr
                    and r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly optional
                    and (r.optional is null or r.optional = false)

                    union all

                    -- include reqs explicitly set to 'optional = false' in case related root is optional
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r
                    where r.collect_nr = (
                        select max(p.collect_nr)
                        from Products p
                        where p.id = r.product_id
                    ) and r.optional is not null and r.optional = false
                )

                union all

                select rh.child_collect_nr, rh.child_product_id, rh.child_req_id
                from IsMandatory nm, RequirementHierarchies rh, Requirements r
                where nm.collect_nr = rh.parent_collect_nr
                and nm.product_id = rh.parent_product_id
                and nm.id = rh.parent_req_id
                and r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and rh.child_collect_nr = r.collect_nr
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.optional is null or r.optional = false)
            ),
            IsOptional(collect_nr, product_id, id) as (
                select collect_nr, product_id, id
                from Requirements r
                where r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and not exists (
                    select *
                    from IsMandatory nm
                    where nm.collect_nr = r.collect_nr
                    and nm.product_id = r.product_id
                    and nm.id = r.id
                )
            )
            select distinct $1 as agg_collect_nr, collect_nr, product_id, id
            from IsOptional
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into ManualRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with recursive NonManual(collect_nr, product_id, id) as (
                select r.collect_nr, r.product_id, r.id
                from (
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where rr.agg_collect_nr = $1
                    -- no need to check that requirement is from latest product collection,
                    -- because otherwise it would not be listed as latest aggregated root requirement
                    and r.collect_nr = rr.req_collect_nr
                    and r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly requires manual verification
                    and (r.manual_verification is null or r.manual_verification = false)

                    union all

                    -- include reqs explicitly set to 'manual_verification = false' in case related root requires manual verification
                    select r.collect_nr, r.product_id, r.id
                    from Requirements r
                    where r.collect_nr = (
                        select max(p.collect_nr)
                        from Products p
                        where p.id = r.product_id
                    ) and r.manual_verification is not null and r.manual_verification = false
                )

                union all

                select rh.child_collect_nr, rh.child_product_id, rh.child_req_id
                from NonManual nm, RequirementHierarchies rh, Requirements r
                where nm.collect_nr = rh.parent_collect_nr
                and nm.product_id = rh.parent_product_id
                and nm.id = rh.parent_req_id
                and r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and rh.child_collect_nr = r.collect_nr
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.manual_verification is null or r.manual_verification = false)
            ),
            IsManual(collect_nr, product_id, id) as (
                select collect_nr, product_id, id
                from Requirements r
                where r.collect_nr = (
                    select max(p.collect_nr)
                    from Products p
                    where p.id = r.product_id
                )
                and not exists (
                    select *
                    from NonManual nm
                    where nm.collect_nr = r.collect_nr
                    and nm.product_id = r.product_id
                    and nm.id = r.id
                )
            )
            select distinct $1 as agg_collect_nr, collect_nr, product_id, id
            from IsManual
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into UsableRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select $1 as agg_collect_nr, collect_nr, product_id, id
            from Requirements r
            where r.collect_nr = (
                select max(p.collect_nr)
                from Products p
                where p.id = r.product_id
            )

            except

            select $1 as agg_collect_nr, req_collect_nr, product_id, req_id
            from
            (
                select req_collect_nr, product_id, req_id
                from DeprecatedRequirements
                where agg_collect_nr = $1

                union all

                select req_collect_nr, product_id, req_id
                from ExcludedRequirements
                where agg_collect_nr = $1
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_non_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into UsableNonManualRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select agg_collect_nr, req_collect_nr, product_id, req_id
            from UsableRequirements
            where agg_collect_nr = $1

            except

            select agg_collect_nr, req_collect_nr, product_id, req_id
            from ManualRequirements
            where agg_collect_nr = $1
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        sqlx::query!(
            "
            insert or ignore into UsableManualRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            select $1 as agg_collect_nr, ur.req_collect_nr, ur.product_id, ur.req_id
            from UsableRequirements ur, ManualRequirements mr
            where ur.agg_collect_nr = $1 and mr.agg_collect_nr = $1
            and ur.req_collect_nr = mr.req_collect_nr
            and ur.product_id = mr.product_id
            and ur.req_id = mr.req_id
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_directly_satisfied_requirements(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let satisfies_kind = TraceKind::Satisfies.as_nr();

        sqlx::query!(
            "
            insert or ignore into DirectlySatisfiedRequirements (
                agg_collect_nr,
                req_collect_nr,
                product_id,
                req_id
            )
            with NonManualSatisfyTraced (req_collect_nr, product_id, req_id) as (
                select ur.req_collect_nr, ur.product_id, ur.req_id
                from UsableNonManualRequirements ur, DirectProductReqTraces dt, Traces t
                where ur.agg_collect_nr = $1 and dt.collect_nr = (
                    select max(collect_nr)
                    from Products p
                    where p.id = dt.product_id
                )
                and ur.req_collect_nr = dt.collect_nr
                and ur.product_id = dt.product_id
                and ur.req_id = dt.req_id and dt.file_hash = t.file_hash
                and dt.line = t.line
                and t.kind = $2
            ),
            ManualReviewed (req_collect_nr, product_id, req_id) as (
                select mr.req_collect_nr, mr.product_id, mr.req_id
                from UsableManualRequirements mr, ManuallyVerifiedRequirements vr
                where mr.agg_collect_nr = $1 and vr.collect_nr = (
                    select max(collect_nr)
                    from Products p
                    where p.id = vr.product_id
                ) and mr.req_collect_nr = vr.collect_nr
                and mr.product_id = vr.product_id
                and mr.req_id = vr.req_id
            )

            select $1 as agg_collect_nr, req_collect_nr, product_id, req_id
            from NonManualSatisfyTraced

            union

            select $1 as agg_collect_nr, req_collect_nr, product_id, req_id
            from ManualReviewed
            ",
            collect_nr,
            satisfies_kind
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
