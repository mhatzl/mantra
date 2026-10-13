use mantra_schema::report::requirement::RequirementState;

use crate::cmd::collect::Collection;

impl<'db> Collection<'db> {
    pub(super) async fn update_indirect_req_verification_states(
        &mut self,
    ) -> Result<(), anyhow::Error> {
        let collect_nr = self.collect_nr();
        let req_verified_nr = RequirementState::Verified.as_nr();
        let req_failed_nr = RequirementState::Failed.as_nr();
        let req_skipped_nr = RequirementState::Skipped.as_nr();
        let req_satisfied_nr = RequirementState::Satisfied.as_nr();
        let req_untraced_nr = RequirementState::Untraced.as_nr();

        sqlx::query!(
            "
            insert or replace into UsableLeafRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                ur.last_collect_nr,
                ur.product_id,
                ur.id
            from UsableRequirements ur, LeafRequirements lr
            where ur.last_collect_nr = $1 and ur.product_id = $2
            and lr.last_collect_nr = $1 and lr.product_id = $2
            and ur.id = lr.id
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update UsableLeafRequirements")?;

        sqlx::query!(
            "
            insert or replace into UsableNonLeafRequirements (
                last_collect_nr,
                product_id,
                id
            )
            select
                ur.last_collect_nr,
                ur.product_id,
                ur.id
            from UsableRequirements ur, RequirementDescendants rd
            where ur.last_collect_nr = $1 and ur.product_id = $2
            and rd.last_collect_nr = $1 and rd.product_id = $2
            and ur.id = rd.id
            ",
            collect_nr,
            product_id
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update UsableNonLeafRequirements")?;

        sqlx::query!(
            "
            insert or replace into RequirementsWithOnlyOptionalChildren (
                last_collect_nr,
                product_id,
                id
            )
            -- Note: Includes leaf requirements needed for indirect check later
            select $1, r.product_id, r.id
            from UsableRequirements r
            where not exists (
                select rh.child_req_id
                from
                    RequirementHierarchies rh,
                    UsableRequirements ur
                where r.id = rh.parent_req_id
                and r.product_id = rh.parent_product_id
                and rh.child_req_id = ur.id
                and rh.child_product_id = ur.product_id
                and not exists (
                    select op.id
                    from OptionalRequirements op
                    where op.id = rh.child_req_id
                    and op.product_id = rh.child_product_id
                )
            )
            ",
            collect_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithOnlyOptionalChildren")?;

        // Note: The table may contain multiple state entries per requirement
        // This will be considered when updating IndirectRequirementVerificationStates below
        sqlx::query!(
            "
            insert or replace into StatesOfRequirementsWithOnlyOptionalChildren (
                last_collect_nr,
                product_id,
                id,
                state
            )
            -- may contain duplicate differing state entries of requirements
            with recursive StatesOfReqWithOnlyOptionalChildren (id, product_id, state) as (
                select lr.id, lr.product_id, ds.state
                from
                    UsableLeafRequirements lr,
                    DirectRequirementVerificationStates ds
                where lr.id = ds.id and lr.product_id = ds.product_id

                union all

                select
                    r.id,
                    r.product_id,
                    case
                        when ds.state = $3 or sor.state = $3 then $3
                        when ds.state = $2 or (ds.state = $4 and sor.state = $2) then $2
                        else ds.state
                    end
                from
                    RequirementsWithOnlyOptionalChildren r,
                    DirectRequirementVerificationStates ds,
                    RequirementHierarchies rh,
                    StatesOfReqWithOnlyOptionalChildren sor
                where r.id = rh.parent_req_id and r.product_id = rh.parent_product_id
                and r.id = ds.id and r.product_id = ds.product_id
                and sor.id = rh.child_req_id and sor.product_id = rh.child_product_id
            )
            select $1, product_id, id, state
            from StatesOfReqWithOnlyOptionalChildren
            ",
            collect_nr,
            req_verified_nr,
            req_failed_nr,
            req_unverified_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update StatesOfRequirementsWithOnlyOptionalChildren")?;

        sqlx::query!(
            "
            insert or replace into VerifiedRequirementsWithOnlyOptionalChildren (
                last_collect_nr,
                product_id,
                id
            )
            with VerifiedReqsWithOnlyOptionalChildren (id, product_id) as (
                -- Note: we ignore failed states, because it is handled by ReqsWithFailedDescendants
                -- This leaves indirectly verified by at least one child,
                -- or direct requirement state entries.
                -- This is sufficient for requirements with only optional children
                select distinct sr.id, sr.product_id
                from StatesOfRequirementsWithOnlyOptionalChildren sr
                where exists (
                    select isr.id
                    from StatesOfRequirementsWithOnlyOptionalChildren isr
                    where sr.id = isr.id
                    and sr.product_id = isr.product_id
                    and isr.state = $2
                )
            )
            select $1, product_id, id
            from VerifiedReqsWithOnlyOptionalChildren
            ",
            collect_nr,
            req_verified_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update VerifiedRequirementsWithOnlyOptionalChildren")?;

        sqlx::query!(
            "
            insert or replace into RequirementsWithUnverifiedNonOptionalChildren (
                last_collect_nr,
                product_id,
                id
            )
            with recursive ReqsWithUnverifiedNonOptionalChildren (id, product_id) as (
                select r.id, r.product_id
                from UsableNonLeafRequirements r
                where exists (
                    select rh.child_req_id
                    from
                        RequirementHierarchies rh,
                        DirectRequirementVerificationStates ds,
                        UsableLeafRequirements lr
                    where r.id = rh.parent_req_id
                    and r.product_id = rh.parent_product_id
                    and rh.child_req_id = lr.id
                    and rh.child_product_id = lr.product_id
                    and lr.id = ds.id and lr.product_id = ds.product_id
                    and ds.state = $2
                    and not exists (
                        select op.id
                        from OptionalRequirements op
                        where op.id = rh.child_req_id
                        and op.product_id = rh.child_product_id
                    )
                ) or exists (
                    select ooc.id
                    from
                        DirectRequirementVerificationStates ds,
                        RequirementsWithOnlyOptionalChildren ooc
                    where r.product_id = ds.product_id
                    and r.last_collect_nr = ds.last_collect_nr
                    and r.id = ds.id
                    and r.product_id = ooc.product_id
                    and r.last_collect_nr = ooc.last_collect_nr
                    and r.id = ooc.id
                    and ds.state = $2
                    and not exists (
                        select vr.id
                        from VerifiedRequirementsWithOnlyOptionalChildren vr
                        where r.product_id = vr.product_id
                        and r.last_collect_nr = vr.last_collect_nr
                        and r.id = vr.id
                    )
                ) or exists (
                    -- manual requirements must be reviewed directly independent of the state of children
					select mr.product_id, mr.id
                    from ManualRequirements mr, DirectRequirementVerificationStates ds
                    where mr.product_id = r.product_id and mr.id = r.id
                    and ds.product_id = mr.product_id and ds.id = mr.id
                    and ds.state = $2
				)

                union all

                select r.id, r.product_id
                from
                    UsableNonLeafRequirements r,
                    ReqsWithUnverifiedNonOptionalChildren uc,
                    RequirementHierarchies rh
                where r.id = rh.parent_req_id and r.product_id = rh.parent_product_id
                and uc.id = rh.child_req_id and uc.product_id = rh.child_product_id
                and not exists (
                    select op.id
                    from OptionalRequirements op
                    where op.id = rh.child_req_id
                    and op.product_id = rh.child_product_id
                )
            )
            select $1, r.product_id, r.id
            from ReqsWithUnverifiedNonOptionalChildren r
            ",
            collect_nr,
            req_unverified_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithUnverifiedNonOptionalChildren")?;

        sqlx::query!(
            "
            insert or replace into RequirementsWithSkippedNonOptionalChildren (
                last_collect_nr,
                product_id,
                id
            )
            with recursive ReqsWithSkippedNonOptionalChildren (id, product_id) as (
                select r.id, r.product_id
                from UsableNonLeafRequirements r
                where exists (
                    select rh.child_req_id
                    from
                        RequirementHierarchies rh,
                        DirectRequirementVerificationStates ds,
                        UsableLeafRequirements lr
                    where r.id = rh.parent_req_id
                    and r.product_id = rh.parent_product_id
                    and rh.child_req_id = lr.id
                    and rh.child_product_id = lr.product_id
                    and lr.id = ds.id and lr.product_id = ds.product_id
                    and ds.state = $2
                    and not exists (
                        select op.id
                        from OptionalRequirements op
                        where op.id = rh.child_req_id
                        and op.product_id = rh.child_product_id
                    )
                ) or exists (
                    select ooc.id
                    from
                        DirectRequirementVerificationStates ds,
                        RequirementsWithOnlyOptionalChildren ooc
                    where r.product_id = ds.product_id
                    and r.last_collect_nr = ds.last_collect_nr
                    and r.id = ds.id
                    and r.product_id = ooc.product_id
                    and r.last_collect_nr = ooc.last_collect_nr
                    and r.id = ooc.id
                    and ds.state = $2
                )

                union all

                select r.id, r.product_id
                from
                    UsableNonLeafRequirements r,
                    ReqsWithSkippedNonOptionalChildren sc,
                    RequirementHierarchies rh
                where r.id = rh.parent_req_id and r.product_id = rh.parent_product_id
                and sc.id = rh.child_req_id and sc.product_id = rh.child_product_id
                and not exists (
                    select op.id
                    from OptionalRequirements op
                    where op.id = rh.child_req_id
                    and op.product_id = rh.child_product_id
                )
            )
            select $1, r.product_id, r.id
            from ReqsWithSkippedNonOptionalChildren r
            ",
            collect_nr,
            req_skipped_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithSkippedNonOptionalChildren")?;

        sqlx::query!(
            "
            insert or replace into IndirectRequirementVerificationStates (
                last_collect_nr,
                product_id,
                id,
                state
            )
            with ReqsWithFailedDescendants (id) as (
                select r.id
                from UsableNonLeafRequirements r
                where exists (
                    select rd.id
                    from RequirementDescendants rd, DirectRequirementVerificationStates s
                    where rd.last_collect_nr = $1 and rd.product_id = $2
                    and rd.id = r.id and rd.descendant_id = s.id
                    and rd.descendant_product_id = s.product_id
                    and s.state = $4
                )
            ), ReqsWithVerifiedNonOptionalDescendants (id) as (
                select r.id
                from UsableNonLeafRequirements r
                where exists (
                    select rd.id
                    from RequirementDescendants rd, DirectRequirementVerificationStates s
                        left join OptionalRequirements opt on
                            s.last_collect_nr = opt.last_collect_nr
                            and s.product_id = opt.product_id
                            and s.id = opt.id
                    where rd.last_collect_nr = $1 and rd.product_id = $2
                    and rd.id = r.id and rd.descendant_id = s.id
                    and rd.descendant_product_id = s.product_id
                    and opt.id is null
                    and s.state = $3
                )
            )
            select
                r.last_collect_nr,
                r.product_id,
                r.id,
                case
                    when exists (
                        select f.id
                        from ReqsWithFailedDescendants f
                        where r.id = f.id
                    ) then $4
                    when exists (
                        select u.id
                        from RequirementsWithUnverifiedNonOptionalChildren u
                        where r.id = u.id
                    ) then $6
                    when exists (
                        select s.id
                        from RequirementsWithSkippedNonOptionalChildren s
                        where r.id = s.id
                    ) then $5
                    when exists (
                        select v.id
                        from ReqsWithVerifiedNonOptionalDescendants v
                        where r.id = v.id
                    ) then $3
                    when exists (
                        select os.id
                        from VerifiedRequirementsWithOnlyOptionalChildren os
                        where r.id = os.id and r.product_id = os.product_id
                        -- filter requirements that are only directly verified
                        -- this requires that leaf requirements are part of VerifiedRequirementsWithOnlyOptionalChildren
                        and exists (
                            select ocs.id
                            from
                                VerifiedRequirementsWithOnlyOptionalChildren ocs,
                                RequirementHierarchies rh
                            where r.id = rh.parent_req_id
                            and r.product_id = rh.parent_product_id
                            and ocs.id = rh.child_req_id
                            and ocs.product_id = rh.child_product_id
                        )
                    ) then $3
                    else $6
                end
            from UsableNonLeafRequirements r
            ",
            collect_nr,
            product_id,
            req_verified_nr,
            req_failed_nr,
            req_skipped_nr,
            req_unverified_nr
        )
        .execute(self.connection_mut())
        .await.context("Failed to update IndirectRequirementVerificationStates")?;

        Ok(())
    }
}
