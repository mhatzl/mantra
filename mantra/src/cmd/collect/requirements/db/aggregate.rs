use mantra_schema::annotations::TraceKind;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(crate) async fn aggregate_requirements_data(&mut self) -> Result<(), anyhow::Error> {
        // Note: order is important, because later queries build on updated tables
        self.update_requirement_descendants().await?;
        self.update_leaf_requirements().await?;
        self.update_deprecated_requirements().await?;
        self.update_excluded_requirements().await?;
        self.update_optional_requirements().await?;
        self.update_manual_requirements().await?;
        self.update_usable_requirements().await?;
        self.update_usable_manual_requirements().await?;
        self.update_usable_non_manual_requirements().await?;
        self.update_directly_satisfied_requirements().await?;

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
        .await?;

        Ok(())
    }

    async fn update_requirement_descendants(&mut self) -> Result<(), anyhow::Error> {
        sqlx::query!(
            "
            with recursive TransitiveChildren(last_collect_nr, product_id, id, descendant_product_id, descendant_id) as
            (
                select
                    last_collect_nr,
                    parent_product_id, parent_req_id,
                    child_product_id, child_req_id
                from RequirementHierarchies
                union all
                select rh.last_collect_nr, tc.product_id, tc.id, rh.child_product_id, rh.child_req_id
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
            "
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
        .await?;

        if !req_cycle_exists.is_empty() {
            for bad in req_cycle_exists {
                eprintln!(
                    "Requirement cycle detected for req '{}' in product id='{}'",
                    bad.req_id, bad.product_id
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
            and (product_id = $2 or descendant_product_id = $2)
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await?;

        Ok(())
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
        .await?;

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
        .await?;

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
            with MarkedExclude(product_id, id) as (
                select product_id, id
                from Requirements
                where exclude = true
                and last_collect_nr = $1
                and product_id = $2
            ),
            ParentMarkedExclude(product_id, id) as (
                select rd.descendant_product_id, rd.descendant_id
                from RequirementDescendants rd, MarkedExclude md
                where rd.product_id = md.product_id and rd.id = md.id
            )
            select $1 as last_collect_nr, product_id, id
            from MarkedExclude
            union all
            select $1 as last_collect_nr, product_id, id
            from ParentMarkedExclude
            ",
            collect_nr,
            product_id
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
        .await?;

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
            with MarkedOptional(product_id, id) as (
                select product_id, id
                from Requirements
                where optional = true
                and last_collect_nr = $1
                and product_id = $2
            ),
            ParentMarkedOptional(product_id, id) as (
                select rd.descendant_product_id, rd.descendant_id
                from RequirementDescendants rd, MarkedOptional md
                where rd.product_id = md.product_id and rd.id = md.id
            )
            select $1 as last_collect_nr, product_id, id
            from MarkedOptional
            union all
            select $1 as last_collect_nr, product_id, id
            from ParentMarkedOptional
            ",
            collect_nr,
            product_id
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
        .await?;

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
        .await?;

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
        .await?;

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
        .await?;

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
        .await?;

        Ok(())
    }
}
