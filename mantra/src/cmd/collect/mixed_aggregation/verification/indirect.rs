use anyhow::Context;
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
        let req_unknown_nr = RequirementState::Unknown.as_nr();
        let req_partially_verified_nr = RequirementState::PartiallyVerified.as_nr();

        sqlx::query!(
            "
            insert or ignore into UsableLeafRequirements (
                product_id,
                req_id
            )
            select distinct
                ur.product_id,
                ur.req_id
            from UsableRequirements ur, LeafRequirements lr
            where ur.product_id = lr.product_id
            and ur.req_id = lr.req_id
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update UsableLeafRequirements")?;

        sqlx::query!(
            "
            insert or ignore into UsableNonLeafRequirements (
                product_id,
                req_id
            )
            select distinct
                product_id,
                req_id
            from UsableRequirements

            except

            select distinct
                product_id,
                req_id
            from UsableLeafRequirements
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update UsableNonLeafRequirements")?;

        sqlx::query!(
            "
            -- Note: Includes leaf requirements needed for indirect check later
            insert or ignore into RequirementsWithOnlyOptionalChildren (
                product_id,
                req_id
            )
            with RequirementsWithNonOptionalChild (
                product_id,
                req_id
            ) as (
                select
                    ur.product_id,
                    ur.req_id
                from UsableNonLeafRequirements ur
                where exists (
                    select rh.child_req_id
                    from
                        RequirementHierarchies rh,
                        UsableRequirements uri
                    where ur.product_id = rh.parent_product_id
                    and ur.req_id = rh.parent_req_id
                    and uri.product_id = rh.child_product_id
                    and uri.req_id = rh.child_req_id
                    and rh.optional = false
                    and not exists (
                        select op.req_id
                        from OptionalRequirements op
                        where op.product_id = rh.child_product_id
                        and op.req_id = rh.child_req_id
                    )
                )
            )
            select
                product_id,
                req_id
            from UsableRequirements ur

            except

            select
                product_id,
                req_id
            from RequirementsWithNonOptionalChild
            "
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithOnlyOptionalChildren")?;

        // Note: The table may contain multiple state entries per requirement
        // This will be considered when updating IndirectRequirementVerificationStates below
        sqlx::query!(
            "
            insert or ignore into StatesOfRequirementsWithOnlyOptionalChildren (
                product_id,
                req_id,
                child_product_id,
                child_req_id,
                state
            )
            with recursive StatesOfReqWithOnlyOptionalChildren (
                product_id,
                req_id,
                child_product_id,
                child_req_id,
                state
            ) as (
                -- Leaf requirements are their own child.
                -- Also needed to have leafs added here to resolve indirect requirement states in the end.
                select
                    lr.product_id,
                    lr.req_id,
                    lr.product_id as child_product_id,
                    lr.req_id as child_req_id,
                    ds.state
                from
                    UsableLeafRequirements lr,
                    DirectRequirementVerificationStates ds
                where lr.product_id = ds.product_id
                and lr.req_id = ds.req_id

                union all

                select
                    r.product_id,
                    r.req_id,
                    rh.child_product_id,
                    rh.child_req_id,
                    case
                        -- failed propagates even for optional reqs
                        when ds.state = $2 or sor.state = $2 then $2
                        when ds.state = $1 or (
                            -- unknown or satisfied
                            (ds.state = $3 or ds.state = $4)
                            and sor.state = $1
                            -- manual requirements must be directly verified
                            and not exists (
                                select m.req_id
                                from ManualRequirements m
                                where m.product_id = r.product_id
                                and m.req_id = r.req_id
                            )
                        ) then $1
                        else ds.state
                    end
                from
                    RequirementsWithOnlyOptionalChildren r,
                    DirectRequirementVerificationStates ds,
                    RequirementHierarchies rh,
                    StatesOfReqWithOnlyOptionalChildren sor
                where r.product_id = ds.product_id
                and r.req_id = ds.req_id
                and r.product_id = rh.parent_product_id
                and r.req_id = rh.parent_req_id
                and sor.product_id = rh.child_product_id
                and sor.req_id = rh.child_req_id
            )
            select distinct
                product_id,
                req_id,
                child_product_id,
                child_req_id,
                state
            from StatesOfReqWithOnlyOptionalChildren
            ",
            req_verified_nr,
            req_failed_nr,
            req_satisfied_nr,
            req_unknown_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update StatesOfRequirementsWithOnlyOptionalChildren")?;

        sqlx::query!(
            "
            insert or ignore into VerifiedRequirementsWithOnlyOptionalChildren (
                product_id,
                req_id
            )
            -- If requirement itself and no child failed, having at least one verifying state
            -- is sufficient for requirements with only optional children,
            -- because either it is verified directly or by at least one child
            select sr.product_id, sr.req_id
            from StatesOfRequirementsWithOnlyOptionalChildren sr
            where exists (
                select isr.req_id
                from StatesOfRequirementsWithOnlyOptionalChildren isr
                where isr.product_id = sr.product_id
                and isr.req_id = sr.req_id
                and isr.state = $1
            )

            except

            select product_id, req_id
            from FailedRequirements
            ",
            req_verified_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update VerifiedRequirementsWithOnlyOptionalChildren")?;

        sqlx::query!(
            "
            insert or ignore into RequirementsWithNonOptionalChildrenHavingSkippedState (
                product_id,
                req_id
            )
            with recursive ReqsWithNonOptionalChildrenHavingSkippedState (
                product_id,
                req_id,
                child_product_id,
                child_req_id
            ) as (
                select
                    product_id,
                    req_id,
                    child_product_id,
                    child_req_id
                from (
                    select
                        r.product_id,
                        r.req_id,
                        rh.child_product_id,
                        rh.child_req_id
                    from UsableNonLeafRequirements r, RequirementHierarchies rh
                    where r.product_id = rh.parent_product_id
                    and r.req_id = rh.parent_req_id
                    and exists (
                        select rh.child_req_id
                        from
                            DirectRequirementVerificationStates ds,
                            UsableLeafRequirements lr
                        where rh.child_product_id = lr.product_id
                        and rh.child_req_id = lr.req_id
                        and lr.product_id = ds.product_id
                        and lr.req_id = ds.req_id
                        and ds.state = $1
                        and rh.optional = false
                        and not exists (
                            select op.req_id
                            from OptionalRequirements op
                            where op.product_id = rh.child_product_id
                            and op.req_id = rh.child_req_id
                        )
                    )

                    union all

                    -- skipped req that only has optional children
                    -- may serve as skipped child for its parents
                    select
                        ds.product_id,
                        ds.req_id,
                        ds.product_id,
                        ds.req_id
                    from
                        DirectRequirementVerificationStates ds,
                        RequirementsWithOnlyOptionalChildren ooc
                    where ooc.product_id = ds.product_id
                    and ooc.req_id = ds.req_id
                    and ds.state = $1
                )

                union all

                select
                    r.product_id,
                    r.req_id,
                    rh.child_product_id,
                    rh.child_req_id
                from
                    UsableNonLeafRequirements r,
                    ReqsWithNonOptionalChildrenHavingSkippedState sc,
                    RequirementHierarchies rh
                where r.product_id = rh.parent_product_id
                and r.req_id = rh.parent_req_id
                and sc.product_id = rh.child_product_id
                and sc.req_id = rh.child_req_id
                and rh.optional = false
                and not exists (
                    select op.req_id
                    from OptionalRequirements op
                    where op.product_id = rh.child_product_id
                    and op.req_id = rh.child_req_id
                )
            )
            select distinct product_id, req_id
            from ReqsWithNonOptionalChildrenHavingSkippedState
            -- filter base requirements since their state did not depend on children
            where product_id != child_product_id
            or req_id != child_req_id
            ",
            req_skipped_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithNonOptionalChildrenHavingSkippedState")?;

        sqlx::query!(
            "
            insert or ignore into RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState (
                product_id,
                req_id
            )
            with recursive ReqsWithNonOptionalChildrenHavingStatisfiedOrUnknownState (
                product_id,
                req_id,
                child_product_id,
                child_req_id
            ) as (
                select
                    product_id,
                    req_id,
                    child_product_id,
                    child_req_id
                from (
                    -- at least one non-optional leaf either has satisfied or unknown
                    select
                        r.product_id,
                        r.req_id,
                        rh.child_product_id,
                        rh.child_req_id
                    from UsableNonLeafRequirements r, RequirementHierarchies rh
                    where r.product_id = rh.parent_product_id
                    and r.req_id = rh.parent_req_id
                    and exists (
                        select lr.req_id
                        from
                            DirectRequirementVerificationStates ds,
                            UsableLeafRequirements lr
                        where rh.child_product_id = lr.product_id
                        and rh.child_req_id = lr.req_id
                        and lr.product_id = ds.product_id
                        and lr.req_id = ds.req_id
                        and (ds.state = $1 or ds.state = $2)
                        and rh.optional = false
                        and not exists (
                            select op.req_id
                            from OptionalRequirements op
                            where op.product_id = rh.child_product_id
                            and op.req_id = rh.child_req_id
                        )
                    )

                    union all

                    select
                        r.product_id,
                        r.req_id,
                        r.product_id as child_product_id,
                        r.req_id as child_req_id
                    from UsableRequirements r
                    -- reqs with only optional children added as base in case they are
                    -- only of satisfied or unknown state, because they are likely children
                    -- of requirements that may have non-optional children as well.
                    where exists (
                        select ooc.req_id
                        from
                            DirectRequirementVerificationStates ds,
                            RequirementsWithOnlyOptionalChildren ooc
                        where ds.product_id = r.product_id
                        and ds.req_id = r.req_id
                        and ooc.product_id = r.product_id
                        and ooc.req_id = r.req_id
                        and (ds.state = $1 or ds.state = $2)
                        and not exists (
                            select vr.req_id
                            from VerifiedRequirementsWithOnlyOptionalChildren vr
                            where r.product_id = vr.product_id
                            and r.req_id = vr.req_id
                        )
                    )
                    -- manual requirements must be reviewed directly independent of the state of children
                    or exists (
    					select mr.req_id
                        from ManualRequirements mr, DirectRequirementVerificationStates ds
                        where mr.product_id = r.product_id
                        and mr.req_id = r.req_id
                        and ds.product_id = r.product_id
                        and ds.req_id = r.req_id
                        and (ds.state = $1 or ds.state = $2)
    				)
				)

                union all

                select
                    r.product_id,
                    r.req_id,
                    rh.child_product_id,
                    rh.child_req_id
                from
                    UsableNonLeafRequirements r,
                    ReqsWithNonOptionalChildrenHavingStatisfiedOrUnknownState uc,
                    RequirementHierarchies rh
                where r.product_id = rh.parent_product_id
                and r.req_id = rh.parent_req_id
                and uc.product_id = rh.child_product_id
                and uc.req_id = rh.child_req_id
                and rh.optional = false
                and not exists (
                    select op.req_id
                    from OptionalRequirements op
                    where op.product_id = rh.child_product_id
                    and op.req_id = rh.child_req_id
                )
            )
            select distinct product_id, req_id
            from ReqsWithNonOptionalChildrenHavingStatisfiedOrUnknownState
            -- filter base requirements since their state did not depend on children
            where product_id != child_product_id
            or req_id != child_req_id
            ",
            req_satisfied_nr,
            req_unknown_nr
        )
        .execute(self.connection_mut())
        .await
        .context("Failed to update RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState")?;

        sqlx::query!(
            "
            insert or ignore into IndirectRequirementVerificationStates (
                product_id,
                req_id,
                state
            )
            with ReqsWithVerifiedDescendants (
                product_id,
                req_id
            ) as (
                select r.product_id, r.req_id
                from
                    UsableNonLeafRequirements r,
                    RequirementDescendants rd,
                    DirectRequirementVerificationStates ds
                where rd.product_id = r.product_id
                and rd.req_id = r.req_id
                and rd.descendant_product_id = ds.product_id
                and rd.descendant_req_id = ds.req_id
                and ds.state = $1
            ), ReqsWithFullyOrPartiallyVerifiedDescendants (
                product_id,
                req_id
            ) as (
                select r.product_id, r.req_id
                from
                    UsableNonLeafRequirements r,
                    RequirementDescendants rd,
                    DirectRequirementVerificationStates ds
                where rd.product_id = r.product_id
                and rd.req_id = r.req_id
                and rd.descendant_product_id = ds.product_id
                and rd.descendant_req_id = ds.req_id
                and (ds.state = $1 or ds.state = $5)
            )
            select
                r.product_id,
                r.req_id,
                case
                    -- failed
                    when exists (
                        select f.req_id
                        from FailedRequirements f
                        where f.product_id = r.product_id
                        and f.req_id = r.req_id
                    ) then $2
                    -- skipped
                    when exists (
                        select s.req_id
                        from RequirementsWithNonOptionalChildrenHavingSkippedState s
                        where s.product_id = r.product_id
                        and s.req_id = r.req_id
                    ) then $3
                    -- verified requirements with only-optional non-failed children
                    when exists (
                        select os.req_id
                        from VerifiedRequirementsWithOnlyOptionalChildren os
                        where os.product_id = r.product_id
                        and os.req_id = r.req_id
                        -- filter requirements that are only directly verified
                        -- this requires that leaf requirements are part of VerifiedRequirementsWithOnlyOptionalChildren
                        and exists (
                            select ocs.req_id
                            from
                                VerifiedRequirementsWithOnlyOptionalChildren ocs,
                                RequirementHierarchies rh
                            where r.product_id = rh.parent_product_id
                            and r.req_id = rh.parent_req_id
                            and ocs.product_id = rh.child_product_id
                            and ocs.req_id = rh.child_req_id
                        )
                    ) then $1
                    -- partially verified or unknown, because satisfied can only be achieved directly
                    when exists (
                        select u.req_id
                        from RequirementsWithNonOptionalChildrenHavingStatisfiedOrUnknownState u
                        where u.product_id = r.product_id
                        and u.req_id = r.req_id
                    ) then
                        case
                            -- partially verified beats unknown, because it provides more information
                            -- even if req itself is manual, partially verified state is valid
                            when exists (
                                select pvd.req_id
                                from ReqsWithFullyOrPartiallyVerifiedDescendants pvd
                                where pvd.product_id = r.product_id
                                and pvd.req_id = r.req_id
                            ) then $5
                            else $4
                        end
                    -- failed, skipped, partially verified, satisfied, or unknown handled above.
                    -- If at least one descendant is verified and the requirement was not matched so far,
                    -- verification must have propagated up.
                    -- => verified
                    when exists (
                        select v.req_id
                        from ReqsWithVerifiedDescendants v
                        where v.product_id = r.product_id
                        and v.req_id = r.req_id
                    ) then $1
                    -- unknown
                    -- e.g. only-optional children and none verified
                    else $4
                end
            from UsableNonLeafRequirements r
            ",
            req_verified_nr,
            req_failed_nr,
            req_skipped_nr,
            req_unknown_nr,
            req_partially_verified_nr
        )
        .execute(self.connection_mut())
        .await.context("Failed to update IndirectRequirementVerificationStates")?;

        Ok(())
    }
}
