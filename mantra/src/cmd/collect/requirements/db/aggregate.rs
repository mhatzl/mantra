use anyhow::{Context, anyhow};

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_requirements_data(&mut self) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();

        let latest_requirements_collection = sqlx::query!(
            r#"
            select max(latest_collection) as "latest_collection:i64"
            from Requirements
            "#
        )
        .fetch_optional(self.connection_mut())
        .await
        .context("Querying latest collection updating requirements")?
        .and_then(|r| r.latest_collection);

        // if there is no requirement data, we still must ensure that aggragation tables get cleared
        if latest_requirements_collection.is_some()
            && latest_requirements_collection != Some(collect_nr)
        {
            log::info!("No requirement data collected, skipping requirement aggregation");

            return Ok(());
        }

        // Note: order is important, because later queries build on updated tables
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

        self.check_replacing_requirements()
            .await
            .context("Failed checking requirement replacements")?;

        self.update_deprecated_requirements()
            .await
            .context("Failed to update deprecated requirements")?;
        self.update_excluded_requirements()
            .await
            .context("Failed to update excluded requirements")?;
        self.update_draft_requirements()
            .await
            .context("Failed to update draft requirements")?;
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

        Ok(())
    }

    async fn update_requirement_hierarchies(&mut self) -> Result<(), anyhow::Error> {
        let missing_parents = sqlx::query!(
            "
            select rp.product_id, rp.req_id, rp.parent_product_id, rp.parent_req_id
            from RequirementParents rp
            where not exists (
                select r.id
                from Requirements r
                where rp.parent_product_id = r.product_id
                and rp.parent_req_id = r.id
            )
            "
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Querying for missing requirement parents")?;

        // TODO: do not fail on first error
        if let Some(missing_parent) = missing_parents.first() {
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
            where not exists (
                select r.id
                from Requirements r
                where rc.child_product_id = r.product_id
                and rc.child_req_id = r.id
            )
            "
        )
        .fetch_all(self.connection_mut())
        .await
        .context("Querying for missing requirement children")?;

        // TODO: do not fail on first error
        if let Some(missing_child) = missing_children.first() {
            anyhow::bail!(
                "Missing requirement child product-id='{}' req-id='{}' for parent product-id='{}' req-id='{}'",
                missing_child.child_product_id,
                missing_child.child_req_id,
                missing_child.product_id,
                missing_child.req_id
            )
        }

        sqlx::query!(
            "
            delete from RequirementHierarchies
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementHierarchies")?;

        sqlx::query!(
            "
            insert into RequirementHierarchies (
                child_product_id,
                child_req_id,
                parent_product_id,
                parent_req_id,
                optional
            )
            select
                product_id as child_product_id,
                req_id as child_req_id,
                parent_product_id,
                parent_req_id,
                (optional is not null and optional = true) as optional
            from RequirementParents

            union

            select
                child_product_id,
                child_req_id,
                product_id as parent_product_id,
                req_id as parent_req_id,
                (optional is not null and optional = true) as optional
            from RequirementChildren
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_root_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from RootRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RootRequirements")?;

        // We have to consider all requirements, also those not collected in this collection,
        // so we check that we only consider requirements that were last collected for their product.
        sqlx::query!(
            "
            insert into RootRequirements (
                product_id,
                req_id
            )
            select r.product_id, r.id
            from Requirements r
            where not exists (
                select *
                from RequirementHierarchies rh
                where rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
            )
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_requirement_descendants(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from RequirementDescendants
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing RequirementDescendants")?;

        // We have to consider all requirements, also those not collected in this collection,
        // so we check that we only consider requirements that were last collected for their product.
        //
        // Optional relations are propagated to descendants.
        sqlx::query!(
            "
            with recursive TransitiveChildren(
                product_id,
                req_id,
                descendant_product_id,
                descendant_req_id,
                optional
            ) as
            (
                select
                    parent_product_id as product_id,
                    parent_req_id as req_id,
                    child_product_id as descendant_product_id,
                    child_req_id as descendant_req_id,
                    optional
                from RequirementHierarchies rh

                union all

                select
                    tc.product_id,
                    tc.req_id,
                    rh.child_product_id as descendant_product_id,
                    rh.child_req_id as descendant_req_id,
                    rh.optional or tc.optional
                from RequirementHierarchies rh, TransitiveChildren tc
                where tc.descendant_product_id = rh.parent_product_id
                and tc.descendant_req_id = rh.parent_req_id
                -- prevents endless recursion in case of requirement cycles
                -- match on parent to have the cycle entry in the descendants,
                -- which is then detected in a separate query.
                and (
                    tc.product_id != rh.parent_product_id
                    or tc.req_id != rh.parent_req_id
                )
            )
            insert into RequirementDescendants (
                product_id,
                req_id,
                descendant_product_id,
                descendant_req_id,
                optional
            )
            select distinct
                product_id,
                req_id,
                descendant_product_id,
                descendant_req_id,
                optional
            from TransitiveChildren
            "
        )
        .execute(self.connection_mut())
        .await?;

        let req_cycle_exists = sqlx::query!(
            "
            select
                rd.product_id,
                rd.req_id
            from RequirementDescendants rd
            where rd.product_id = rd.descendant_product_id
            and rd.req_id = rd.descendant_req_id
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

        Ok(())
    }

    async fn check_replacing_requirements(&mut self) -> Result<(), anyhow::Error> {
        let bad_replacements = sqlx::query!(
            "
            select rr.product_id, rr.req_id, rr.replaced_req_id
            from RequirementReplacements rr, RequirementDescendants rd
            where rr.product_id = rd.product_id
            and rr.product_id = rd.descendant_product_id
            and rr.replaced_req_id = rd.req_id
            and rr.req_id = rd.descendant_req_id
            "
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
        sqlx::query!(
            "
            delete from LeafRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing LeafRequirements")?;

        sqlx::query!(
            "
            insert into LeafRequirements (
                product_id,
                req_id
            )
            select product_id, id
            from Requirements r
            where not exists (
                select *
                from RequirementHierarchies rh
                where r.product_id = rh.parent_product_id
                and r.id = rh.parent_req_id
            )
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_deprecated_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from DeprecatedRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing DeprecatedRequirements")?;

        sqlx::query!(
            "
            insert into DeprecatedRequirements (
                product_id,
                req_id
            )
            with recursive IsActive(product_id, id) as (
                select product_id, id
                from (
                    select r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly deprecated
                    and (r.deprecated is null or r.deprecated = false)
                    and not exists (
                        select rp.req_id
                        from RequirementReplacements rp
                        where rp.product_id = r.product_id
                        and rp.replaced_req_id = r.id
                    )

                    union all

                    -- include reqs explicitly set to 'deprecated = false' in case related root is deprecated
                    select r.product_id, r.id
                    from Requirements r
                    where r.deprecated is not null and r.deprecated = false
                    -- being replaced always deprecates a requirement
                    and not exists (
                        select rp.req_id
                        from RequirementReplacements rp
                        where rp.product_id = r.product_id
                        and rp.replaced_req_id = r.id
                    )
                )

                union all

                select rh.child_product_id, rh.child_req_id
                from IsActive ia, RequirementHierarchies rh, Requirements r
                where ia.product_id = rh.parent_product_id
                and ia.id = rh.parent_req_id
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.deprecated is null or r.deprecated = false)
                and not exists (
                    select rp.req_id
                    from RequirementReplacements rp
                    where rp.product_id = r.product_id
                    and rp.replaced_req_id = r.id
                )
            ),
            IsDeprecated(product_id, id) as (
                select product_id, id
                from Requirements r
                where not exists (
                    select *
                    from IsActive ia
                    where ia.product_id = r.product_id
                    and ia.id = r.id
                )
            )
            select distinct product_id, id
            from IsDeprecated
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_excluded_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from ExcludedRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing ExcludedRequirements")?;

        sqlx::query!(
            "
            insert into ExcludedRequirements (
                product_id,
                req_id
            )
            with recursive IsIncluded(product_id, id) as (
                select product_id, id
                from (
                    select r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly excluded
                    and (r.exclude is null or r.exclude = false)

                    union all

                    -- include reqs explicitly set to 'exclude = false' in case related root is excluded
                    select r.product_id, r.id
                    from Requirements r
                    where r.exclude is not null and r.exclude = false
                )

                union all

                select rh.child_product_id, rh.child_req_id
                from IsIncluded nm, RequirementHierarchies rh, Requirements r
                where nm.product_id = rh.parent_product_id
                and nm.id = rh.parent_req_id
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.exclude is null or r.exclude = false)
            ),
            IsExcluded(product_id, id) as (
                select product_id, id
                from Requirements r
                where not exists (
                    select *
                    from IsIncluded nm
                    where nm.product_id = r.product_id
                    and nm.id = r.id
                )
            )
            select distinct product_id, id
            from IsExcluded
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_draft_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from DraftRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing DraftRequirements")?;

        sqlx::query!(
            "
            insert into DraftRequirements (
                product_id,
                req_id
            )
            with recursive IsAccepted(product_id, id) as (
                select product_id, id
                from (
                    select r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly draft
                    and (r.draft is null or r.draft = false)

                    union all

                    -- include reqs explicitly set to 'draft = false' in case related root is draft
                    select r.product_id, r.id
                    from Requirements r
                    where r.draft is not null and r.draft = false
                )

                union all

                select rh.child_product_id, rh.child_req_id
                from IsAccepted ia, RequirementHierarchies rh, Requirements r
                where ia.product_id = rh.parent_product_id
                and ia.id = rh.parent_req_id
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.draft is null or r.draft = false)
            ),
            IsDraft(product_id, id) as (
                select product_id, id
                from Requirements r
                where not exists (
                    select *
                    from IsAccepted ia
                    where ia.product_id = r.product_id
                    and ia.id = r.id
                )
            )
            select distinct product_id, id
            from IsDraft
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_optional_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from OptionalRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing OptionalRequirements")?;

        sqlx::query!(
            "
            insert into OptionalRequirements (
                product_id,
                req_id
            )
            with recursive IsMandatory(product_id, id) as (
                select product_id, id
                from (
                    select r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly optional
                    and (r.optional is null or r.optional = false)

                    union all

                    -- include reqs explicitly set to 'optional = false' in case related root is optional
                    select r.product_id, r.id
                    from Requirements r
                    where r.optional is not null and r.optional = false
                )

                union all

                select rh.child_product_id, rh.child_req_id
                from IsMandatory nm, RequirementHierarchies rh, Requirements r
                where nm.product_id = rh.parent_product_id
                and nm.id = rh.parent_req_id
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.optional is null or r.optional = false)
                and rh.optional = false
            ),
            IsOptional(product_id, id) as (
                select product_id, id
                from Requirements r
                where not exists (
                    select *
                    from IsMandatory nm
                    where nm.product_id = r.product_id
                    and nm.id = r.id
                )
            )
            select distinct product_id, id
            from IsOptional
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from ManualRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing ManualRequirements")?;

        sqlx::query!(
            "
            insert into ManualRequirements (
                product_id,
                req_id
            )
            with recursive NonManual(product_id, id) as (
                select product_id, id
                from (
                    select r.product_id, r.id
                    from Requirements r, RootRequirements rr
                    where r.product_id = rr.product_id
                    and r.id = rr.req_id
                    -- means not explicitly requires manual verification
                    and (r.manual_verification is null or r.manual_verification = false)

                    union all

                    -- include reqs explicitly set to 'manual_verification = false' in case related root requires manual verification
                    select r.product_id, r.id
                    from Requirements r
                    where r.manual_verification is not null and r.manual_verification = false
                )

                union all

                select rh.child_product_id, rh.child_req_id
                from NonManual nm, RequirementHierarchies rh, Requirements r
                where nm.product_id = rh.parent_product_id
                and nm.id = rh.parent_req_id
                and rh.child_product_id = r.product_id
                and rh.child_req_id = r.id
                and (r.manual_verification is null or r.manual_verification = false)
            ),
            IsManual(product_id, id) as (
                select product_id, id
                from Requirements r
                where not exists (
                    select *
                    from NonManual nm
                    where nm.product_id = r.product_id
                    and nm.id = r.id
                )
            )
            select distinct product_id, id
            from IsManual
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from UsableRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing UsableRequirements")?;

        sqlx::query!(
            "
            insert into UsableRequirements (
                product_id,
                req_id
            )
            select product_id, id
            from Requirements r

            except

            select product_id, req_id
            from
            (
                select product_id, req_id
                from DeprecatedRequirements

                union all

                select product_id, req_id
                from (
                    select product_id, req_id
                    from ExcludedRequirements

                    union all

                    select product_id, req_id
                    from DraftRequirements
                )
            )
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_non_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from UsableNonManualRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing UsableNonManualRequirements")?;

        sqlx::query!(
            "
            insert into UsableNonManualRequirements (
                product_id,
                req_id
            )
            select product_id, req_id
            from UsableRequirements

            except

            select product_id, req_id
            from ManualRequirements
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }

    async fn update_usable_manual_requirements(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            delete from UsableManualRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Clearing UsableManualRequirements")?;

        sqlx::query!(
            "
            insert into UsableManualRequirements (
                product_id,
                req_id
            )
            select ur.product_id, ur.req_id
            from UsableRequirements ur, ManualRequirements mr
            where ur.product_id = mr.product_id
            and ur.req_id = mr.req_id
            "
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
    }
}
